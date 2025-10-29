use stratum_core::bitcoin::hashes::{Hash, HashEngine, sha256d::Hash as Sha256dHash};

/// A Merkle tree implementation for easily extracting the merkle path from a block's txdata.
#[derive(Clone, Debug)]
pub enum MerkleTree {
    Leaf(Sha256dHash),
    Node {
        left: Box<MerkleTree>,
        right: Box<MerkleTree>,
        hash: Sha256dHash,
    },
}

impl MerkleTree {
    /// Build a merkle tree from an ordered list of transaction hashes
    pub fn build(hashes: Vec<Sha256dHash>) -> Self {
        if hashes.len() == 1 {
            return MerkleTree::Leaf(hashes[0]);
        }

        let mut current_level: Vec<MerkleTree> = hashes.into_iter().map(MerkleTree::Leaf).collect();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..current_level.len()).step_by(2) {
                let left = current_level[i].clone();
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1].clone()
                } else {
                    left.clone() // Duplicate if odd number of nodes
                };

                let left_hash = left.hash();
                let right_hash = right.hash();

                // Compute parent hash: SHA256(SHA256(left || right))
                let mut hasher = Sha256dHash::engine();
                HashEngine::input(&mut hasher, left_hash.as_byte_array());
                HashEngine::input(&mut hasher, right_hash.as_byte_array());
                let parent_hash = Sha256dHash::from_engine(hasher);

                next_level.push(MerkleTree::Node {
                    left: Box::new(left),
                    right: Box::new(right),
                    hash: parent_hash,
                });
            }

            current_level = next_level;
        }

        current_level
            .into_iter()
            .next()
            .expect("Tree should have at least one node")
    }

    /// Get the hash of this node/leaf
    pub fn hash(&self) -> Sha256dHash {
        match self {
            MerkleTree::Leaf(hash) => *hash,
            MerkleTree::Node { hash, .. } => *hash,
        }
    }

    /// Extract the merkle path from the coinbase transaction (index 0) to the root
    pub fn extract_coinbase_path(&self) -> Vec<Sha256dHash> {
        let mut path = Vec::new();
        self.extract_path_internal(0, 0, &mut path);
        path
    }

    /// Internal recursive helper to extract path for a specific index
    /// The path is built bottom-up (leaf to root) by recursing first, then pushing
    fn extract_path_internal(
        &self,
        target_index: usize,
        current_index: usize,
        path: &mut Vec<Sha256dHash>,
    ) {
        match self {
            MerkleTree::Leaf(_) => {
                // Reached a leaf, nothing to add to path
            }
            MerkleTree::Node { left, right, .. } => {
                let left_size = left.size();
                let right_index = current_index + left_size;

                if target_index < right_index {
                    // Target is in the left subtree, so right is the sibling
                    // Recurse first to build path bottom-up
                    left.extract_path_internal(target_index, current_index, path);
                    // Then push the sibling hash at this level
                    let sibling_hash = match right.as_ref() {
                        MerkleTree::Leaf(hash) => *hash,
                        MerkleTree::Node { hash, .. } => *hash,
                    };
                    path.push(sibling_hash);
                } else {
                    // Target is in the right subtree, so left is the sibling
                    // Recurse first to build path bottom-up
                    right.extract_path_internal(target_index, right_index, path);
                    // Then push the sibling hash at this level
                    let sibling_hash = match left.as_ref() {
                        MerkleTree::Leaf(hash) => *hash,
                        MerkleTree::Node { hash, .. } => *hash,
                    };
                    path.push(sibling_hash);
                }
            }
        }
    }

    /// Get the number of leaves in this subtree
    fn size(&self) -> usize {
        match self {
            MerkleTree::Leaf(_) => 1,
            MerkleTree::Node { left, right, .. } => left.size() + right.size(),
        }
    }
}
