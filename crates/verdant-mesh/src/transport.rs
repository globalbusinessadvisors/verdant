use verdant_core::error::{RadioError, TransportError};
use verdant_core::traits::{MeshTransport, PostQuantumCrypto, RadioHardware};
use verdant_core::types::{MeshFrame, NodeId};
use verdant_qudag::dag::DagState;
use verdant_qudag::message::{EncryptedPayload, QuDagMessage};

use crate::routing::RoutingTable;

/// Maximum serialized frame size for radio transmission.
const MAX_WIRE_FRAME: usize = 8192;

/// The mesh communication layer wiring together radio, crypto, routing, and DAG.
pub struct MeshCommunicationLayer<R: RadioHardware, C: PostQuantumCrypto> {
    radio: R,
    crypto: C,
    routing_table: RoutingTable,
    dag_state: DagState,
    self_id: NodeId,
}

impl<R: RadioHardware, C: PostQuantumCrypto> MeshCommunicationLayer<R, C> {
    pub fn new(radio: R, crypto: C, self_id: NodeId) -> Self {
        Self {
            radio,
            crypto,
            routing_table: RoutingTable::new(self_id),
            dag_state: DagState::new(),
            self_id,
        }
    }

    pub fn routing_table(&self) -> &RoutingTable {
        &self.routing_table
    }

    pub fn routing_table_mut(&mut self) -> &mut RoutingTable {
        &mut self.routing_table
    }

    pub fn dag_state(&self) -> &DagState {
        &self.dag_state
    }

    /// Build a QuDagMessage wrapping a MeshFrame: sign it and attach DAG references.
    fn build_message(&mut self, frame: &MeshFrame) -> Result<QuDagMessage, TransportError> {
        let mut payload_buf = [0u8; 4096];
        let payload_bytes =
            postcard::to_slice(frame, &mut payload_buf).map_err(|_| TransportError::EncodingFailed)?;

        let signature = self
            .crypto
            .sign(payload_bytes)
            .map_err(|_| TransportError::EncodingFailed)?;

        let tips = self.dag_state.current_tips(3);
        let mut dag_parents = heapless::Vec::<_, 3>::new();
        for tip in tips.iter().take(3) {
            let _ = dag_parents.push(*tip);
        }

        let mut ciphertext = heapless::Vec::new();
        ciphertext
            .extend_from_slice(payload_bytes)
            .map_err(|_| TransportError::FrameTooLarge)?;

        let msg = QuDagMessage {
            dag_parents,
            payload: EncryptedPayload {
                ciphertext,
                kem_ciphertext: None,
            },
            signature,
            ttl: frame.ttl,
            timestamp: 0,
        };

        // Insert into our DAG
        let hash = msg.compute_hash();
        let parents: heapless::Vec<_, 3> = msg.dag_parents.clone();
        // Ignore duplicate/orphan errors on our own messages
        let _ = self.dag_state.insert(hash, &parents);

        Ok(msg)
    }

    /// Serialize and transmit a QuDagMessage over radio.
    fn transmit_message(&mut self, msg: &QuDagMessage) -> Result<(), TransportError> {
        let mut wire_buf = [0u8; MAX_WIRE_FRAME];
        let wire_bytes =
            postcard::to_slice(msg, &mut wire_buf).map_err(|_| TransportError::EncodingFailed)?;
        self.radio
            .transmit_frame(wire_bytes)
            .map_err(|_| TransportError::Timeout)
    }

