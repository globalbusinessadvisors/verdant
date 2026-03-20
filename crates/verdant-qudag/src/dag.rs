use verdant_core::config::MAX_DAG_TIPS;
use verdant_core::types::MessageHash;

use crate::bloom::BloomFilter;

/// Errors from DAG validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DagError {
    /// One or more parent hashes have not been seen.
    OrphanMessage,
    /// The message has no parents (only genesis may have none).
    NoParents,
    /// The message hash has already been inserted.
    DuplicateMessage,
}

/// A well-known genesis hash derived from a fixed seed.
const GENESIS_SEED: [u8; 32] = {
    let mut h = [0u8; 32];
    // "VERDANT_GENESIS_2026" hashed by hand-mixing
    h[0] = 0x56; // V
    h[1] = 0x45; // E
    h[2] = 0x52; // R
    h[3] = 0x44; // D
    h[4] = 0x41; // A
    h[5] = 0x4E; // N
    h[6] = 0x54; // T
    h[7] = 0x5F; // _
    h[8] = 0x47; // G
    h[9] = 0x45; // E
    h[10] = 0x4E; // N
    h[11] = 0x45; // E
    h[12] = 0x53; // S
    h[13] = 0x49; // I
    h[14] = 0x53; // S
    h[15] = 0x00;
    h
};

/// DAG state machine tracking message ordering and tip set.
///
/// Enforces causal ordering: every message must reference at least one
/// previously seen parent hash. The genesis hash is implicitly seen.
pub struct DagState {
    tips: heapless::Vec<MessageHash, MAX_DAG_TIPS>,
    seen: BloomFilter,
    sequence: u64,
}

impl DagState {
    /// Create a new DAG state with the genesis hash pre-inserted.
    pub fn new() -> Self {
        let mut state = Self {
            tips: heapless::Vec::new(),
            seen: BloomFilter::new(),
            sequence: 0,
        };
        let genesis = Self::genesis_hash();
        state.seen.insert(&genesis);
        let _ = state.tips.push(genesis);
        state
    }

    /// The well-known genesis hash that every node starts with.
    pub const fn genesis_hash() -> MessageHash {
        MessageHash(GENESIS_SEED)
    }

    /// Validate that all parent hashes have been seen.
    pub fn validate_parents(&self, parents: &[MessageHash]) -> Result<(), DagError> {
        if parents.is_empty() {
            return Err(DagError::NoParents);
        }
        for parent in parents {
            if !self.seen.contains(parent) {
                return Err(DagError::OrphanMessage);
            }
        }
        Ok(())
    }

    /// Insert a new message hash into the DAG after validating parents.
    ///
    /// The new hash becomes a tip, and any parents that are referenced
    /// are removed from the tip set (they are no longer leaves).
    pub fn insert(
        &mut self,
        hash: MessageHash,
        parents: &[MessageHash],
    ) -> Result<(), DagError> {
        self.validate_parents(parents)?;

        if self.seen.contains(&hash) {
            return Err(DagError::DuplicateMessage);
        }

        self.seen.insert(&hash);
        self.sequence += 1;

        // Remove consumed parents from tips
        self.tips.retain(|tip| !parents.contains(tip));

        // Add new hash as a tip; if full, evict the oldest
        if self.tips.is_full() {
            self.tips.remove(0);
        }
        let _ = self.tips.push(hash);

        Ok(())
    }

    /// Return up to `max` current DAG tips.
    pub fn current_tips(&self, max: usize) -> heapless::Vec<MessageHash, MAX_DAG_TIPS> {
        let take = max.min(self.tips.len());
        let mut out = heapless::Vec::new();
        for tip in self.tips.iter().rev().take(take) {
            let _ = out.push(*tip);
        }
        out
    }

    /// Check whether a hash has been seen (bloom filter — may have false positives).
    pub fn is_seen(&self, hash: &MessageHash) -> bool {
        self.seen.contains(hash)
    }

    /// Monotonically increasing insertion count.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
}

