use verdant_core::types::{ConfirmedEvent, MeshFrame};

use crate::db::{DbError, EventStore};

/// Bridge errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeError {
    MeshUnavailable,
    DeserializationFailed,
    StoreFailed(DbError),
}

/// Events received from the mesh.
#[derive(Clone, Debug)]
pub enum MeshEvent {
    Confirmed(ConfirmedEvent),
    NodeHeartbeat { raw: Vec<u8> },
}

/// Bridges the mesh radio to the gateway's local network and database.
#[cfg_attr(test, mockall::automock)]
pub trait MeshBridge: Send {
    fn poll_mesh_events(&mut self) -> Result<Vec<MeshEvent>, BridgeError>;
    fn send_to_mesh(&mut self, frame: MeshFrame) -> Result<(), BridgeError>;
}

/// Process one bridge cycle: poll mesh events and store confirmed events.
pub fn bridge_cycle(
    bridge: &mut impl MeshBridge,
    event_store: &impl EventStore,
) -> Result<u32, BridgeError> {
    let events = bridge.poll_mesh_events()?;
    let mut stored = 0u32;

    for event in &events {
        if let MeshEvent::Confirmed(confirmed) = event {
            event_store
                .store_event(confirmed)
                .map_err(BridgeError::StoreFailed)?;
            stored += 1;
        }
    }

    Ok(stored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::MockEventStore;
    use verdant_core::types::{EventCategory, PestSpecies, RecommendedAction, Timestamp, ZoneId};

    fn test_confirmed() -> ConfirmedEvent {
        ConfirmedEvent {
            event_id: 42,
            category: EventCategory::Pest {
                species_hint: Some(PestSpecies::EmeraldAshBorer),
            },
            confidence: 0.95,
            affected_zone: ZoneId([1, 0, 0, 0]),
            corroborating_count: 4,
            recommended_action: RecommendedAction::AlertAndMonitor,
            timestamp: Timestamp::from_secs(1000),
        }
    }

    #[test]
    fn bridge_stores_incoming_events() {
        let mut bridge = MockMeshBridge::new();
        let mut store = MockEventStore::new();

        bridge.expect_poll_mesh_events().returning(|| {
            Ok(vec![MeshEvent::Confirmed(test_confirmed())])
        });

        store
            .expect_store_event()
            .withf(|e| e.event_id == 42)
            .times(1)
            .returning(|_| Ok(1));

        let count = bridge_cycle(&mut bridge, &store).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn bridge_skips_non_event_messages() {
        let mut bridge = MockMeshBridge::new();
        let store = MockEventStore::new();

        bridge.expect_poll_mesh_events().returning(|| {
            Ok(vec![MeshEvent::NodeHeartbeat {
                raw: vec![1, 2, 3],
            }])
        });

        // store_event should NOT be called
        let count = bridge_cycle(&mut bridge, &store).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn bridge_handles_empty_poll() {
        let mut bridge = MockMeshBridge::new();
        let store = MockEventStore::new();

        bridge.expect_poll_mesh_events().returning(|| Ok(vec![]));

        let count = bridge_cycle(&mut bridge, &store).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn bridge_propagates_mesh_error() {
        let mut bridge = MockMeshBridge::new();
        let store = MockEventStore::new();

        bridge
            .expect_poll_mesh_events()
            .returning(|| Err(BridgeError::MeshUnavailable));

        let result = bridge_cycle(&mut bridge, &store);
        assert!(result.is_err());
    }
}
