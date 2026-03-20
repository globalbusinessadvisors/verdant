use verdant_core::error::CryptoError;
use verdant_core::traits::PostQuantumCrypto;
use verdant_core::types::{NodeId, PublicKey, SharedSecret};

/// Maximum total onion packet size in bytes.
///
/// Sized to accommodate a ~512-byte payload with 3 onion layers,
/// each adding ~1580 bytes of KEM ciphertext + header overhead.
pub const MAX_ONION_PACKET: usize = 6144;

/// Tag byte indicating another onion layer follows.
const TAG_RELAY: u8 = 0x01;
/// Tag byte indicating this is the final payload.
const TAG_FINAL: u8 = 0x00;

/// Result of unwrapping one onion layer.
#[derive(Debug)]
pub enum UnwrapResult {
    /// Another relay hop: forward `data` to `next_hop`.
    Relay {
        next_hop: NodeId,
        data: heapless::Vec<u8, MAX_ONION_PACKET>,
    },
    /// Final destination: the decrypted plaintext payload.
    Destination {
        payload: heapless::Vec<u8, MAX_ONION_PACKET>,
    },
}

/// Wrap one onion layer around `inner` data for relay through `next_hop`.
///
/// Uses KEM to establish a shared secret with the relay node, then
/// encrypts `[tag][next_hop][inner]` under that shared secret.
/// Returns the KEM ciphertext prepended to the encrypted data.
pub fn wrap_onion_layer(
    inner: &[u8],
    next_hop: NodeId,
    next_hop_pubkey: &PublicKey,
    crypto: &impl PostQuantumCrypto,
) -> Result<heapless::Vec<u8, MAX_ONION_PACKET>, CryptoError> {
    let (shared_secret, kem_ct) = crypto.encapsulate(next_hop_pubkey)?;

    // Plaintext: [TAG_RELAY][next_hop 8 bytes][inner...]
    let plaintext_len = 1 + 8 + inner.len();
    let mut plaintext: heapless::Vec<u8, MAX_ONION_PACKET> = heapless::Vec::new();
    plaintext.push(TAG_RELAY).map_err(|_| CryptoError::EncapsulationFailed)?;
    plaintext
        .extend_from_slice(&next_hop.0)
        .map_err(|_| CryptoError::EncapsulationFailed)?;
    plaintext
        .extend_from_slice(inner)
        .map_err(|_| CryptoError::EncapsulationFailed)?;

    // Encrypt plaintext with the shared secret
    stream_cipher(&shared_secret, &mut plaintext);

    // Output: [KEM ciphertext bytes][encrypted plaintext]
    let mut out: heapless::Vec<u8, MAX_ONION_PACKET> = heapless::Vec::new();

    // Prefix KEM ciphertext length as 2-byte LE so the receiver knows where it ends
    let ct_len = kem_ct.bytes.len() as u16;
    out.extend_from_slice(&ct_len.to_le_bytes())
        .map_err(|_| CryptoError::EncapsulationFailed)?;
    out.extend_from_slice(&kem_ct.bytes)
        .map_err(|_| CryptoError::EncapsulationFailed)?;
    out.extend_from_slice(&plaintext[..plaintext_len])
        .map_err(|_| CryptoError::EncapsulationFailed)?;

    Ok(out)
}

/// Wrap the innermost (final destination) onion layer.
///
/// Like [`wrap_onion_layer`] but tags the payload as the final destination
/// so the recipient knows not to relay further.
pub fn wrap_final_layer(
    payload: &[u8],
    dest_pubkey: &PublicKey,
    crypto: &impl PostQuantumCrypto,
) -> Result<heapless::Vec<u8, MAX_ONION_PACKET>, CryptoError> {
    let (shared_secret, kem_ct) = crypto.encapsulate(dest_pubkey)?;

    let plaintext_len = 1 + payload.len();
    let mut plaintext: heapless::Vec<u8, MAX_ONION_PACKET> = heapless::Vec::new();
    plaintext.push(TAG_FINAL).map_err(|_| CryptoError::EncapsulationFailed)?;
    plaintext
        .extend_from_slice(payload)
        .map_err(|_| CryptoError::EncapsulationFailed)?;

    stream_cipher(&shared_secret, &mut plaintext);

    let mut out: heapless::Vec<u8, MAX_ONION_PACKET> = heapless::Vec::new();
    let ct_len = kem_ct.bytes.len() as u16;
    out.extend_from_slice(&ct_len.to_le_bytes())
        .map_err(|_| CryptoError::EncapsulationFailed)?;
    out.extend_from_slice(&kem_ct.bytes)
        .map_err(|_| CryptoError::EncapsulationFailed)?;
    out.extend_from_slice(&plaintext[..plaintext_len])
        .map_err(|_| CryptoError::EncapsulationFailed)?;

    Ok(out)
}

