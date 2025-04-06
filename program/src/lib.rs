#![allow(unexpected_cfgs)]

pub mod instruction;
mod processor;
pub mod state;
pub mod utils;

use instruction::MerkleTreeInstruction;
use processor::process_insert_leaf;
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = MerkleTreeInstruction::unpack(instruction_data)?;

    match instruction {
        MerkleTreeInstruction::InsertLeaf { hash } => {
            process_insert_leaf(program_id, accounts, &hash)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use borsh::BorshDeserialize;
    use sha2::{Digest, Sha256};
    use solana_program_test::{ProgramTest, processor};
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signer::Signer,
        system_program,
        transaction::Transaction,
    };
    use state::MerkleStateAccount;
    use utils::{find_merkle_state_pda, hash_sorted_pair};

    #[tokio::test]
    async fn success_init_merkle_state() {
        // Setup test env
        let program_id = Pubkey::new_unique();
        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "merkle_tree_program",
            program_id,
            processor!(process_instruction),
        )
        .start()
        .await;

        // Calculate merkle state pda
        let (merkle_state_pda, _) = find_merkle_state_pda(&program_id);

        // Prepare insert ix
        let mut hasher = Sha256::new();
        let data = 1337u32;
        hasher.update(data.to_le_bytes());
        let hash: [u8; 32] = hasher.finalize().try_into().expect("Invalid hash length");

        let insert_leaf_ix = Instruction::new_with_bytes(
            program_id,
            &instruction::MerkleTreeInstruction::InsertLeaf { hash: hash.clone() }.pack(),
            vec![
                AccountMeta::new(merkle_state_pda, false),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        // Check merkle state before tx
        let merkle_state = banks_client
            .get_account(merkle_state_pda)
            .await
            .expect("Can't get merkle state account");
        assert!(merkle_state.is_none());

        let mut tx = Transaction::new_with_payer(&[insert_leaf_ix], Some(&payer.pubkey()));
        tx.sign(&[&payer], recent_blockhash);
        let result = banks_client
            .process_transaction_with_metadata(tx)
            .await
            .expect("Can't process tx");

        // Verify that root hash event
        let Some(metadata) = result.metadata else {
            panic!("Tx metadata is empty");
        };
        assert!(
            metadata
                .log_messages
                .iter()
                .any(|log| log.contains(&format!("{:x?}", hash)))
        );

        // Check merkle state after tx
        let Some(merkle_state_account) = banks_client
            .get_account(merkle_state_pda)
            .await
            .expect("Can't get merkle state account")
        else {
            panic!("Merkle state account is uninitialized");
        };

        let merkle_state = MerkleStateAccount::try_from_slice(&merkle_state_account.data)
            .expect("Invalid merkle state account data");
        assert_eq!(merkle_state.get_root_hash(), hash);
        assert_eq!(merkle_state.get_leaf_hashes(), vec![hash]);
    }

    #[tokio::test]
    async fn success_insert_leaf() {
        // Setup test env
        let program_id = Pubkey::new_unique();
        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "merkle_tree_program",
            program_id,
            processor!(process_instruction),
        )
        .start()
        .await;

        // Calculate merkle state pda
        let (merkle_state_pda, _) = find_merkle_state_pda(&program_id);

        // Prepare insert ix
        let data_values = vec![1u32, 2, 3, 4, 5];
        let data_hashes: Vec<[u8; 32]> = data_values
            .iter()
            .map(|value| Sha256::digest(value.to_le_bytes()).into())
            .collect();

        // Submit all leaf hashes
        for hash in &data_hashes {
            let insert_leaf_ix = Instruction::new_with_bytes(
                program_id,
                &instruction::MerkleTreeInstruction::InsertLeaf { hash: *hash }.pack(),
                vec![
                    AccountMeta::new(merkle_state_pda, false),
                    AccountMeta::new(payer.pubkey(), true),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
            );

            let mut tx = Transaction::new_with_payer(&[insert_leaf_ix], Some(&payer.pubkey()));
            tx.sign(&[&payer], recent_blockhash);
            banks_client
                .process_transaction(tx)
                .await
                .expect("Can't process tx");
        }

        // Obtain `MerkleStateAccount` state
        let Some(merkle_state_account) = banks_client
            .get_account(merkle_state_pda)
            .await
            .expect("Can't get merkle state account")
        else {
            panic!("Merkle state account is uninitialized");
        };
        let merkle_state = MerkleStateAccount::try_from_slice(&merkle_state_account.data)
            .expect("Invalid merkle state data");
        assert_eq!(merkle_state.get_leaf_hashes().len(), data_hashes.len());

        // Verify root hash off-chain
        /*
         *       Root
         *        /\
         *      H3  H4
         *     /\    |
         *  H0   H1  H2(H2)
         *  /\   /\  |
         * 1 2  3 4  5(5)
         */
        // First layer
        let h0 = hash_sorted_pair(&data_hashes[0], &data_hashes[1]);
        let h1 = hash_sorted_pair(&data_hashes[2], &data_hashes[3]);
        let h2 = hash_sorted_pair(&data_hashes[4], &data_hashes[4]);

        // Second layer
        let h3 = hash_sorted_pair(&h0, &h1);
        let h4 = hash_sorted_pair(&h2, &h2);

        // Root
        let root_hash = hash_sorted_pair(&h3, &h4);
        assert_eq!(merkle_state.get_root_hash(), root_hash);
    }
}
