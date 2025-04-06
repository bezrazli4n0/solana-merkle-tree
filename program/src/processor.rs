use crate::{state::MerkleStateAccount, utils::find_merkle_state_pda};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};

pub fn process_insert_leaf(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    hash: &[u8; 32],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let merkle_state_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    // 1. Verify passed system program
    if !system_program::check_id(system_program.key) {
        return Err(ProgramError::InvalidAccountData);
    }

    // 2. Verify passed merkle state PDA
    let (merkle_state_pda, merkle_state_bump) = find_merkle_state_pda(program_id);
    if &merkle_state_pda != merkle_state_account.key {
        return Err(ProgramError::InvalidAccountData);
    }

    // 3. Get or create merkle state account, append leaf node, recalc root hash..
    if merkle_state_account.data_is_empty() {
        let rent = Rent::get()?;
        let lamports = rent.minimum_balance(MerkleStateAccount::INIT_LEN);

        invoke_signed(
            &system_instruction::create_account(
                payer_account.key,
                &merkle_state_pda,
                lamports,
                MerkleStateAccount::INIT_LEN as u64,
                program_id,
            ),
            &[
                payer_account.clone(),
                merkle_state_account.clone(),
                system_program.clone(),
            ],
            &[&[b"merkle_state", &[merkle_state_bump]]],
        )?;

        let merkle_state = MerkleStateAccount::new(hash);
        merkle_state.serialize(&mut &mut merkle_state_account.data.borrow_mut()[..])?;

        msg!("{:x?}", merkle_state.get_root_hash());
        Ok(())
    } else {
        let rent = Rent::get()?;
        let mut merkle_state =
            MerkleStateAccount::try_from_slice(&merkle_state_account.data.borrow())?;

        // Calculate new size and updated rent-excempt balance
        let new_size = merkle_state_account.data.borrow().len() + MerkleStateAccount::LEAF_LEN;
        let lamports_diff = rent
            .minimum_balance(new_size)
            .checked_sub(merkle_state_account.lamports())
            .ok_or(ProgramError::AccountNotRentExempt)?;

        invoke(
            &system_instruction::transfer(payer_account.key, &merkle_state_pda, lamports_diff),
            &[
                payer_account.clone(),
                merkle_state_account.clone(),
                system_program.clone(),
            ],
        )?;

        merkle_state_account.realloc(new_size, false)?;

        merkle_state.add_leaf(hash);
        merkle_state.serialize(&mut &mut merkle_state_account.data.borrow_mut()[..])?;

        msg!("{:x?}", merkle_state.get_root_hash());
        Ok(())
    }
}
