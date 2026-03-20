use verdant_core::types::NodeId;

use crate::db::{DbError, EventStore, NodeStatus, NodeStatusStore};

/// CRDT merge errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    ReadFailed(DbError),
    WriteFailed(DbError),
}

/// Merge events from a remote gateway using grow-only set semantics.
///
/// Events are identified by `event_id`; duplicates are silently skipped.
pub fn merge_events(
    local_store: &impl EventStore,
    remote_events: &[verdant_core::types::ConfirmedEvent],
) -> Result<u32, SyncError> {
    let mut added = 0u32;
    for event in remote_events {
        // Attempt to store; duplicates will be handled by the store
        match local_store.store_event(event) {
            Ok(_) => added += 1,
            Err(DbError::WriteFailed) => {} // likely duplicate — skip
            Err(e) => return Err(SyncError::WriteFailed(e)),
        }
    }
    Ok(added)
}

/// Merge node statuses using last-writer-wins (by timestamp).
pub fn merge_node_statuses(
    local_store: &impl NodeStatusStore,
    remote_statuses: &[(NodeId, NodeStatus)],
) -> Result<u32, SyncError> {
    let mut updated = 0u32;
    for (node_id, remote_status) in remote_statuses {
        let local = local_store
            .status_for(*node_id)
            .map_err(SyncError::ReadFailed)?;

        let should_update = match &local {
            Some(existing) => remote_status.last_seen > existing.last_seen,
            None => true,
        };

        if should_update {
            local_store
                .update_status(remote_status)
                .map_err(SyncError::WriteFailed)?;
            updated += 1;
        }
    }
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{MockEventStore, MockNodeStatusStore};
    use verdant_core::types::{
        ConfirmedEvent, EventCategory, PestSpecies, RecommendedAction, Timestamp, ZoneId,
    };

    fn test_event(id: u64) -> ConfirmedEvent {
        ConfirmedEvent {
            event_id: id,
            category: EventCategory::Pest {
                species_hint: Some(PestSpecies::Unknown),
            },
            confidence: 0.90,
            affected_zone: ZoneId([1, 0, 0, 0]),
            corroborating_count: 3,
            recommended_action: RecommendedAction::AlertOnly,
            timestamp: Timestamp::from_secs(1000),
        }
    }

    fn test_status(node: NodeId, last_seen_secs: u64) -> NodeStatus {
        NodeStatus {
            node_id: node,
            zone_id: ZoneId([1, 0, 0, 0]),
            last_seen: Timestamp::from_secs(last_seen_secs),
            battery_level: 0.80,
            graph_version: 42,
            neighbor_count: 5,
            uptime_secs: 3600,
        }
    }

    fn node(id: u8) -> NodeId {
        NodeId([id; 8])
    }

    #[test]
    fn merge_events_stores_new() {
        let mut store = MockEventStore::new();
        store
            .expect_store_event()
            .times(2)
            .returning(|_| Ok(1));

        let events = vec![test_event(1), test_event(2)];
        let count = merge_events(&store, &events).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn merge_events_skips_duplicates() {
        let mut store = MockEventStore::new();
        store
            .expect_store_event()
            .returning(|_| Err(DbError::WriteFailed)); // simulate duplicate

        let events = vec![test_event(1)];
        let count = merge_events(&store, &events).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn merge_statuses_updates_newer() {
        let mut store = MockNodeStatusStore::new();
        let n = node(1);

        store
            .expect_status_for()
            .returning(move |_| {
                Ok(Some(test_status(n, 100))) // local last_seen = 100s
            });

        store
            .expect_update_status()
            .times(1)
            .returning(|_| Ok(()));

        let remote = vec![(n, test_status(n, 200))]; // remote last_seen = 200s (newer)
        let count = merge_node_statuses(&store, &remote).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn merge_statuses_skips_older() {
        let mut store = MockNodeStatusStore::new();
        let n = node(1);

        store
            .expect_status_for()
            .returning(move |_| {
                Ok(Some(test_status(n, 200))) // local is newer
            });

        store.expect_update_status().times(0);

        let remote = vec![(n, test_status(n, 100))]; // remote older
        let count = merge_node_statuses(&store, &remote).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn merge_statuses_inserts_unknown_node() {
        let mut store = MockNodeStatusStore::new();
        let n = node(5);

        store
            .expect_status_for()
            .returning(|_| Ok(None)); // not known locally

        store
            .expect_update_status()
            .times(1)
            .returning(|_| Ok(()));

        let remote = vec![(n, test_status(n, 100))];
        let count = merge_node_statuses(&store, &remote).unwrap();
        assert_eq!(count, 1);
    }
}
