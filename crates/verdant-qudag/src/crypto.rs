//! Concrete [`PostQuantumCrypto`] implementation using CRYSTALS-Dilithium
//! (ML-DSA-65 / FIPS 204) for signatures and CRYSTALS-Kyber (ML-KEM-1024
//! / FIPS 203) for key encapsulation.
//!
//! This module is only available with the `std` feature because the
//! underlying `pqcrypto-*` FFI crates require a standard library.

use pqcrypto_dilithium::dilithium3;
use pqcrypto_kyber::kyber1024;
use pqcrypto_traits::kem::{
    Ciphertext as KemCiphertext, PublicKey as KemPublicKey, SharedSecret as KemSharedSecret,
};
use pqcrypto_traits::sign::{DetachedSignature, PublicKey as SignPublicKey};

use verdant_core::error::CryptoError;
use verdant_core::traits::PostQuantumCrypto;
use verdant_core::types;

/// Holds Dilithium signing and Kyber KEM key pairs.
pub struct PqCrypto {
    sign_sk: dilithium3::SecretKey,
    sign_pk: dilithium3::PublicKey,
    kem_sk: kyber1024::SecretKey,
    kem_pk: kyber1024::PublicKey,
}

impl PqCrypto {
    /// Generate fresh Dilithium + Kyber key pairs.
    pub fn generate() -> Self {
        let (sign_pk, sign_sk) = dilithium3::keypair();
        let (kem_pk, kem_sk) = kyber1024::keypair();
        Self {
            sign_sk,
            sign_pk,
            kem_sk,
            kem_pk,
        }
    }

    /// Return our Dilithium public key in wire format.
    pub fn signing_public_key(&self) -> types::PublicKey {
        let bytes = self.sign_pk.as_bytes();
        types::PublicKey {
            bytes: heapless::Vec::from_slice(bytes).expect("Dilithium3 pk fits in 1952 bytes"),
        }
    }

    /// Return our Kyber public key in wire format.
    pub fn kem_public_key(&self) -> types::PublicKey {
        let bytes = self.kem_pk.as_bytes();
        types::PublicKey {
            bytes: heapless::Vec::from_slice(bytes).expect("Kyber1024 pk fits in 1952 bytes"),
        }
    }
}

impl PostQuantumCrypto for PqCrypto {
    fn sign(&self, data: &[u8]) -> Result<types::DilithiumSignature, CryptoError> {
        let sig = dilithium3::detached_sign(data, &self.sign_sk);
        let sig_bytes = sig.as_bytes();
        let bytes = heapless::Vec::from_slice(sig_bytes)
            .map_err(|_| CryptoError::SigningFailed)?;
        Ok(types::DilithiumSignature { bytes })
    }

    fn verify(
        &self,
        data: &[u8],
        sig: &types::DilithiumSignature,
        pk: &types::PublicKey,
    ) -> Result<bool, CryptoError> {
        let sig = dilithium3::DetachedSignature::from_bytes(&sig.bytes)
            .map_err(|_| CryptoError::InvalidSignatureLength)?;
        let pk = dilithium3::PublicKey::from_bytes(&pk.bytes)
            .map_err(|_| CryptoError::InvalidKeyLength)?;
        Ok(dilithium3::verify_detached_signature(&sig, data, &pk).is_ok())
    }

    fn encapsulate(
        &self,
        pk: &types::PublicKey,
    ) -> Result<(types::SharedSecret, types::Ciphertext), CryptoError> {
        let pk = kyber1024::PublicKey::from_bytes(&pk.bytes)
            .map_err(|_| CryptoError::InvalidKeyLength)?;
        let (ss, ct) = kyber1024::encapsulate(&pk);

        let ss_bytes: [u8; 32] = ss
            .as_bytes()
            .try_into()
            .map_err(|_| CryptoError::EncapsulationFailed)?;

        let ct_bytes = heapless::Vec::from_slice(ct.as_bytes())
            .map_err(|_| CryptoError::EncapsulationFailed)?;

        Ok((
            types::SharedSecret { bytes: ss_bytes },
            types::Ciphertext { bytes: ct_bytes },
        ))
    }

    fn decapsulate(&self, ct: &types::Ciphertext) -> Result<types::SharedSecret, CryptoError> {
        let ct = kyber1024::Ciphertext::from_bytes(&ct.bytes)
            .map_err(|_| CryptoError::DecapsulationFailed)?;
        let ss = kyber1024::decapsulate(&ct, &self.kem_sk);

        let ss_bytes: [u8; 32] = ss
            .as_bytes()
            .try_into()
            .map_err(|_| CryptoError::DecapsulationFailed)?;

        Ok(types::SharedSecret { bytes: ss_bytes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify_roundtrip() {
        let crypto = PqCrypto::generate();
        let data = b"Verdant mesh message";

        let sig = crypto.sign(data).unwrap();
        let pk = crypto.signing_public_key();
        let valid = crypto.verify(data, &sig, &pk).unwrap();

        assert!(valid);
    }

    #[test]
    fn verify_rejects_wrong_data() {
        let crypto = PqCrypto::generate();
        let sig = crypto.sign(b"original").unwrap();
        let pk = crypto.signing_public_key();

        let valid = crypto.verify(b"tampered", &sig, &pk).unwrap();
        assert!(!valid);
    }

    #[test]
    fn verify_rejects_wrong_key() {
        let crypto1 = PqCrypto::generate();
        let crypto2 = PqCrypto::generate();

        let data = b"message";
        let sig = crypto1.sign(data).unwrap();
        let wrong_pk = crypto2.signing_public_key();

        let valid = crypto1.verify(data, &sig, &wrong_pk).unwrap();
        assert!(!valid);
    }

    #[test]
    fn kem_encapsulate_decapsulate_roundtrip() {
        let crypto = PqCrypto::generate();
        let pk = crypto.kem_public_key();

        let (ss_sender, ct) = crypto.encapsulate(&pk).unwrap();
        let ss_receiver = crypto.decapsulate(&ct).unwrap();

        assert_eq!(ss_sender.bytes, ss_receiver.bytes);
    }

    #[test]
    fn kem_wrong_secret_key_produces_different_secret() {
        let crypto1 = PqCrypto::generate();
        let crypto2 = PqCrypto::generate();

        let pk1 = crypto1.kem_public_key();
        let (_ss_sender, ct) = crypto1.encapsulate(&pk1).unwrap();

        // crypto2 has a different secret key
        let ss_wrong = crypto2.decapsulate(&ct).unwrap();
        assert_ne!(_ss_sender.bytes, ss_wrong.bytes);
    }
}
