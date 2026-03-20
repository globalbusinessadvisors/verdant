use serde::{Deserialize, Serialize};
use verdant_core::types::{Ciphertext, DilithiumSignature, MessageHash};

/// Maximum number of DAG parent references per message.
pub const MAX_PARENTS: usize = 3;

/// Maximum encrypted payload size in bytes.
pub const MAX_ENCRYPTED_PAYLOAD: usize = 4096;

/// A message in the QuDAG protocol.
///
/// Each message references 1–3 previously seen DAG parents, carries an
/// encrypted payload, and is signed with a post-quantum Dilithium signature.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuDagMessage {
    /// References to causally preceding messages in the DAG.
    pub dag_parents: heapless::Vec<MessageHash, MAX_PARENTS>,
    /// Encrypted message content.
    pub payload: EncryptedPayload,
    /// Dilithium signature over the payload.
    pub signature: DilithiumSignature,
    /// Remaining hop count (decremented at each relay).
    pub ttl: u8,
    /// Sender monotonic timestamp (milliseconds).
    pub timestamp: u64,
}

/// Encrypted payload carried within a [`QuDagMessage`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EncryptedPayload {
    /// Symmetrically encrypted content.
    pub ciphertext: heapless::Vec<u8, MAX_ENCRYPTED_PAYLOAD>,
    /// KEM ciphertext for initial key establishment (present on first
    /// message of a session; absent for subsequent messages using the
    /// same shared secret).
    pub kem_ciphertext: Option<Ciphertext>,
}

impl QuDagMessage {
    /// Compute a deterministic hash of this message for DAG insertion.
    ///
    /// Uses a simple byte-mixing function over the serialized content.
    /// In production, this would be SHA-256; here we use a lightweight
    /// hash suitable for `no_std` without pulling in a full SHA crate.
    pub fn compute_hash(&self) -> MessageHash {
        // Hash over: parents + payload ciphertext + ttl + timestamp
        let mut h = [0u8; 32];

        // Mix in parent hashes
        for parent in &self.dag_parents {
            for (i, b) in parent.0.iter().enumerate() {
                h[i] ^= b;
            }
        }

        // Mix in payload bytes (sample first 32 or fewer)
        let take = 32.min(self.payload.ciphertext.len());
        for (h_byte, &p_byte) in h.iter_mut().zip(self.payload.ciphertext[..take].iter()) {
            *h_byte = h_byte.wrapping_add(p_byte).wrapping_mul(0x9E);
        }

        // Mix in ttl and timestamp
        h[0] ^= self.ttl;
        let ts_bytes = self.timestamp.to_le_bytes();
        for i in 0..8 {
            h[24 + i] ^= ts_bytes[i];
        }

        // Avalanche pass
        for i in 1..32 {
            h[i] ^= h[i - 1].wrapping_mul(0x6D).wrapping_add(0xA7);
        }
        for i in (0..31).rev() {
            h[i] ^= h[i + 1].wrapping_mul(0x3B).wrapping_add(0x5F);
        }

        MessageHash(h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_message(ttl: u8, ts: u64) -> QuDagMessage {
        let mut parents = heapless::Vec::new();
        let _ = parents.push(MessageHash([0xAA; 32]));

        let mut ct = heapless::Vec::new();
        for b in 0..64u8 {
            let _ = ct.push(b);
        }

        QuDagMessage {
            dag_parents: parents,
            payload: EncryptedPayload {
                ciphertext: ct,
                kem_ciphertext: None,
            },
            signature: DilithiumSignature {
                bytes: heapless::Vec::new(),
            },
            ttl,
            timestamp: ts,
        }
    }

    #[test]
    fn message_roundtrip_postcard() {
        let msg = make_message(5, 1234567890);
        let bytes = postcard::to_allocvec(&msg).unwrap();
        let decoded: QuDagMessage = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn message_with_kem_ciphertext_roundtrip() {
        let mut msg = make_message(3, 42);
        msg.payload.kem_ciphertext = Some(Ciphertext {
            bytes: {
                let mut v = heapless::Vec::new();
                for i in 0..64u8 {
                    let _ = v.push(i);
                }
                v
            },
        });

        let bytes = postcard::to_allocvec(&msg).unwrap();
        let decoded: QuDagMessage = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn compute_hash_deterministic() {
        let msg = make_message(5, 100);
        let h1 = msg.compute_hash();
        let h2 = msg.compute_hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn compute_hash_differs_by_content() {
        let m1 = make_message(5, 100);
        let m2 = make_message(5, 200);
        assert_ne!(m1.compute_hash(), m2.compute_hash());
    }

    #[test]
    fn compute_hash_differs_by_ttl() {
        let m1 = make_message(5, 100);
        let m2 = make_message(10, 100);
        assert_ne!(m1.compute_hash(), m2.compute_hash());
    }
}