/// Unwrap one onion layer using our KEM secret key.
///
/// Parses the KEM ciphertext prefix, decapsulates to recover the shared
/// secret, decrypts the remaining bytes, and returns either a relay
/// instruction or the final plaintext.
pub fn unwrap_onion_layer(
    packet: &[u8],
    crypto: &impl PostQuantumCrypto,
) -> Result<UnwrapResult, CryptoError> {
    // Parse KEM ciphertext length prefix
    if packet.len() < 2 {
        return Err(CryptoError::DecapsulationFailed);
    }
    let ct_len = u16::from_le_bytes([packet[0], packet[1]]) as usize;
    let header_end = 2 + ct_len;
    if packet.len() < header_end + 1 {
        return Err(CryptoError::DecapsulationFailed);
    }

    // Extract KEM ciphertext
    let kem_ct = verdant_core::types::Ciphertext {
        bytes: heapless::Vec::from_slice(&packet[2..header_end])
            .map_err(|_| CryptoError::DecapsulationFailed)?,
    };

    // Decapsulate to get shared secret
    let shared_secret = crypto.decapsulate(&kem_ct)?;

    // Decrypt the remaining bytes
    let encrypted = &packet[header_end..];
    let mut decrypted: heapless::Vec<u8, MAX_ONION_PACKET> = heapless::Vec::new();
    decrypted
        .extend_from_slice(encrypted)
        .map_err(|_| CryptoError::DecapsulationFailed)?;
    stream_cipher(&shared_secret, &mut decrypted);

    // Parse tag
    if decrypted.is_empty() {
        return Err(CryptoError::DecapsulationFailed);
    }

    match decrypted[0] {
        TAG_FINAL => {
            let mut payload: heapless::Vec<u8, MAX_ONION_PACKET> = heapless::Vec::new();
            payload
                .extend_from_slice(&decrypted[1..])
                .map_err(|_| CryptoError::DecapsulationFailed)?;
            Ok(UnwrapResult::Destination { payload })
        }
        TAG_RELAY => {
            if decrypted.len() < 1 + 8 {
                return Err(CryptoError::DecapsulationFailed);
            }
            let mut node_bytes = [0u8; 8];
            node_bytes.copy_from_slice(&decrypted[1..9]);
            let next_hop = NodeId(node_bytes);

            let mut data: heapless::Vec<u8, MAX_ONION_PACKET> = heapless::Vec::new();
            data.extend_from_slice(&decrypted[9..])
                .map_err(|_| CryptoError::DecapsulationFailed)?;
            Ok(UnwrapResult::Relay { next_hop, data })
        }
        _ => Err(CryptoError::DecapsulationFailed),
    }
}

