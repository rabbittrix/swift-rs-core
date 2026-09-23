use crate::hashutil::sha256;
use crate::types::Hash;

pub fn merkle_root(leaves: &[Hash]) -> Hash {
    if leaves.is_empty() {
        return Hash(sha256(b"SCBPS-EMPTY-MERKLE"));
    }
    let mut level: Vec<[u8; 32]> = leaves.iter().map(|leaf| leaf.0).collect();
    while level.len() > 1 {
        if level.len() % 2 == 1 {
            level.push(*level.last().expect("level is non-empty"));
        }
        let mut next = Vec::with_capacity(level.len() / 2);
        for pair in level.chunks(2) {
            let mut buf = [0u8; 64];
            buf[..32].copy_from_slice(&pair[0]);
            buf[32..].copy_from_slice(&pair[1]);
            next.push(sha256(&buf));
        }
        level = next;
    }
    Hash(level[0])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleProof {
    pub index: usize,
    pub siblings: Vec<Hash>,
}

pub fn merkle_proof(leaves: &[Hash], index: usize) -> Option<MerkleProof> {
    if index >= leaves.len() || leaves.is_empty() {
        return None;
    }
    let mut level: Vec<[u8; 32]> = leaves.iter().map(|leaf| leaf.0).collect();
    let mut idx = index;
    let mut siblings = Vec::new();
    while level.len() > 1 {
        if level.len() % 2 == 1 {
            level.push(*level.last().expect("level is non-empty"));
        }
        let sibling = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
        siblings.push(Hash(level[sibling]));
        let mut next = Vec::with_capacity(level.len() / 2);
        for pair in level.chunks(2) {
            let mut buf = [0u8; 64];
            buf[..32].copy_from_slice(&pair[0]);
            buf[32..].copy_from_slice(&pair[1]);
            next.push(sha256(&buf));
        }
        level = next;
        idx /= 2;
    }
    Some(MerkleProof { index, siblings })
}

pub fn verify_proof(leaf: &Hash, proof: &MerkleProof, root: &Hash) -> bool {
    let mut current = leaf.0;
    let mut idx = proof.index;
    for sibling in &proof.siblings {
        let mut buf = [0u8; 64];
        if idx % 2 == 0 {
            buf[..32].copy_from_slice(&current);
            buf[32..].copy_from_slice(&sibling.0);
        } else {
            buf[..32].copy_from_slice(&sibling.0);
            buf[32..].copy_from_slice(&current);
        }
        current = sha256(&buf);
        idx /= 2;
    }
    current == root.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_roundtrip_for_each_leaf() {
        let leaves: Vec<Hash> = (0..5u8)
            .map(|n| Hash::sha256(&[n]))
            .collect();
        let root = merkle_root(&leaves);
        for (index, leaf) in leaves.iter().enumerate() {
            let proof = merkle_proof(&leaves, index).unwrap();
            assert!(verify_proof(leaf, &proof, &root));
        }
    }

    #[test]
    fn empty_root_is_stable() {
        assert_eq!(merkle_root(&[]), merkle_root(&[]));
    }
}
