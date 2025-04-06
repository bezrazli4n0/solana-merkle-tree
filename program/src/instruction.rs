use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub enum MerkleTreeInstruction {
    InsertLeaf { hash: [u8; 32] },
}

impl MerkleTreeInstruction {
    pub fn pack(&self) -> Vec<u8> {
        match self {
            Self::InsertLeaf { hash } => {
                let mut instruction_data = vec![0u8];
                instruction_data.extend_from_slice(hash);

                instruction_data
            }
        }
    }

    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (instruction_id, instruction_data) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        match instruction_id {
            0 => {
                let hash: [u8; 32] = instruction_data
                    .try_into()
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::InsertLeaf { hash })
            }
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