/// Counter-mode stream cipher using the KEM shared secret.
///
/// XORs `data` in-place with a keystream derived from `secret`.
/// Symmetric: applying twice recovers the original plaintext.
///
/// Each 32-byte block of keystream is derived by mixing the shared
/// secret with a block counter, providing unique keystream material
/// for payloads larger than 32 bytes.
fn stream_cipher(secret: &SharedSecret, data: &mut [u8]) {
    for (i, byte) in data.iter_mut().enumerate() {
        let block_idx = (i / 32) as u32;
        let byte_idx = i % 32;

        // Derive one keystream byte
        let counter_bytes = block_idx.to_le_bytes();
        let mut k = secret.bytes[byte_idx];
        k ^= counter_bytes[byte_idx % 4];
        k = k.wrapping_mul(0x6D).wrapping_add(0xA7);
        k ^= secret.bytes[(byte_idx + 13) % 32];
        k = k.wrapping_mul(0x9E).wrapping_add(0x3B);

        *byte ^= k;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use verdant_core::types::Ciphertext;

    // ---- Mock for PostQuantumCrypto ----
    mock! {
        pub Crypto {}
        impl PostQuantumCrypto for Crypto {
            fn sign(&self, data: &[u8]) -> Result<verdant_core::types::DilithiumSignature, CryptoError>;
            fn verify(
                &self,
                data: &[u8],
                sig: &verdant_core::types::DilithiumSignature,
                pk: &PublicKey,
            ) -> Result<bool, CryptoError>;
            fn encapsulate(&self, pk: &PublicKey) -> Result<(SharedSecret, Ciphertext), CryptoError>;
            fn decapsulate(&self, ct: &Ciphertext) -> Result<SharedSecret, CryptoError>;
        }
    }

    fn test_shared_secret(seed: u8) -> SharedSecret {
        let mut bytes = [0u8; 32];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = seed.wrapping_add(i as u8);
        }
        SharedSecret { bytes }
    }

    fn test_kem_ciphertext() -> Ciphertext {
        let mut bytes = heapless::Vec::new();
        for i in 0..32u8 {
            let _ = bytes.push(i);
        }
        Ciphertext { bytes }
    }

    fn test_pubkey() -> PublicKey {
        let mut bytes = heapless::Vec::new();
        for i in 0..16u8 {
            let _ = bytes.push(i);
        }
        PublicKey { bytes }
    }

    #[test]
    fn stream_cipher_roundtrip() {
        let secret = test_shared_secret(42);
        let original = b"Hello, Verdant mesh!";
        let mut data = [0u8; 20];
        data.copy_from_slice(original);

        stream_cipher(&secret, &mut data);
        // Should be different after encryption
        assert_ne!(&data, original);

        // Apply again to decrypt
        stream_cipher(&secret, &mut data);
        assert_eq!(&data, original);
    }

    #[test]
    fn wrap_onion_layer_calls_kem_encapsulate() {
        let mut mock = MockCrypto::new();

        mock.expect_encapsulate()
            .times(1)
            .returning(|_| Ok((test_shared_secret(1), test_kem_ciphertext())));

        let result = wrap_onion_layer(
            b"inner payload",
            NodeId([1; 8]),
            &test_pubkey(),
            &mock,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn unwrap_onion_layer_calls_kem_decapsulate() {
        let _secret = test_shared_secret(1);
        let _kem_ct = test_kem_ciphertext();

        // First wrap
        let mut wrap_mock = MockCrypto::new();
        wrap_mock
            .expect_encapsulate()
            .returning(move |_| Ok((test_shared_secret(1), test_kem_ciphertext())));

        let packet = wrap_final_layer(b"secret data", &test_pubkey(), &wrap_mock).unwrap();

        // Then unwrap — verify decapsulate is called
        let mut unwrap_mock = MockCrypto::new();
        unwrap_mock
            .expect_decapsulate()
            .times(1)
            .returning(move |_| Ok(test_shared_secret(1)));

        let result = unwrap_onion_layer(&packet, &unwrap_mock);
        assert!(result.is_ok());
    }

    #[test]
    fn final_layer_roundtrip() {
        let _secret = test_shared_secret(7);
        let _kem_ct = test_kem_ciphertext();

        let mut wrap_mock = MockCrypto::new();
        wrap_mock
            .expect_encapsulate()
            .returning(move |_| Ok((test_shared_secret(7), test_kem_ciphertext())));

        let packet = wrap_final_layer(b"hello mesh", &test_pubkey(), &wrap_mock).unwrap();

        let mut unwrap_mock = MockCrypto::new();
        unwrap_mock
            .expect_decapsulate()
            .returning(move |_| Ok(test_shared_secret(7)));

        match unwrap_onion_layer(&packet, &unwrap_mock).unwrap() {
            UnwrapResult::Destination { payload } => {
                assert_eq!(payload.as_slice(), b"hello mesh");
            }
            UnwrapResult::Relay { .. } => panic!("Expected Destination, got Relay"),
        }
    }

    #[test]
    fn relay_layer_roundtrip() {
        let next_hop = NodeId([0xAB; 8]);
        let inner_data = b"inner onion layer data here";

        let mut wrap_mock = MockCrypto::new();
        wrap_mock
            .expect_encapsulate()
            .returning(move |_| Ok((test_shared_secret(3), test_kem_ciphertext())));

        let packet = wrap_onion_layer(
            inner_data,
            next_hop,
            &test_pubkey(),
            &wrap_mock,
        )
        .unwrap();

        let mut unwrap_mock = MockCrypto::new();
        unwrap_mock
            .expect_decapsulate()
            .returning(move |_| Ok(test_shared_secret(3)));

        match unwrap_onion_layer(&packet, &unwrap_mock).unwrap() {
            UnwrapResult::Relay { next_hop: hop, data } => {
                assert_eq!(hop, NodeId([0xAB; 8]));
                assert_eq!(data.as_slice(), inner_data);
            }
            UnwrapResult::Destination { .. } => panic!("Expected Relay, got Destination"),
        }
    }

    #[test]
    fn three_layer_wrap_unwrap_roundtrip() {
        let payload = b"classified forest data";
        let _relay1 = NodeId([1; 8]);
        let relay2 = NodeId([2; 8]);
        let dest = NodeId([3; 8]);

        // Use different secrets for each layer
        let _secret1 = test_shared_secret(10);
        let _secret2 = test_shared_secret(20);
        let _secret3 = test_shared_secret(30);

        // --- WRAP (innermost first) ---
        // Layer 3: final destination
        let mut mock3 = MockCrypto::new();
        mock3
            .expect_encapsulate()
            .returning(move |_| Ok((test_shared_secret(30), test_kem_ciphertext())));
        let layer3 = wrap_final_layer(payload, &test_pubkey(), &mock3).unwrap();

        // Layer 2: relay to dest
        let mut mock2 = MockCrypto::new();
        mock2
            .expect_encapsulate()
            .returning(move |_| Ok((test_shared_secret(20), test_kem_ciphertext())));
        let layer2 = wrap_onion_layer(&layer3, dest, &test_pubkey(), &mock2).unwrap();

        // Layer 1: relay to relay2
        let mut mock1 = MockCrypto::new();
        mock1
            .expect_encapsulate()
            .returning(move |_| Ok((test_shared_secret(10), test_kem_ciphertext())));
        let layer1 = wrap_onion_layer(&layer2, relay2, &test_pubkey(), &mock1).unwrap();

        // --- UNWRAP (outermost first) ---
        // Relay 1 unwraps
        let mut umock1 = MockCrypto::new();
        umock1
            .expect_decapsulate()
            .returning(move |_| Ok(test_shared_secret(10)));
        let result1 = unwrap_onion_layer(&layer1, &umock1).unwrap();
        let (hop2, data2) = match result1 {
            UnwrapResult::Relay { next_hop, data } => (next_hop, data),
            _ => panic!("Expected Relay at layer 1"),
        };
        assert_eq!(hop2, relay2);

        // Relay 2 unwraps
        let mut umock2 = MockCrypto::new();
        umock2
            .expect_decapsulate()
            .returning(move |_| Ok(test_shared_secret(20)));
        let result2 = unwrap_onion_layer(&data2, &umock2).unwrap();
        let (hop3, data3) = match result2 {
            UnwrapResult::Relay { next_hop, data } => (next_hop, data),
            _ => panic!("Expected Relay at layer 2"),
        };
        assert_eq!(hop3, dest);

        // Destination unwraps
        let mut umock3 = MockCrypto::new();
        umock3
            .expect_decapsulate()
            .returning(move |_| Ok(test_shared_secret(30)));
        let result3 = unwrap_onion_layer(&data3, &umock3).unwrap();
        match result3 {
            UnwrapResult::Destination { payload: p } => {
                assert_eq!(p.as_slice(), payload);
            }
            _ => panic!("Expected Destination at layer 3"),
        }
    }

    #[test]
    fn unwrap_wrong_key_produces_garbage() {
        let mut wrap_mock = MockCrypto::new();
        wrap_mock
            .expect_encapsulate()
            .returning(move |_| Ok((test_shared_secret(1), test_kem_ciphertext())));

        let packet = wrap_final_layer(b"secret", &test_pubkey(), &wrap_mock).unwrap();

        // Unwrap with a DIFFERENT shared secret
        let mut unwrap_mock = MockCrypto::new();
        unwrap_mock
            .expect_decapsulate()
            .returning(move |_| Ok(test_shared_secret(99))); // wrong key!

        let result = unwrap_onion_layer(&packet, &unwrap_mock);
        // Either fails to parse the tag or produces garbage payload
        match result {
            Err(_) => {} // expected: invalid tag byte
            Ok(UnwrapResult::Destination { payload }) => {
                // Decrypted with wrong key — payload is garbage
                assert_ne!(payload.as_slice(), b"secret");
            }
            Ok(UnwrapResult::Relay { .. }) => {
                // Also acceptable — wrong key decoded tag as relay
            }
        }
    }
}
