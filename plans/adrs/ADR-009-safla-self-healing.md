# ADR-009: SAFLA Self-Aware Feedback Loop Architecture

**Status:** Accepted
**Date:** 2026-03-20
**Deciders:** Verdant Core Team
**SPARC Refs:** FR-N6, FR-I3, FR-I4, FR-I6, G4, G7, G8

## Context

The Verdant mesh must be autonomously self-managing. When a node dies, routes must reroute. When multiple nodes detect correlated anomalies, the mesh must reach consensus without human intervention. When a new threat pattern is learned, it must propagate across the mesh. SAFLA (Self-Aware Feedback Loop Algorithm) is the subsystem that orchestrates all of this.

Candidate approaches:
1. **Centralized coordinator** — simple but violates decentralization; single point of failure
2. **Raft/Paxos consensus** — strong consistency but requires majority availability; too heavy for constrained nodes
3. **Gossip-based eventual consistency** — lightweight, partition-tolerant, but no formal consensus
4. **SAFLA: gossip + local quorum consensus** — gossip for dissemination, local quorum for event confirmation

## Decision

Implement SAFLA as a **four-phase feedback loop** running on every node: Health Assessment, Anomaly Consensus, Pattern Propagation, and Topology Optimization. Consensus uses **local spatial-temporal quorum** (not global consensus).

## Rationale

