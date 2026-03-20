# ADR-002: QuDAG Post-Quantum Mesh Protocol

**Status:** Accepted
**Date:** 2026-03-20
**Deciders:** Verdant Core Team
**SPARC Refs:** FR-M3, FR-M4, NFR-10, G3, G6

## Context

The Verdant mesh must provide quantum-resistant, anonymous, decentralized communication between nodes. Messages must be ordered without a central coordinator, sender/recipient anonymity must be preserved, and the protocol must resist replay attacks. The mesh may operate for decades — well into the era of cryptographically relevant quantum computers (CRQC).

Candidate approaches:
1. **Standard TLS/DTLS over mesh** — well-understood but requires central CA, not quantum-resistant
2. **Signal Protocol adaptation** — excellent for two-party, but assumes server-mediated key exchange
3. **Custom DAG + classical crypto (Ed25519)** — lightweight, but not quantum-resistant
4. **Custom DAG + NIST post-quantum (CRYSTALS-Dilithium/Kyber)** — future-proof, heavier signatures

## Decision

Use a custom **DAG-based message ordering protocol** with **CRYSTALS-Dilithium** for signatures and **CRYSTALS-Kyber** for key encapsulation, combined with **onion routing** (minimum 3 hops) for sender anonymity.

## Rationale

- **DAG over blockchain:** A blockchain requires consensus on total ordering, which is impossible without a central coordinator in a partition-prone mesh. A DAG requires only partial ordering (each message references its causal predecessors), which nodes can verify independently.
- **Dilithium over Ed25519:** NIST has standardized CRYSTALS-Dilithium (FIPS 204) as the primary post-quantum signature scheme. Signature size is larger (~2.4 KB vs. 64 bytes) but acceptable for the mesh's message rate (~1 msg/30s/node).
- **Kyber over X25519:** CRYSTALS-Kyber (FIPS 203) is the standardized PQ key encapsulation mechanism. Ciphertext is ~1 KB vs. 32 bytes, but key exchange happens infrequently (session setup, not per-message).
- **Onion routing for anonymity:** Vermont's data sovereignty requirement means no node should know both the sender and recipient of a message. Three-hop onion routing achieves this with acceptable latency (3 × single-hop latency).
- **Hybrid option as fallback:** If Dilithium proves too slow on ESP32 (see Risk Register), we can use a hybrid scheme: Ed25519 for real-time verification + Dilithium for periodic batch verification.

## Consequences

- **Positive:** Quantum-resistant from day one; no central authority; replay-resistant via DAG ordering; anonymous routing
- **Negative:** Larger message sizes (~4 KB vs. ~200 bytes for classical); higher CPU cost for signature verification (~40M cycles on ESP32); more complex implementation
- **Mitigated by:** Bandwidth-adaptive compression (ADR-008); signature caching for frequently-seen peers; batch verification where possible

## London School TDD Approach

```rust
// verdant-qudag is tested by mocking the crypto and transport layers.
// The DAG logic itself is pure and tested via state verification,
// but its INTERACTIONS with crypto and transport are tested London School style.

pub trait Signer {
    fn sign(&self, data: &[u8]) -> Result<Signature, CryptoError>;
    fn verify(&self, data: &[u8], sig: &Signature, pubkey: &PublicKey) -> Result<bool, CryptoError>;
}

pub trait KeyEncapsulation {
    fn encapsulate(&self, pubkey: &PublicKey) -> Result<(SharedSecret, Ciphertext), CryptoError>;
    fn decapsulate(&self, ciphertext: &Ciphertext) -> Result<SharedSecret, CryptoError>;
}

pub trait RelayTransport {
    fn send_to_neighbor(&mut self, neighbor: NodeId, frame: &[u8]) -> Result<(), TransportError>;
    fn receive_from_neighbors(&mut self) -> Result<Vec<(NodeId, Vec<u8>)>, TransportError>;
}

#[cfg(test)]
mod tests {
    use mockall::predicate::*;

    #[test]
    fn send_message_wraps_in_three_onion_layers_and_transmits() {
        let mut mock_signer = MockSigner::new();
        let mut mock_kem = MockKeyEncapsulation::new();
        let mut mock_transport = MockRelayTransport::new();

        // GIVEN: a routing table with a known path of 3 hops
        let path = vec![hop_a, hop_b, hop_c];

        // THEN: KEM is called 3 times (once per onion layer)
        mock_kem.expect_encapsulate()
            .times(3)
            .returning(|_| Ok((test_secret(), test_ciphertext())));

        // AND: signer is called once for the inner message
        mock_signer.expect_sign()
            .times(1)
            .returning(|_| Ok(test_signature()));

        // AND: transport sends to first hop only
        mock_transport.expect_send_to_neighbor()
            .with(eq(hop_a.node_id), always())
            .times(1)
            .returning(|_, _| Ok(()));

        // WHEN: we send a message
        let protocol = QuDagProtocol::new(mock_signer, mock_kem, mock_transport);
        protocol.send_message(payload, destination_zone, &routing_table);
    }

    #[test]
    fn receive_drops_message_with_invalid_dag_parents() {
        let mut mock_signer = MockSigner::new();
        let mut mock_transport = MockRelayTransport::new();

        // GIVEN: a message referencing DAG parents we haven't seen
        let orphan_msg = QuDagMessage {
            dag_parents: vec![unknown_hash()],
            ..valid_message()
        };

        // THEN: signer.verify is never called (we reject before crypto)
        mock_signer.expect_verify().times(0);

        // AND: transport.send is never called (message is dropped)
        mock_transport.expect_send_to_neighbor().times(0);

        let protocol = QuDagProtocol::new(mock_signer, mock_kem, mock_transport);
        let result = protocol.receive_and_relay(orphan_msg);
        assert!(result.is_dropped());
    }

    #[test]
    fn receive_drops_message_with_invalid_signature() {
        let mut mock_signer = MockSigner::new();

        // GIVEN: valid DAG parents but bad signature
        mock_signer.expect_verify()
            .times(1)
            .returning(|_, _, _| Ok(false));  // signature invalid

        // THEN: message is dropped, not forwarded
        let mut mock_transport = MockRelayTransport::new();
        mock_transport.expect_send_to_neighbor().times(0);

        let protocol = QuDagProtocol::new(mock_signer, mock_kem, mock_transport);
        let result = protocol.receive_and_relay(signed_message_with_bad_sig());
        assert!(result.is_dropped());
    }
}
```
