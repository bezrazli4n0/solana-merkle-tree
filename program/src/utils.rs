use sha2::{Digest, Sha256};
use solana_program::pubkey::Pubkey;

pub fn hash_sorted_pair(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    let (left, right) = if a <= b { (a, b) } else { (b, a) };

    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

pub fn find_merkle_state_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"merkle_state"], program_id)
}