- **Local quorum, not global consensus.** Verdant doesn't need all nodes to agree. It needs *nearby* nodes to agree. If 3 out of 5 nodes in Zone A detect a pest anomaly within 5 minutes, that's a confirmed event — regardless of what nodes in Zone D think.
- **Spatial-temporal correlation.** Two anomaly reports are correlated if they are within a configurable zone radius *and* within a temporal window (default: 300 seconds). This naturally filters false positives (a single node's logging truck detection) from true events (multiple nodes detecting pest damage).
- **Pattern propagation via epidemic gossip.** When a node learns a new threat signature (e.g., emerald ash borer RF pattern), it computes a compressed vector graph delta and gossips it with a TTL of 16 hops. Each relay node merges the delta into its own graph. This is fire-and-forget — no acknowledgment required.
- **Topology optimization.** Nodes periodically evaluate whether their routing table is efficient and propose changes (e.g., "I should relay through node X instead of node Y because my link quality to X has improved"). This is advisory, not mandatory.

## Consequences

- **Positive:** Fully autonomous; partition-tolerant; lightweight; no global consensus overhead
- **Negative:** Eventual consistency means brief windows of disagreement; consensus quorum tuning is site-specific
- **Mitigated by:** Conservative defaults (quorum = 3, window = 300s); tuning guide in pilot documentation

## London School TDD Approach

```rust
pub trait NeighborAnomalyCollector {
    fn collect_recent(&self, window_secs: u64) -> Vec<NeighborAnomaly>;
}

pub trait EventPropagator {
    fn propagate_confirmed(&mut self, event: &ConfirmedEvent) -> Result<(), PropagationError>;
    fn propagate_pattern_delta(&mut self, delta: &CompressedDelta, ttl: u8) -> Result<(), PropagationError>;
}

pub trait ActionDeterminer {
    fn determine_action(&self, category: EventCategory) -> RecommendedAction;
}

pub trait HealthMonitor {
    fn mark_suspect(&mut self, neighbor: NodeId);
    fn mark_dead(&mut self, neighbor: NodeId);
    fn reroute_around(&mut self, dead_node: NodeId) -> Result<(), RoutingError>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn consensus_confirms_event_when_quorum_reached() {
        let mut mock_collector = MockNeighborAnomalyCollector::new();
        let mut mock_propagator = MockEventPropagator::new();
        let mut mock_determiner = MockActionDeterminer::new();

        // GIVEN: local anomaly detected (pest, score 0.92)
        let local_anomaly = Anomaly {
            category: EventCategory::Pest,
            score: 0.92,
            location: zone_a_center(),
            timestamp: now(),
        };

        // AND: 3 neighbors also detected pest anomalies recently
        mock_collector.expect_collect_recent()
            .with(eq(300))
            .returning(|_| vec![
                neighbor_anomaly(EventCategory::Pest, zone_a_north(), now_minus(60)),
                neighbor_anomaly(EventCategory::Pest, zone_a_south(), now_minus(120)),
                neighbor_anomaly(EventCategory::Pest, zone_a_east(), now_minus(30)),
            ]);

        // THEN: action determiner is consulted
        mock_determiner.expect_determine_action()
            .with(eq(EventCategory::Pest))
            .returning(|_| RecommendedAction::AlertAndMonitor);

        // AND: confirmed event is propagated
        mock_propagator.expect_propagate_confirmed()
            .withf(|e| {
                e.category == EventCategory::Pest
                && e.confidence > 0.9
                && matches!(e.recommended_action, RecommendedAction::AlertAndMonitor)
            })
            .times(1)
            .returning(|_| Ok(()));

        let safla = SaflaCycle::new(
            mock_collector, mock_propagator, mock_determiner, mock_health
        );
        safla.run_anomaly_consensus(vec![local_anomaly]);
    }

    #[test]
    fn consensus_does_not_confirm_when_below_quorum() {
        let mut mock_collector = MockNeighborAnomalyCollector::new();
        let mut mock_propagator = MockEventPropagator::new();

        // GIVEN: local anomaly but only 1 neighbor corroborates (below quorum of 3)
        mock_collector.expect_collect_recent()
            .returning(|_| vec![
                neighbor_anomaly(EventCategory::Pest, zone_a_north(), now_minus(60)),
            ]);

        // THEN: propagator is never called
        mock_propagator.expect_propagate_confirmed().times(0);

        let safla = SaflaCycle::new(
            mock_collector, mock_propagator, mock_determiner, mock_health
        );
        safla.run_anomaly_consensus(vec![local_anomaly()]);
    }

    #[test]
    fn consensus_filters_out_temporally_distant_corroborations() {
        let mut mock_collector = MockNeighborAnomalyCollector::new();
        let mut mock_propagator = MockEventPropagator::new();

        // GIVEN: 3 neighbor anomalies, but 2 are outside the 300s window
        mock_collector.expect_collect_recent()
            .returning(|_| vec![
                neighbor_anomaly(EventCategory::Pest, zone_a_north(), now_minus(60)),   // in window
                neighbor_anomaly(EventCategory::Pest, zone_a_south(), now_minus(400)),  // too old
                neighbor_anomaly(EventCategory::Pest, zone_a_east(), now_minus(500)),   // too old
            ]);

        // THEN: no confirmed event (only 1 corroboration in window, below quorum)
        mock_propagator.expect_propagate_confirmed().times(0);

        let safla = SaflaCycle::new(
            mock_collector, mock_propagator, mock_determiner, mock_health
        );
        safla.run_anomaly_consensus(vec![local_anomaly()]);
    }

    #[test]
    fn health_check_reroutes_around_dead_neighbor() {
        let mut mock_health = MockHealthMonitor::new();

        // GIVEN: neighbor has missed 4 consecutive heartbeats
        let neighbor = NeighborState {
            id: node_b,
            last_seen: now_minus(600),
            consecutive_misses: 4,
        };

        // THEN: node is marked dead
        mock_health.expect_mark_dead()
            .with(eq(node_b))
            .times(1)
            .returning(|_| ());

        // AND: routes are recalculated around it
        mock_health.expect_reroute_around()
            .with(eq(node_b))
            .times(1)
            .returning(|_| Ok(()));

        let safla = SaflaCycle::new(
            mock_collector, mock_propagator, mock_determiner, mock_health
        );
        safla.run_health_assessment(vec![neighbor]);
    }

    #[test]
    fn pattern_propagation_broadcasts_significant_delta() {
        let mut mock_propagator = MockEventPropagator::new();

        // GIVEN: vector graph has new patterns since last propagation
        let delta = CompressedDelta {
            added: vec![new_pattern_node()],
            removed: vec![],
            version: 42,
            compressed_size: 256,
        };

        // THEN: delta is propagated with TTL=16
        mock_propagator.expect_propagate_pattern_delta()
            .with(always(), eq(16))
            .times(1)
            .returning(|_, _| Ok(()));

        let safla = SaflaCycle::new(
            mock_collector, mock_propagator, mock_determiner, mock_health
        );
        safla.run_pattern_propagation(delta);
    }
}
```
