use borsh::BorshDeserialize;
use clap::{Parser, Subcommand};
use merkle_tree_program::{instruction, state::MerkleStateAccount, utils::find_merkle_state_pda};
use sha2::{Digest, Sha256};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::read_keypair_file,
    signer::Signer,
    system_program,
    transaction::Transaction,
};
use solana_transaction_status::option_serializer::OptionSerializer;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Solana RPC URL.
    #[arg(short, long, default_value = "http://127.0.0.1:8899")]
    url: String,

    /// Merkle tree program id.
    #[arg(
        short,
        long,
        default_value = "FuWr9Bgn4aWiXLzDoV69Amp3pLwThpjwXJVAE7GTT7bV"
    )]
    program_id: Pubkey,

    /// Keypair path.
    #[arg(short, long)]
    keypair_path: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Send `InsertLeaf` transaction instruction.
    InsertLeaf { value: u32 },
    /// Fetch root hash from merkle state pda.
    GetRootHash,
    /// Compute sha256 hash for `value`.
    GetValueHash { value: u32 },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let payer = read_keypair_file(&args.keypair_path).expect("Invalid keypair file/path");

    let client = RpcClient::new(args.url);
    let (merkle_state_pda, _) = find_merkle_state_pda(&args.program_id);

    match args.command {
        Commands::InsertLeaf { value } => {
            let hash: [u8; 32] = Sha256::digest(value.to_le_bytes()).into();

            let insert_leaf_ix = Instruction::new_with_bytes(
                args.program_id,
                &instruction::MerkleTreeInstruction::InsertLeaf { hash }.pack(),
                vec![
                    AccountMeta::new(merkle_state_pda, false),
                    AccountMeta::new(payer.pubkey(), true),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            );

            let mut tx = Transaction::new_with_payer(&[insert_leaf_ix], Some(&payer.pubkey()));
            let recent_blockhash = client
                .get_latest_blockhash()
                .await
                .expect("Can't get latest blockhash");
            tx.sign(&[&payer], recent_blockhash);

            let tx_sig = client
                .send_and_confirm_transaction(&tx)
                .await
                .expect("Can't send tx");
            println!("Signature: {}", tx_sig);

            let tx_with_meta = client
                .get_transaction_with_config(
                    &tx_sig,
                    RpcTransactionConfig {
                        encoding: None,
                        commitment: None,
                        max_supported_transaction_version: None,
                    },
                )
                .await
                .expect("Can't get tx by sig");
            let tx_meta = tx_with_meta.transaction.meta.expect("Tx meta is empty");
            let OptionSerializer::Some(tx_logs) = tx_meta.log_messages else {
                panic!("Tx logs are empty");
            };

            let root_hash_log = tx_logs
                .iter()
                .find(|&tx_log| tx_log.contains("Program log: ["))
                .expect("Tx program log is not found");
            println!("Root hash log: {root_hash_log}");
        }
        Commands::GetRootHash => {
            let merkle_state_account = client
                .get_account(&merkle_state_pda)
                .await
                .expect("Can't get merkle state account or it's empty(not initialized)");

            let merkle_state = MerkleStateAccount::try_from_slice(&merkle_state_account.data)
                .expect("Invalid account data");

            println!("Root hash: {:x?}", merkle_state.get_root_hash());
        }
        Commands::GetValueHash { value } => {
            let hash: [u8; 32] = Sha256::digest(value.to_le_bytes()).into();
            println!("Value hash: {:x?}", hash);
        }
    }
}
