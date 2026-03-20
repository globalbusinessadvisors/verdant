use verdant_core::config::PATTERN_PROPAGATION_TTL;
use verdant_core::error::TransportError;
use verdant_core::traits::MeshTransport;
use verdant_core::types::{MeshFrame, NodeId, Version};
use verdant_vector::graph::VectorGraph;

/// Propagation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropagationError {
    TransportFailed(TransportError),
    SerializationFailed,
}

impl From<TransportError> for PropagationError {
    fn from(e: TransportError) -> Self {
        Self::TransportFailed(e)
    }
}

/// Manages propagation of vector graph pattern deltas across the mesh.
///
/// Tracks the last propagated version and only broadcasts when new
/// patterns have been learned since the last propagation.
pub struct PatternPropagator {
    last_propagated_version: Version,
    self_id: NodeId,
}

impl PatternPropagator {
    pub fn new(self_id: NodeId) -> Self {
        Self {
            last_propagated_version: Version::new(0),
            self_id,
        }
    }

    /// Check if the graph has new patterns since last propagation, and
    /// if so, broadcast the compressed delta to the mesh.
    ///
    /// Returns `Ok(true)` if a delta was propagated, `Ok(false)` if not.
    pub fn check_and_propagate(
        &mut self,
        graph: &VectorGraph,
        transport: &mut impl MeshTransport,
    ) -> Result<bool, PropagationError> {
        let current_version = graph.version();
        if current_version <= self.last_propagated_version {
            return Ok(false);
        }

        let delta = graph.compute_delta(self.last_propagated_version);
        if delta.is_empty() {
            self.last_propagated_version = current_version;
            return Ok(false);
        }

        // Serialize the delta into a mesh frame payload
        let mut payload_buf = [0u8; 4096];
        let payload_bytes = postcard::to_slice(&delta, &mut payload_buf)
            .map_err(|_| PropagationError::SerializationFailed)?;

        let mut payload = heapless::Vec::new();
        payload
            .extend_from_slice(payload_bytes)
            .map_err(|_| PropagationError::SerializationFailed)?;

        let frame = MeshFrame {
            source: self.self_id,
            payload,
            ttl: PATTERN_PROPAGATION_TTL,
        };

        transport.broadcast(&frame, PATTERN_PROPAGATION_TTL)?;
        self.last_propagated_version = current_version;
        Ok(true)
    }

    /// Get the last propagated version (for testing/diagnostics).
    pub fn last_propagated_version(&self) -> Version {
        self.last_propagated_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use verdant_core::config::EMBEDDING_DIM;
    use verdant_core::traits::SeasonalClock;
    use verdant_core::types::{Embedding, SeasonSlot};

    mock! {
        pub Transport {}
        impl MeshTransport for Transport {
            fn send(&mut self, frame: &MeshFrame) -> Result<(), TransportError>;
            fn receive(&mut self) -> Result<Option<MeshFrame>, TransportError>;
            fn broadcast(&mut self, frame: &MeshFrame, ttl: u8) -> Result<(), TransportError>;
        }
    }

    struct FixedClock;
    impl SeasonalClock for FixedClock {
        fn current_slot(&self) -> SeasonSlot {
            SeasonSlot::new(10)
        }
    }

    fn emb(val: i16) -> Embedding {
        let mut data = [0i16; EMBEDDING_DIM];
        data[0] = val;
        Embedding { data }
    }

    fn node(id: u8) -> NodeId {
        NodeId([id; 8])
    }

    #[test]
    fn propagates_when_new_patterns_exist() {
        let mut graph = VectorGraph::new(2048);
        let clock = FixedClock;

        // Insert a node so the graph has version > 0
        graph.update(emb(1000), &clock);

        let mut transport = MockTransport::new();
        transport
            .expect_broadcast()
            .withf(|_, ttl| *ttl == PATTERN_PROPAGATION_TTL)
            .times(1)
            .returning(|_, _| Ok(()));

        let mut propagator = PatternPropagator::new(node(1));
        let result = propagator.check_and_propagate(&graph, &mut transport).unwrap();
        assert!(result);
    }

    #[test]
    fn does_not_propagate_when_no_changes() {
        let graph = VectorGraph::new(2048);
        // Empty graph, version=0

        let mut transport = MockTransport::new();
        transport.expect_broadcast().times(0);

        let mut propagator = PatternPropagator::new(node(1));
        let result = propagator.check_and_propagate(&graph, &mut transport).unwrap();
        assert!(!result);
    }

    #[test]
    fn does_not_propagate_twice_for_same_version() {
        let mut graph = VectorGraph::new(2048);
        let clock = FixedClock;
        graph.update(emb(1000), &clock);

        let mut transport = MockTransport::new();
        transport
            .expect_broadcast()
            .times(1)
            .returning(|_, _| Ok(()));

        let mut propagator = PatternPropagator::new(node(1));
        propagator.check_and_propagate(&graph, &mut transport).unwrap();

        // Second call — no new changes
        let result = propagator.check_and_propagate(&graph, &mut transport).unwrap();
        assert!(!result);
    }

    #[test]
    fn propagates_again_after_new_update() {
        let mut graph = VectorGraph::new(2048);
        let clock = FixedClock;
        graph.update(emb(1000), &clock);

        let mut transport = MockTransport::new();
        transport
            .expect_broadcast()
            .times(2)
            .returning(|_, _| Ok(()));

        let mut propagator = PatternPropagator::new(node(1));
        propagator.check_and_propagate(&graph, &mut transport).unwrap();

        // New update
        let mut e2 = [0i16; EMBEDDING_DIM];
        e2[1] = 1000;
        graph.update(Embedding { data: e2 }, &clock);

        let result = propagator.check_and_propagate(&graph, &mut transport).unwrap();
        assert!(result);
    }
}
