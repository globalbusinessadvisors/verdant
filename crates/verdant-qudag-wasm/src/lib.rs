use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use verdant_core::types::MessageHash;
use verdant_qudag::dag::DagState;

// ---------------------------------------------------------------------------
// Public WASM API
// ---------------------------------------------------------------------------

/// Verify a CRYSTALS-Dilithium (ML-DSA-65 / FIPS 204) signature.
///
/// Uses the pure-Rust `ml-dsa` crate so this works in WebAssembly without
/// any C FFI.  The encoded formats match FIPS 204 §7 (verifying key) and
/// §8 (signature).
///
/// Returns `true` if the signature is valid for the given data and public key.
#[wasm_bindgen]
pub fn verify_dilithium_signature(data: &[u8], signature: &[u8], public_key: &[u8]) -> bool {
    verify_dilithium_inner(data, signature, public_key)
}

/// Verify that all parent hashes in a message exist in a set of known tips.
///
/// `message_json`: JSON object with `"dag_parents"` field (array of hex-encoded
///                 32-byte hashes).
/// `known_tips_json`: JSON array of hex-encoded 32-byte hashes representing the
///                    known DAG state (genesis + all seen hashes).
#[wasm_bindgen]
pub fn verify_dag_parents(message_json: &str, known_tips_json: &str) -> bool {
    verify_dag_parents_inner(message_json, known_tips_json)
}

/// Verify integrity of proposal votes.
///
/// Input JSON:
/// ```json
/// {
///   "proposal_id": "abc123",
///   "votes": [
///     { "voter_zone": "aabb0011", "vote": "Yes", "data": "<hex>",
///       "signature": "<hex>", "public_key": "<hex>" }
///   ]
/// }
/// ```
///
/// Returns a JSON `VerificationResult` via `JsValue`.
#[wasm_bindgen]
pub fn verify_proposal_votes(proposal_json: &str) -> JsValue {
    let result = verify_proposal_votes_inner(proposal_json);
    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}

// ---------------------------------------------------------------------------
// Internal implementation (testable without JsValue)
// ---------------------------------------------------------------------------

fn verify_dilithium_inner(data: &[u8], signature: &[u8], public_key: &[u8]) -> bool {
    use ml_dsa::{EncodedSignature, EncodedVerifyingKey, MlDsa65, Signature, VerifyingKey};

    let vk_enc: &EncodedVerifyingKey<MlDsa65> = match public_key.try_into() {
        Ok(a) => a,
        Err(_) => return false,
    };
    let vk = VerifyingKey::<MlDsa65>::decode(vk_enc);

    let sig_enc: &EncodedSignature<MlDsa65> = match signature.try_into() {
        Ok(a) => a,
        Err(_) => return false,
    };
    let sig = match Signature::<MlDsa65>::decode(sig_enc) {
        Some(s) => s,
        None => return false,
    };

    vk.verify_with_context(data, &[], &sig)
}

fn verify_dag_parents_inner(message_json: &str, known_tips_json: &str) -> bool {
    let msg: DagParentsInput = match serde_json::from_str(message_json) {
        Ok(m) => m,
        Err(_) => return false,
    };

    let known: Vec<String> = match serde_json::from_str(known_tips_json) {
        Ok(k) => k,
        Err(_) => return false,
    };

    let mut dag = DagState::new();
    let genesis = DagState::genesis_hash();

    for hex_str in &known {
        if let Some(hash) = hex_to_hash(hex_str) {
            if hash != genesis {
                let _ = dag.insert(hash, &[genesis]);
            }
        }
    }

    let parents: Vec<MessageHash> = msg
        .dag_parents
        .iter()
        .filter_map(|h| hex_to_hash(h))
        .collect();

    dag.validate_parents(&parents).is_ok()
}