    /// Try to receive and validate one message from radio.
    fn try_receive_message(&mut self) -> Result<Option<QuDagMessage>, TransportError> {
        let mut buf = [0u8; MAX_WIRE_FRAME];
        let len = match self.radio.receive_frame(&mut buf) {
            Ok(0) => return Ok(None),
            Ok(n) => n,
            Err(RadioError::ReceiveFailed) => return Ok(None),
            Err(_) => return Err(TransportError::Timeout),
        };

        let msg: QuDagMessage =
            postcard::from_bytes(&buf[..len]).map_err(|_| TransportError::EncodingFailed)?;

        // Validate DAG parents
        if self.dag_state.validate_parents(&msg.dag_parents).is_err() {
            return Ok(None); // drop orphaned messages silently
        }

        // Verify signature
        let sender_pk = verdant_core::types::PublicKey {
            bytes: heapless::Vec::new(), // placeholder — in production, look up from key registry
        };
        // For now, verify the payload against the signature
        // In tests, the mock crypto will handle this
        match self.crypto.verify(
            &msg.payload.ciphertext,
            &msg.signature,
            &sender_pk,
        ) {
            Ok(true) => {}
            Ok(false) => return Ok(None), // invalid signature → drop
            Err(_) => return Ok(None),
        }

        // Insert into DAG
        let hash = msg.compute_hash();
        let _ = self.dag_state.insert(hash, &msg.dag_parents);

        Ok(Some(msg))
    }

    /// Decode a validated QuDagMessage back into a MeshFrame.
    fn decode_frame(msg: &QuDagMessage) -> Result<MeshFrame, TransportError> {
        postcard::from_bytes(&msg.payload.ciphertext).map_err(|_| TransportError::EncodingFailed)
    }
}

