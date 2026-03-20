use verdant_core::error::TransportError;
use verdant_core::traits::MeshTransport;
use verdant_core::types::NodeId;

use crate::mission::RobotState;

/// Handles mesh relay behavior for a robot in transit.
///
/// While navigating or executing, the robot acts as a mobile mesh relay
/// node: it receives mesh frames and forwards them to extend the mesh's
/// coverage into otherwise unreachable areas.
pub struct MobileRelay {
    self_id: NodeId,
}

impl MobileRelay {
    pub fn new(self_id: NodeId) -> Self {
        Self { self_id }
    }

    /// Process one relay cycle: drain incoming messages and forward
    /// those not addressed to us.
    ///
    /// Only relays when the robot is in a mobile state (Navigating, Executing,
    /// Returning). Idle or aborted robots don't relay.
    pub fn relay_cycle(
        &self,
        state: RobotState,
        transport: &mut impl MeshTransport,
    ) -> Result<u32, TransportError> {
        match state {
            RobotState::Navigating | RobotState::Executing | RobotState::Returning => {}
            _ => return Ok(0), // not in a relay-capable state
        }

        let mut forwarded = 0u32;
        while let Some(frame) = transport.receive()? {
            if frame.source == self.self_id {
                continue; // don't forward our own messages
            }
            if frame.ttl == 0 {
                continue; // TTL expired
            }

            let mut fwd = frame.clone();
            fwd.ttl = fwd.ttl.saturating_sub(1);
            transport.send(&fwd)?;
            forwarded += 1;
        }

        Ok(forwarded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use verdant_core::types::MeshFrame;

    mock! {
        pub Transport {}
        impl MeshTransport for Transport {
            fn send(&mut self, frame: &MeshFrame) -> Result<(), TransportError>;
            fn receive(&mut self) -> Result<Option<MeshFrame>, TransportError>;
            fn broadcast(&mut self, frame: &MeshFrame, ttl: u8) -> Result<(), TransportError>;
        }
    }

    fn test_frame(source_id: u8, ttl: u8) -> MeshFrame {
        let mut payload = heapless::Vec::new();
        let _ = payload.extend_from_slice(b"relay data");
        MeshFrame {
            source: NodeId([source_id; 8]),
            payload,
            ttl,
        }
    }

    #[test]
    fn forwards_mesh_messages_while_navigating() {
        let relay = MobileRelay::new(NodeId([1; 8]));
        let mut transport = MockTransport::new();

        // One incoming message from another node
        let _incoming = test_frame(2, 5);
        let mut call = 0u32;
        transport.expect_receive().returning(move || {
            call += 1;
            if call == 1 {
                Ok(Some(test_frame(2, 5)))
            } else {
                Ok(None)
            }
        });

        // Verify send is called with decremented TTL
        transport.expect_send()
            .withf(|frame| frame.ttl == 4 && frame.source == NodeId([2; 8]))
            .times(1)
            .returning(|_| Ok(()));

        let count = relay.relay_cycle(RobotState::Navigating, &mut transport).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn does_not_relay_when_idle() {
        let relay = MobileRelay::new(NodeId([1; 8]));
        let mut transport = MockTransport::new();

        // Should not even try to receive
        transport.expect_receive().times(0);
        transport.expect_send().times(0);

        let count = relay.relay_cycle(RobotState::Idle, &mut transport).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn does_not_relay_when_aborted() {
        let relay = MobileRelay::new(NodeId([1; 8]));
        let mut transport = MockTransport::new();

        transport.expect_receive().times(0);
        transport.expect_send().times(0);

        let count = relay.relay_cycle(RobotState::SafetyAbort, &mut transport).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn does_not_forward_own_messages() {
        let relay = MobileRelay::new(NodeId([1; 8]));
        let mut transport = MockTransport::new();

        // Message from ourselves
        let mut call = 0u32;
        transport.expect_receive().returning(move || {
            call += 1;
            if call == 1 {
                Ok(Some(test_frame(1, 5))) // from self
            } else {
                Ok(None)
            }
        });

        transport.expect_send().times(0);

        let count = relay.relay_cycle(RobotState::Navigating, &mut transport).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn does_not_forward_zero_ttl() {
        let relay = MobileRelay::new(NodeId([1; 8]));
        let mut transport = MockTransport::new();

        let mut call = 0u32;
        transport.expect_receive().returning(move || {
            call += 1;
            if call == 1 {
                Ok(Some(test_frame(2, 0))) // TTL expired
            } else {
                Ok(None)
            }
        });

        transport.expect_send().times(0);

        let count = relay.relay_cycle(RobotState::Navigating, &mut transport).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn relays_during_executing_state() {
        let relay = MobileRelay::new(NodeId([1; 8]));
        let mut transport = MockTransport::new();

        let mut call = 0u32;
        transport.expect_receive().returning(move || {
            call += 1;
            if call == 1 {
                Ok(Some(test_frame(3, 10)))
            } else {
                Ok(None)
            }
        });

        transport.expect_send()
            .times(1)
            .returning(|_| Ok(()));

        let count = relay.relay_cycle(RobotState::Executing, &mut transport).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn relays_during_returning_state() {
        let relay = MobileRelay::new(NodeId([1; 8]));
        let mut transport = MockTransport::new();

        let mut call = 0u32;
        transport.expect_receive().returning(move || {
            call += 1;
            if call == 1 {
                Ok(Some(test_frame(4, 3)))
            } else {
                Ok(None)
            }
        });

        transport.expect_send()
            .times(1)
            .returning(|_| Ok(()));

        let count = relay.relay_cycle(RobotState::Returning, &mut transport).unwrap();
        assert_eq!(count, 1);
    }
}
