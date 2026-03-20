#![no_main]

use libfuzzer_sys::fuzz_target;
use verdant_core::types::MessageHash;
use verdant_qudag::dag::DagState;

/// Fuzz input: a sequence of (hash, parent_indices) pairs.
/// Parent indices reference previously inserted hashes.
#[derive(Debug, arbitrary::Arbitrary)]
struct FuzzInput {
    operations: Vec<DagOp>,
}

#[derive(Debug, arbitrary::Arbitrary)]
struct DagOp {
    hash_seed: [u8; 32],
    /// Indices into previously-inserted hashes to use as parents.
    parent_indices: Vec<u8>,
}

fuzz_target!(|input: FuzzInput| {
    let mut dag = DagState::new();
    let mut inserted: Vec<MessageHash> = vec![DagState::genesis_hash()];

    for op in &input.operations {
        let hash = MessageHash(op.hash_seed);

        // Build parents from indices into the `inserted` list.
        let parents: Vec<MessageHash> = op
            .parent_indices
            .iter()
            .take(3)
            .filter_map(|&idx| inserted.get(idx as usize % inserted.len()).copied())
            .collect();

        if parents.is_empty() {
            // Try with genesis as fallback — should work or fail gracefully
            let _ = dag.insert(hash, &[DagState::genesis_hash()]);
        } else {
            let _ = dag.insert(hash, &parents);
        }

        if dag.is_seen(&hash) {
            inserted.push(hash);
        }
    }

    // Invariants that must always hold
    assert!(dag.is_seen(&DagState::genesis_hash()));
    let tips = dag.current_tips(8);
    assert!(tips.len() <= 8);
});