fn verify_proposal_votes_inner(proposal_json: &str) -> VerificationResult {
    let input: ProposalVotesInput = match serde_json::from_str(proposal_json) {
        Ok(i) => i,
        Err(_) => {
            return VerificationResult {
                valid: false,
                total_votes: 0,
                discrepancies: 0,
            };
        }
    };

    let total_votes = input.votes.len() as u32;
    let mut discrepancies: u32 = 0;

    for vote in &input.votes {
        let data = match hex::decode(&vote.data) {
            Some(d) => d,
            None => {
                discrepancies += 1;
                continue;
            }
        };

        let sig_bytes = match hex::decode(&vote.signature) {
            Some(s) => s,
            None => {
                discrepancies += 1;
                continue;
            }
        };

        let pk_bytes = match hex::decode(&vote.public_key) {
            Some(p) => p,
            None => {
                discrepancies += 1;
                continue;
            }
        };

        if !verify_dilithium_inner(&data, &sig_bytes, &pk_bytes) {
            discrepancies += 1;
        }
    }

    VerificationResult {
        valid: discrepancies == 0,
        total_votes,
        discrepancies,
    }
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct DagParentsInput {
    dag_parents: Vec<String>,
}

#[derive(Deserialize)]
struct ProposalVotesInput {
    #[allow(dead_code)]
    proposal_id: String,
    votes: Vec<VoteInput>,
}

#[derive(Deserialize)]
struct VoteInput {
    #[allow(dead_code)]
    voter_zone: String,
    #[allow(dead_code)]
    vote: String,
    data: String,
    signature: String,
    public_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct VerificationResult {
    valid: bool,
    total_votes: u32,
    discrepancies: u32,
}

// ---------------------------------------------------------------------------
// Hex utilities
// ---------------------------------------------------------------------------

mod hex {
    pub fn decode(s: &str) -> Option<Vec<u8>> {
        if s.len() % 2 != 0 {
            return None;
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
            .collect()
    }

    #[cfg(test)]
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}

fn hex_to_hash(s: &str) -> Option<MessageHash> {
    let bytes = hex::decode(s)?;
    if bytes.len() != 32 {
        return None;
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Some(MessageHash(arr))
}

// ---------------------------------------------------------------------------
// Tests (standard Rust unit tests, not in-browser)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ml_dsa::{KeyGen, MlDsa65};

    fn generate_test_keypair() -> (ml_dsa::SigningKey<MlDsa65>, ml_dsa::VerifyingKey<MlDsa65>) {
        let seed = ml_dsa::B32::default(); // deterministic zero seed for tests
        let kp = MlDsa65::from_seed(&seed);
        let sk = kp.signing_key().clone();
        let vk = kp.verifying_key().clone();
        (sk, vk)
    }

    fn sign_data(sk: &ml_dsa::SigningKey<MlDsa65>, data: &[u8]) -> Vec<u8> {
        let sig = sk.sign_deterministic(data, &[]).unwrap();
        sig.encode().to_vec()
    }

    fn encode_vk(vk: &ml_dsa::VerifyingKey<MlDsa65>) -> Vec<u8> {
        vk.encode().to_vec()
    }

    #[test]
    fn verify_dilithium_roundtrip() {
        let (sk, vk) = generate_test_keypair();
        let data = b"governance vote payload";
        let sig = sign_data(&sk, data);
        let pk = encode_vk(&vk);

        assert!(verify_dilithium_inner(data, &sig, &pk));
    }

    #[test]
    fn verify_dilithium_rejects_bad_data() {
        let (sk, vk) = generate_test_keypair();
        let sig = sign_data(&sk, b"original");
        let pk = encode_vk(&vk);

        assert!(!verify_dilithium_inner(b"tampered", &sig, &pk));
    }

    #[test]
    fn verify_dilithium_rejects_wrong_key() {
        let (sk1, _) = generate_test_keypair();
        let seed2 = {
            let mut s = ml_dsa::B32::default();
            s[0] = 0x99;
            s
        };
        let kp2 = MlDsa65::from_seed(&seed2);
        let vk2 = kp2.verifying_key();

        let sig = sign_data(&sk1, b"data");
        let pk2 = encode_vk(vk2);

        assert!(!verify_dilithium_inner(b"data", &sig, &pk2));
    }

    #[test]
    fn verify_dilithium_rejects_empty_inputs() {
        assert!(!verify_dilithium_inner(b"data", &[], &[]));
    }

    #[test]
    fn verify_dag_parents_valid() {
        let genesis = DagState::genesis_hash();
        let genesis_hex = hex::encode(&genesis.0);

        let msg_json = format!(r#"{{"dag_parents": ["{genesis_hex}"]}}"#);
        let known_json = format!(r#"["{genesis_hex}"]"#);

        assert!(verify_dag_parents_inner(&msg_json, &known_json));
    }

    #[test]
    fn verify_dag_parents_rejects_unknown() {
        let unknown = "ff".repeat(32);
        let genesis_hex = hex::encode(&DagState::genesis_hash().0);

        let msg_json = format!(r#"{{"dag_parents": ["{unknown}"]}}"#);
        let known_json = format!(r#"["{genesis_hex}"]"#);

        assert!(!verify_dag_parents_inner(&msg_json, &known_json));
    }

    #[test]
    fn verify_proposal_votes_valid() {
        let (sk, vk) = generate_test_keypair();
        let data = b"vote:Yes:proposal-abc";
        let sig = sign_data(&sk, data);
        let pk = encode_vk(&vk);

        let json = format!(
            r#"{{
                "proposal_id": "abc",
                "votes": [{{
                    "voter_zone": "z1",
                    "vote": "Yes",
                    "data": "{}",
                    "signature": "{}",
                    "public_key": "{}"
                }}]
            }}"#,
            hex::encode(data),
            hex::encode(&sig),
            hex::encode(&pk),
        );

        let result = verify_proposal_votes_inner(&json);
        assert!(result.valid);
        assert_eq!(result.total_votes, 1);
        assert_eq!(result.discrepancies, 0);
    }

    #[test]
    fn verify_proposal_votes_catches_bad_hex() {
        let json = r#"{
            "proposal_id": "abc",
            "votes": [{
                "voter_zone": "z1",
                "vote": "Yes",
                "data": "not_hex",
                "signature": "also_not_hex",
                "public_key": "nope"
            }]
        }"#;

        let result = verify_proposal_votes_inner(json);
        assert!(!result.valid);
        assert_eq!(result.total_votes, 1);
        assert_eq!(result.discrepancies, 1);
    }

    #[test]
    fn verify_proposal_votes_invalid_json() {
        let result = verify_proposal_votes_inner("not json");
        assert!(!result.valid);
        assert_eq!(result.total_votes, 0);
    }

    #[test]
    fn hex_roundtrip() {
        assert_eq!(hex::decode("aabb"), Some(vec![0xaa, 0xbb]));
        assert_eq!(hex::decode("00ff"), Some(vec![0x00, 0xff]));
        assert!(hex::decode("xyz").is_none());
        assert_eq!(hex::encode(&[0xaa, 0xbb]), "aabb");
    }
}