impl<R: RadioHardware, C: PostQuantumCrypto> MeshTransport
    for MeshCommunicationLayer<R, C>
{
    fn send(&mut self, frame: &MeshFrame) -> Result<(), TransportError> {
        let msg = self.build_message(frame)?;
        self.transmit_message(&msg)
    }

    fn receive(&mut self) -> Result<Option<MeshFrame>, TransportError> {
        let msg = match self.try_receive_message()? {
            Some(m) => m,
            None => return Ok(None),
        };

        // If TTL > 0 and frame is not for us, forward
        let frame = Self::decode_frame(&msg)?;
        if frame.source != self.self_id && msg.ttl > 0 {
            // Check if this is a relay (destination is not us)
            // For now, deliver all validated messages locally.
            // Forwarding logic is handled by the firmware orchestrator.
        }

        Ok(Some(frame))
    }

    fn broadcast(&mut self, frame: &MeshFrame, ttl: u8) -> Result<(), TransportError> {
        let mut broadcast_frame = frame.clone();
        broadcast_frame.ttl = ttl;
        let msg = self.build_message(&broadcast_frame)?;
        self.transmit_message(&msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use verdant_core::error::CryptoError;
    use verdant_core::types::{
        BleBeacon, Ciphertext, DilithiumSignature, PublicKey, SharedSecret,
    };

    mock! {
        pub Radio {}
        impl RadioHardware for Radio {
            fn transmit_frame(&mut self, raw: &[u8]) -> Result<(), RadioError>;
            fn receive_frame(&mut self, buf: &mut [u8]) -> Result<usize, RadioError>;
            fn scan_ble_beacons(&mut self) -> Result<heapless::Vec<BleBeacon, 32>, RadioError>;
        }
    }

    mock! {
        pub Crypto {}
        impl PostQuantumCrypto for Crypto {
            fn sign(&self, data: &[u8]) -> Result<DilithiumSignature, CryptoError>;
            fn verify(&self, data: &[u8], sig: &DilithiumSignature, pk: &PublicKey) -> Result<bool, CryptoError>;
            fn encapsulate(&self, pk: &PublicKey) -> Result<(SharedSecret, Ciphertext), CryptoError>;
            fn decapsulate(&self, ct: &Ciphertext) -> Result<SharedSecret, CryptoError>;
        }
    }

    fn node(id: u8) -> NodeId {
        NodeId([id; 8])
    }

    fn test_frame() -> MeshFrame {
        let mut payload = heapless::Vec::new();
        payload.extend_from_slice(b"test payload").unwrap();
        MeshFrame {
            source: node(1),
            payload,
            ttl: 5,
        }
    }

    fn test_sig() -> DilithiumSignature {
        DilithiumSignature { bytes: heapless::Vec::new() }
    }

    /// Convert DAG tips (Vec<_, 8>) to message parents (Vec<_, 3>).
    fn tips_to_parents(dag: &DagState) -> heapless::Vec<verdant_core::types::MessageHash, 3> {
        let tips = dag.current_tips(3);
        let mut parents = heapless::Vec::new();
        for t in tips.iter().take(3) {
            let _ = parents.push(*t);
        }
        parents
    }

    #[test]
    fn send_signs_message_then_transmits() {
        let mut mock_radio = MockRadio::new();
        let mut mock_crypto = MockCrypto::new();

        mock_crypto
            .expect_sign()
            .times(1)
            .returning(|_| Ok(test_sig()));

        mock_radio
            .expect_transmit_frame()
            .times(1)
            .returning(|_| Ok(()));

        let mut layer = MeshCommunicationLayer::new(mock_radio, mock_crypto, node(0));
        layer.send(&test_frame()).unwrap();
    }

    #[test]
    fn send_builds_dag_references() {
        let mut mock_radio = MockRadio::new();
        let mut mock_crypto = MockCrypto::new();

        mock_crypto
            .expect_sign()
            .returning(|_| Ok(test_sig()));

        // Capture the transmitted bytes to verify DAG parents
        mock_radio
            .expect_transmit_frame()
            .times(1)
            .withf(|raw: &[u8]| {
                // Deserialize and check that dag_parents is non-empty
                let msg: QuDagMessage = postcard::from_bytes(raw).unwrap();
                !msg.dag_parents.is_empty()
            })
            .returning(|_| Ok(()));

        let mut layer = MeshCommunicationLayer::new(mock_radio, mock_crypto, node(0));
        layer.send(&test_frame()).unwrap();
    }

    #[test]
    fn receive_rejects_invalid_signature() {
        let mut mock_radio = MockRadio::new();
        let mut mock_crypto = MockCrypto::new();

        // Build a valid wire message first
        let frame = test_frame();
        let mut payload_buf = [0u8; 4096];
        let payload_bytes = postcard::to_slice(&frame, &mut payload_buf).unwrap();

        let mut ciphertext = heapless::Vec::new();
        ciphertext.extend_from_slice(payload_bytes).unwrap();

        let dag = DagState::new();
        let msg = QuDagMessage {
            dag_parents: tips_to_parents(&dag),
            payload: EncryptedPayload {
                ciphertext,
                kem_ciphertext: None,
            },
            signature: test_sig(),
            ttl: 5,
            timestamp: 0,
        };

        let mut wire_buf = [0u8; MAX_WIRE_FRAME];
        let wire_bytes = postcard::to_slice(&msg, &mut wire_buf).unwrap();
        let wire_len = wire_bytes.len();
        let wire_copy: heapless::Vec<u8, MAX_WIRE_FRAME> =
            heapless::Vec::from_slice(wire_bytes).unwrap();

        mock_radio
            .expect_receive_frame()
            .times(1)
            .returning(move |buf: &mut [u8]| {
                buf[..wire_len].copy_from_slice(&wire_copy[..wire_len]);
                Ok(wire_len)
            });

        // Signature verification fails
        mock_crypto
            .expect_verify()
            .times(1)
            .returning(|_, _, _| Ok(false));

        // sign is not needed for receive
        mock_crypto.expect_sign().times(0);

        let mut layer = MeshCommunicationLayer::new(mock_radio, mock_crypto, node(0));
        let result = layer.receive().unwrap();
        assert!(result.is_none()); // dropped due to invalid sig
    }

    #[test]
    fn receive_delivers_locally_when_valid() {
        let mut mock_radio = MockRadio::new();
        let mut mock_crypto = MockCrypto::new();

        let frame = test_frame();
        let mut payload_buf = [0u8; 4096];
        let payload_bytes = postcard::to_slice(&frame, &mut payload_buf).unwrap();

        let mut ciphertext = heapless::Vec::new();
        ciphertext.extend_from_slice(payload_bytes).unwrap();

        let dag = DagState::new();
        let msg = QuDagMessage {
            dag_parents: tips_to_parents(&dag),
            payload: EncryptedPayload {
                ciphertext,
                kem_ciphertext: None,
            },
            signature: test_sig(),
            ttl: 5,
            timestamp: 0,
        };

        let mut wire_buf = [0u8; MAX_WIRE_FRAME];
        let wire_bytes = postcard::to_slice(&msg, &mut wire_buf).unwrap();
        let wire_len = wire_bytes.len();
        let wire_copy: heapless::Vec<u8, MAX_WIRE_FRAME> =
            heapless::Vec::from_slice(wire_bytes).unwrap();

        mock_radio
            .expect_receive_frame()
            .times(1)
            .returning(move |buf: &mut [u8]| {
                buf[..wire_len].copy_from_slice(&wire_copy[..wire_len]);
                Ok(wire_len)
            });

        mock_crypto
            .expect_verify()
            .times(1)
            .returning(|_, _, _| Ok(true));

        let mut layer = MeshCommunicationLayer::new(mock_radio, mock_crypto, node(0));
        let received = layer.receive().unwrap();
        assert!(received.is_some());
        let rx_frame = received.unwrap();
        assert_eq!(rx_frame.source, node(1));
        assert_eq!(rx_frame.payload.as_slice(), b"test payload");
    }

    #[test]
    fn receive_returns_none_when_no_data() {
        let mut mock_radio = MockRadio::new();
        let mock_crypto = MockCrypto::new();

        mock_radio
            .expect_receive_frame()
            .times(1)
            .returning(|_| Ok(0));

        let mut layer = MeshCommunicationLayer::new(mock_radio, mock_crypto, node(0));
        let result = layer.receive().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn receive_rejects_orphan_dag_parents() {
        let mut mock_radio = MockRadio::new();
        let mock_crypto = MockCrypto::new();

        // Build a message with unknown DAG parents
        let frame = test_frame();
        let mut payload_buf = [0u8; 4096];
        let payload_bytes = postcard::to_slice(&frame, &mut payload_buf).unwrap();

        let mut ciphertext = heapless::Vec::new();
        ciphertext.extend_from_slice(payload_bytes).unwrap();

        let mut orphan_parents = heapless::Vec::new();
        let _ = orphan_parents.push(verdant_core::types::MessageHash([0xFF; 32]));

        let msg = QuDagMessage {
            dag_parents: orphan_parents,
            payload: EncryptedPayload {
                ciphertext,
                kem_ciphertext: None,
            },
            signature: test_sig(),
            ttl: 5,
            timestamp: 0,
        };

        let mut wire_buf = [0u8; MAX_WIRE_FRAME];
        let wire_bytes = postcard::to_slice(&msg, &mut wire_buf).unwrap();
        let wire_len = wire_bytes.len();
        let wire_copy: heapless::Vec<u8, MAX_WIRE_FRAME> =
            heapless::Vec::from_slice(wire_bytes).unwrap();

        mock_radio
            .expect_receive_frame()
            .times(1)
            .returning(move |buf: &mut [u8]| {
                buf[..wire_len].copy_from_slice(&wire_copy[..wire_len]);
                Ok(wire_len)
            });

        // verify should never be called — we reject before signature check
        // (but the mock doesn't enforce this explicitly since the method isn't expected)

        let mut layer = MeshCommunicationLayer::new(mock_radio, mock_crypto, node(0));
        let result = layer.receive().unwrap();
        assert!(result.is_none()); // dropped due to orphan parents
    }

    #[test]
    fn broadcast_sets_ttl() {
        let mut mock_radio = MockRadio::new();
        let mut mock_crypto = MockCrypto::new();

        mock_crypto
            .expect_sign()
            .returning(|_| Ok(test_sig()));

        mock_radio
            .expect_transmit_frame()
            .times(1)
            .withf(|raw: &[u8]| {
                let msg: QuDagMessage = postcard::from_bytes(raw).unwrap();
                msg.ttl == 16
            })
            .returning(|_| Ok(()));

        let mut layer = MeshCommunicationLayer::new(mock_radio, mock_crypto, node(0));
        layer.broadcast(&test_frame(), 16).unwrap();
    }
}
