use crate::utils::hash_sorted_pair;
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct MerkleStateAccount {
    root_hash: [u8; 32],
    leaf_hashes: Vec<[u8; 32]>,
}

impl MerkleStateAccount {
    /// Merkle state account length(in bytes).
    /// 32(root_hash) + 4(vec) + Self::LEAF_LEN * n(total leaf nodes).
    pub const INIT_LEN: usize = 32 + 4 + Self::LEAF_LEN;

    /// Leaf node size in bytes.
    pub const LEAF_LEN: usize = 32;

    pub fn new(init_hash: &[u8; 32]) -> Self {
        Self {
            root_hash: *init_hash,
            leaf_hashes: vec![*init_hash],
        }
    }

    pub fn add_leaf(&mut self, leaf_hash: &[u8; 32]) {
        self.leaf_hashes.push(*leaf_hash);
        self.update_root_hash();
    }

    fn update_root_hash(&mut self) {
        let mut current_layer = self.leaf_hashes.to_vec();

        while current_layer.len() > 1 {
            let mut next_layer = Vec::new();

            for pair in current_layer.chunks(2) {
                let combined = match pair {
                    [a, b] => hash_sorted_pair(a, b),
                    [a] => hash_sorted_pair(a, a),
                    _ => unreachable!(),
                };
                next_layer.push(combined);
            }

            current_layer = next_layer;
        }

        self.root_hash = current_layer[0];
    }

    pub fn get_root_hash(&self) -> [u8; 32] {
        self.root_hash
    }

    pub fn get_leaf_hashes(&self) -> Vec<[u8; 32]> {
        self.leaf_hashes.clone()
    }
}
