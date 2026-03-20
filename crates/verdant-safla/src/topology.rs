use verdant_core::error::HealError;
use verdant_core::traits::NetworkHealer;
use verdant_core::types::{Timestamp, TopologyAction, TopologyChange};
use verdant_mesh::routing::RoutingTable;

/// Periodically proposes topology optimizations for degraded links.
pub struct TopologyOptimizer {
    rebalance_interval_ms: u64,
    last_rebalance: Timestamp,
}

impl TopologyOptimizer {
    pub fn new(rebalance_interval_secs: u64) -> Self {
        Self {
            rebalance_interval_ms: rebalance_interval_secs * 1000,
            last_rebalance: Timestamp(0),
        }
    }

    /// Whether enough time has elapsed for a rebalance check.
    pub fn should_rebalance(&self, now: Timestamp) -> bool {
        now.saturating_diff(&self.last_rebalance) >= self.rebalance_interval_ms
    }

    /// Propose topology changes for neighbors with degraded links.
    ///
    /// Returns the number of proposals submitted. Resets the rebalance timer.
    pub fn propose_changes(
        &mut self,
        routing_table: &RoutingTable,
        now: Timestamp,
        healer: &mut impl NetworkHealer,
    ) -> Result<u8, HealError> {
        if !self.should_rebalance(now) {
            return Ok(0);
        }
        self.last_rebalance = now;

        if !routing_table.should_rebalance() {
            return Ok(0);
        }

        let mut count = 0u8;

        // For each neighbor with poor link quality, propose dropping
        // and re-routing through a better path
        for neighbor in routing_table.active_neighbors() {
            if neighbor.link_quality.pdr < 0.5 {
                let change = TopologyChange {
                    action: TopologyAction::DropLink {
                        neighbor: neighbor.id,
                    },
                    affected_node: routing_table.self_id(),
                };
                healer.propose_topology_change(&change)?;
                count = count.saturating_add(1);
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use verdant_core::error::HealError;
    use verdant_core::types::NodeId;

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

    #[test]
    fn should_rebalance_after_interval() {
        let opt = TopologyOptimizer::new(300);
        assert!(opt.should_rebalance(Timestamp::from_secs(300)));
        assert!(opt.should_rebalance(Timestamp::from_secs(500)));
    }

    #[test]
    fn should_not_rebalance_before_interval() {
        let mut opt = TopologyOptimizer::new(300);
        opt.last_rebalance = Timestamp::from_secs(100);
        assert!(!opt.should_rebalance(Timestamp::from_secs(200)));
    }

    #[test]
    fn propose_changes_for_poor_links() {
        let mut rt = RoutingTable::new(node(0));
        rt.update_link_quality(node(1), 0.30, 500); // poor
        rt.update_link_quality(node(2), 0.95, 50);  // good

        let mut healer = MockHealer::new();
        healer
            .expect_propose_topology_change()
            .times(1) // only the poor link
            .returning(|_| Ok(()));

        let mut opt = TopologyOptimizer::new(300);
        let count = opt
            .propose_changes(&rt, Timestamp::from_secs(500), &mut healer)
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn no_proposals_when_all_links_good() {
        let mut rt = RoutingTable::new(node(0));
        rt.update_link_quality(node(1), 0.95, 50);
        rt.update_link_quality(node(2), 0.90, 60);

        let mut healer = MockHealer::new();
        healer.expect_propose_topology_change().times(0);

        let mut opt = TopologyOptimizer::new(300);
        let count = opt
            .propose_changes(&rt, Timestamp::from_secs(500), &mut healer)
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn no_proposals_before_interval() {
        let mut rt = RoutingTable::new(node(0));
        rt.update_link_quality(node(1), 0.30, 500);

        let mut healer = MockHealer::new();
        healer.expect_propose_topology_change().times(0);

        let mut opt = TopologyOptimizer::new(300);
        opt.last_rebalance = Timestamp::from_secs(400);
        let count = opt
            .propose_changes(&rt, Timestamp::from_secs(500), &mut healer)
            .unwrap();
        assert_eq!(count, 0);
    }
}
