use proptest::prelude::*;
use verdant_core::types::{DilithiumSignature, MessageHash};
use verdant_qudag::dag::DagState;
use verdant_qudag::message::{EncryptedPayload, QuDagMessage};

fn arb_message_hash() -> impl Strategy<Value = MessageHash> {
    proptest::array::uniform32(any::<u8>()).prop_map(MessageHash)
}

fn arb_qudag_message() -> impl Strategy<Value = QuDagMessage> {
    (
        proptest::collection::vec(arb_message_hash(), 1..=3),
        proptest::collection::vec(any::<u8>(), 0..128),
        any::<u8>(),
        any::<u64>(),
    )
        .prop_map(|(parents, payload_bytes, ttl, timestamp)| {
            let mut dag_parents = heapless::Vec::new();
            for p in parents.iter().take(3) {
                let _ = dag_parents.push(*p);
            }

            let mut ciphertext = heapless::Vec::new();
            for b in &payload_bytes {
                let _ = ciphertext.push(*b);
            }

            QuDagMessage {
                dag_parents,
                payload: EncryptedPayload {
                    ciphertext,
                    kem_ciphertext: None,
                },
                signature: DilithiumSignature {
                    bytes: heapless::Vec::new(),
                },
                ttl,
                timestamp,
            }
        })
}

proptest! {
    /// A valid chain of messages (each referencing its predecessor) must
    /// always be accepted by the DAG.
    #[test]
    fn dag_accepts_valid_chains(chain_length in 2..50usize) {
        let mut dag = DagState::new();
        let mut prev = DagState::genesis_hash();

        for i in 0..chain_length {
            let mut h = [0u8; 32];
            let i_bytes = (i as u64).to_le_bytes();
            for j in 0..8 {
                h[j] = i_bytes[j];
            }
            // Make each hash unique with a secondary mix
            h[8] = (i as u8).wrapping_mul(0x6D);
            h[9] = (i as u8).wrapping_add(0xAB);
            let hash = MessageHash(h);

            let result = dag.insert(hash, &[prev]);
            prop_assert!(result.is_ok(), "Chain insert failed at index {}: {:?}", i, result);
            prev = hash;
        }
    }

    /// A random hash that was never inserted must be rejected as a parent.
    #[test]
    fn dag_rejects_all_orphans(unknown in arb_message_hash()) {
        let dag = DagState::new();
        // Ensure it's not the genesis hash
        if unknown != DagState::genesis_hash() {
            let result = dag.validate_parents(&[unknown]);
            prop_assert_eq!(result, Err(verdant_qudag::dag::DagError::OrphanMessage));
        }
    }

    /// Any `QuDagMessage` must survive postcard serialization round-trip.
    #[test]
    fn message_roundtrip_serialization(msg in arb_qudag_message()) {
        let bytes = postcard::to_allocvec(&msg).unwrap();
        let decoded: QuDagMessage = postcard::from_bytes(&bytes).unwrap();
        prop_assert_eq!(msg, decoded);
    }
}
