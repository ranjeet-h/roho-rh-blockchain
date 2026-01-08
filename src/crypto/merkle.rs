//! Merkle tree implementation
//! 
//! Used for computing transaction merkle roots in blocks.

use super::{Hash, hash_pair};

/// Compute the merkle root of a list of hashes
/// 
/// If the list is empty, returns zero hash.
/// If odd number of elements, duplicates the last element.
pub fn compute_merkle_root(hashes: &[Hash]) -> Hash {
    if hashes.is_empty() {
        return Hash::zero();
    }
    
    if hashes.len() == 1 {
        return hashes[0];
    }
    
    let mut current_level: Vec<Hash> = hashes.to_vec();
    
    while current_level.len() > 1 {
        // If odd number, duplicate last
        if current_level.len() % 2 == 1 {
            current_level.push(*current_level.last().unwrap());
        }
        
        let mut next_level = Vec::with_capacity(current_level.len() / 2);
        
        for chunk in current_level.chunks(2) {
            let combined = hash_pair(&chunk[0], &chunk[1]);
            next_level.push(combined);
        }
        
        current_level = next_level;
    }
    
    current_level[0]
}

/// Merkle proof for a transaction
#[derive(Debug, Clone)]
pub struct MerkleProof {
    /// Index of the transaction in the block
    pub index: usize,
    /// Sibling hashes from leaf to root
    pub siblings: Vec<(Hash, bool)>, // (hash, is_left)
}

impl MerkleProof {
    /// Verify this proof against a root hash
    pub fn verify(&self, tx_hash: &Hash, root: &Hash) -> bool {
        let mut current = *tx_hash;
        
        for (sibling, is_left) in &self.siblings {
            current = if *is_left {
                hash_pair(sibling, &current)
            } else {
                hash_pair(&current, sibling)
            };
        }
        
        current == *root
    }
}

/// Build a merkle proof for a transaction at given index
pub fn build_merkle_proof(hashes: &[Hash], index: usize) -> Option<MerkleProof> {
    if hashes.is_empty() || index >= hashes.len() {
        return None;
    }
    
    if hashes.len() == 1 {
        return Some(MerkleProof {
            index,
            siblings: vec![],
        });
    }
    
    let mut current_level: Vec<Hash> = hashes.to_vec();
    let mut current_index = index;
    let mut siblings = Vec::new();
    
    while current_level.len() > 1 {
        // If odd number, duplicate last
        if current_level.len() % 2 == 1 {
            current_level.push(*current_level.last().unwrap());
        }
        
        // Get sibling
        let sibling_index = if current_index % 2 == 0 {
            current_index + 1
        } else {
            current_index - 1
        };
        
        let is_left = current_index % 2 == 1;
        siblings.push((current_level[sibling_index], is_left));
        
        // Move to next level
        let mut next_level = Vec::with_capacity(current_level.len() / 2);
        for chunk in current_level.chunks(2) {
            next_level.push(hash_pair(&chunk[0], &chunk[1]));
        }
        
        current_level = next_level;
        current_index /= 2;
    }
    
    Some(MerkleProof { index, siblings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::hash_bytes;

    fn make_hashes(n: usize) -> Vec<Hash> {
        (0..n)
            .map(|i| hash_bytes(&i.to_le_bytes()))
            .collect()
    }

    #[test]
    fn test_empty_merkle_root() {
        let root = compute_merkle_root(&[]);
        assert_eq!(root, Hash::zero());
    }

    #[test]
    fn test_single_element() {
        let hashes = make_hashes(1);
        let root = compute_merkle_root(&hashes);
        assert_eq!(root, hashes[0]);
    }

    #[test]
    fn test_two_elements() {
        let hashes = make_hashes(2);
        let root = compute_merkle_root(&hashes);
        let expected = hash_pair(&hashes[0], &hashes[1]);
        assert_eq!(root, expected);
    }

    #[test]
    fn test_merkle_root_deterministic() {
        let hashes = make_hashes(10);
        let root1 = compute_merkle_root(&hashes);
        let root2 = compute_merkle_root(&hashes);
        assert_eq!(root1, root2);
    }

    #[test]
    fn test_merkle_proof_verification() {
        let hashes = make_hashes(8);
        let root = compute_merkle_root(&hashes);
        
        for i in 0..hashes.len() {
            let proof = build_merkle_proof(&hashes, i).unwrap();
            assert!(proof.verify(&hashes[i], &root));
        }
    }

    #[test]
    fn test_merkle_proof_wrong_hash_fails() {
        let hashes = make_hashes(8);
        let root = compute_merkle_root(&hashes);
        let proof = build_merkle_proof(&hashes, 0).unwrap();
        
        let wrong_hash = hash_bytes(b"wrong");
        assert!(!proof.verify(&wrong_hash, &root));
    }

    #[test]
    fn test_odd_number_of_elements() {
        let hashes = make_hashes(5);
        let root = compute_merkle_root(&hashes);
        
        // Should still work
        let proof = build_merkle_proof(&hashes, 4).unwrap();
        assert!(proof.verify(&hashes[4], &root));
    }
}