impl Default for DagState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash_from(seed: u8) -> MessageHash {
        let mut h = [0u8; 32];
        for (i, b) in h.iter_mut().enumerate() {
            *b = seed.wrapping_add(i as u8).wrapping_mul(0x9E);
        }
        MessageHash(h)
    }

    #[test]
    fn genesis_is_seen() {
        let dag = DagState::new();
        assert!(dag.is_seen(&DagState::genesis_hash()));
    }

    #[test]
    fn validate_parents_accepts_known_parents() {
        let dag = DagState::new();
        let result = dag.validate_parents(&[DagState::genesis_hash()]);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_parents_rejects_unknown_parents() {
        let dag = DagState::new();
        let result = dag.validate_parents(&[hash_from(99)]);
        assert_eq!(result, Err(DagError::OrphanMessage));
    }

    #[test]
    fn validate_parents_rejects_empty_parents() {
        let dag = DagState::new();
        let result = dag.validate_parents(&[]);
        assert_eq!(result, Err(DagError::NoParents));
    }

    #[test]
    fn insert_makes_hash_seen() {
        let mut dag = DagState::new();
        let h = hash_from(1);
        assert!(!dag.is_seen(&h));
        dag.insert(h, &[DagState::genesis_hash()]).unwrap();
        assert!(dag.is_seen(&h));
    }

    #[test]
    fn insert_updates_tips() {
        let mut dag = DagState::new();
        let genesis = DagState::genesis_hash();
        let h1 = hash_from(1);

        dag.insert(h1, &[genesis]).unwrap();

        let tips = dag.current_tips(8);
        // h1 should be a tip, genesis should not (it was consumed)
        assert!(tips.contains(&h1));
        assert!(!tips.contains(&genesis));
    }

    #[test]
    fn insert_rejects_orphan() {
        let mut dag = DagState::new();
        let orphan = hash_from(1);
        let unknown_parent = hash_from(2);
        let result = dag.insert(orphan, &[unknown_parent]);
        assert_eq!(result, Err(DagError::OrphanMessage));
    }

    #[test]
    fn insert_rejects_duplicate() {
        let mut dag = DagState::new();
        let h = hash_from(1);
        dag.insert(h, &[DagState::genesis_hash()]).unwrap();
        let result = dag.insert(h, &[DagState::genesis_hash()]);
        assert_eq!(result, Err(DagError::DuplicateMessage));
    }

    #[test]
    fn is_seen_returns_false_for_unknown() {
        let dag = DagState::new();
        assert!(!dag.is_seen(&hash_from(42)));
    }

    #[test]
    fn tips_bounded_to_max() {
        let mut dag = DagState::new();
        let genesis = DagState::genesis_hash();

        // Insert MAX_DAG_TIPS + 2 messages, each referencing genesis
        // (genesis remains a tip until consumed, but we reference it each time)
        let mut prev = genesis;
        for i in 0..(MAX_DAG_TIPS + 2) {
            let h = hash_from(i as u8 + 10);
            dag.insert(h, &[prev]).unwrap();
            prev = h;
        }

        assert!(dag.current_tips(MAX_DAG_TIPS).len() <= MAX_DAG_TIPS);
    }

    #[test]
    fn chain_of_messages_accepted() {
        let mut dag = DagState::new();
        let mut prev = DagState::genesis_hash();
        for i in 0..20u8 {
            let h = hash_from(i);
            dag.insert(h, &[prev]).unwrap();
            prev = h;
        }
        assert_eq!(dag.sequence(), 20);
    }

    #[test]
    fn current_tips_respects_max() {
        let mut dag = DagState::new();
        let genesis = DagState::genesis_hash();

        // Create 3 branches from genesis
        let h1 = hash_from(1);
        let h2 = hash_from(2);
        let h3 = hash_from(3);
        dag.insert(h1, &[genesis]).unwrap();
        dag.insert(h2, &[genesis]).unwrap();
        dag.insert(h3, &[genesis]).unwrap();

        let tips_1 = dag.current_tips(1);
        assert_eq!(tips_1.len(), 1);

        let tips_2 = dag.current_tips(2);
        assert_eq!(tips_2.len(), 2);
    }

    #[test]
    fn multiple_parents_merge_tips() {
        let mut dag = DagState::new();
        let genesis = DagState::genesis_hash();

        let h1 = hash_from(1);
        let h2 = hash_from(2);
        dag.insert(h1, &[genesis]).unwrap();
        dag.insert(h2, &[genesis]).unwrap();

        // Merge: h3 references both h1 and h2
        let h3 = hash_from(3);
        dag.insert(h3, &[h1, h2]).unwrap();

        let tips = dag.current_tips(8);
        assert_eq!(tips.len(), 1);
        assert!(tips.contains(&h3));
    }
}
