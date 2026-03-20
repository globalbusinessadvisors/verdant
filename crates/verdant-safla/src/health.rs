use verdant_core::traits::NetworkHealer;
use verdant_core::types::{NodeId, Timestamp};
use verdant_mesh::routing::{Neighbor, NeighborStatus};

/// Result of a health assessment cycle.
pub struct HealthReport {
    pub newly_suspect: heapless::Vec<NodeId, 16>,
    pub newly_dead: heapless::Vec<NodeId, 16>,
    pub reroute_results: heapless::Vec<(NodeId, bool), 16>,
}

/// Assesses neighbor health by checking heartbeat timeouts and
/// consecutive missed contacts.
pub struct HealthAssessor {
    /// Duration (in milliseconds) after which a silent neighbor becomes suspect.
    timeout_threshold_ms: u64,
    /// Number of consecutive misses (status transitions) before declaring dead.
    max_consecutive_misses: u8,
}

impl HealthAssessor {
    pub fn new(timeout_threshold_secs: u64, max_consecutive_misses: u8) -> Self {
        Self {
            timeout_threshold_ms: timeout_threshold_secs * 1000,
            max_consecutive_misses,
        }
    }

    /// Evaluate all neighbors and return a health report.
    ///
    /// - Neighbors whose `last_seen` exceeds the timeout become **suspect**.
    /// - Suspect neighbors that have been suspect for `max_consecutive_misses`
    ///   cycles (approximated by checking timeout multiples) become **dead**,
    ///   and `healer.reroute_around()` is called.
    /// - Active neighbors with recent `last_seen` are left alone.
    pub fn assess(
        &self,
        neighbors: &[Neighbor],
        now: Timestamp,
        healer: &mut impl NetworkHealer,
    ) -> HealthReport {
        let mut report = HealthReport {
            newly_suspect: heapless::Vec::new(),
            newly_dead: heapless::Vec::new(),
            reroute_results: heapless::Vec::new(),
        };

        for neighbor in neighbors {
            if neighbor.status == NeighborStatus::Dead {
                continue; // already handled
            }

            let elapsed = now.saturating_diff(&neighbor.last_seen);

            if elapsed <= self.timeout_threshold_ms {
                continue; // healthy
            }

            // Determine severity by how many timeout periods have passed
            let misses = (elapsed / self.timeout_threshold_ms) as u8;

            if misses > self.max_consecutive_misses {
                // Dead — reroute
                let _ = report.newly_dead.push(neighbor.id);
                let success = healer.reroute_around(neighbor.id).is_ok();
                let _ = report.reroute_results.push((neighbor.id, success));
            } else {
                // Suspect
                let _ = report.newly_suspect.push(neighbor.id);
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use verdant_core::error::HealError;
    use verdant_core::types::TopologyChange;
    use verdant_mesh::routing::LinkQuality;

    mock! {
        pub Healer {}
        impl NetworkHealer for Healer {
            fn reroute_around(&mut self, dead: NodeId) -> Result<(), HealError>;
            fn propose_topology_change(&mut self, change: &TopologyChange) -> Result<(), HealError>;
        }
    }

    fn node(id: u8) -> NodeId {
        NodeId([id; 8])
    }

    fn make_neighbor(id: u8, last_seen_ms: u64, status: NeighborStatus) -> Neighbor {
        Neighbor {
            id: node(id),
            link_quality: LinkQuality::from_measurements(0.9, 50),
            last_seen: Timestamp(last_seen_ms),
            status,
        }
    }

    #[test]
    fn marks_dead_and_reroutes_after_threshold() {
        let assessor = HealthAssessor::new(60, 3); // 60s timeout, 3 misses
        let now = Timestamp::from_secs(600);
        // Neighbor last seen at t=0 → 600s ago → 10 misses (> 3)
        let neighbors = [make_neighbor(1, 0, NeighborStatus::Active)];

        let mut healer = MockHealer::new();
        healer
            .expect_reroute_around()
            .with(mockall::predicate::eq(node(1)))
            .times(1)
            .returning(|_| Ok(()));

        let report = assessor.assess(&neighbors, now, &mut healer);
        assert!(report.newly_dead.contains(&node(1)));
        assert!(report.newly_suspect.is_empty());
        assert_eq!(report.reroute_results[0], (node(1), true));
    }

    #[test]
    fn marks_suspect_but_not_dead_at_threshold() {
        let assessor = HealthAssessor::new(60, 3);
        let now = Timestamp::from_secs(200);
        // Neighbor last seen at t=80s → 120s ago → 2 misses (≤ 3)
        let neighbors = [make_neighbor(1, 80_000, NeighborStatus::Active)];

        let mut healer = MockHealer::new();
        healer.expect_reroute_around().times(0);

        let report = assessor.assess(&neighbors, now, &mut healer);
        assert!(report.newly_suspect.contains(&node(1)));
        assert!(report.newly_dead.is_empty());
    }

    #[test]
    fn ignores_healthy_neighbors() {
        let assessor = HealthAssessor::new(60, 3);
        let now = Timestamp::from_secs(100);
        // Neighbor last seen at t=95s → 5s ago → healthy
        let neighbors = [make_neighbor(1, 95_000, NeighborStatus::Active)];

        let mut healer = MockHealer::new();
        healer.expect_reroute_around().times(0);

        let report = assessor.assess(&neighbors, now, &mut healer);
        assert!(report.newly_suspect.is_empty());
        assert!(report.newly_dead.is_empty());
    }

    #[test]
    fn ignores_already_dead_neighbors() {
        let assessor = HealthAssessor::new(60, 3);
        let now = Timestamp::from_secs(600);
        let neighbors = [make_neighbor(1, 0, NeighborStatus::Dead)];

        let mut healer = MockHealer::new();
        healer.expect_reroute_around().times(0);

        let report = assessor.assess(&neighbors, now, &mut healer);
        assert!(report.newly_dead.is_empty());
    }

    #[test]
    fn handles_reroute_failure() {
        let assessor = HealthAssessor::new(60, 3);
        let now = Timestamp::from_secs(600);
        let neighbors = [make_neighbor(1, 0, NeighborStatus::Active)];

        let mut healer = MockHealer::new();
        healer
            .expect_reroute_around()
            .returning(|_| Err(HealError::NoAlternativePath));

        let report = assessor.assess(&neighbors, now, &mut healer);
        assert!(report.newly_dead.contains(&node(1)));
        assert_eq!(report.reroute_results[0], (node(1), false));
    }

    #[test]
    fn multiple_neighbors_assessed() {
        let assessor = HealthAssessor::new(60, 3);
        let now = Timestamp::from_secs(600);
        let neighbors = [
            make_neighbor(1, 0, NeighborStatus::Active),       // dead (10 misses)
            make_neighbor(2, 500_000, NeighborStatus::Active),  // suspect (1-2 misses)
            make_neighbor(3, 598_000, NeighborStatus::Active),  // healthy
        ];

        let mut healer = MockHealer::new();
        healer
            .expect_reroute_around()
            .with(mockall::predicate::eq(node(1)))
            .times(1)
            .returning(|_| Ok(()));

        let report = assessor.assess(&neighbors, now, &mut healer);
        assert!(report.newly_dead.contains(&node(1)));
        assert!(report.newly_suspect.contains(&node(2)));
        assert!(!report.newly_suspect.contains(&node(3)));
    }
}
