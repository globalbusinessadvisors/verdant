use verdant_core::types::Timestamp;

use crate::routing::RoutingTable;

/// Default gateway contact timeout in seconds.
const DEFAULT_GATEWAY_TIMEOUT_SECS: u64 = 600;

/// Network connectivity state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectivityMode {
    /// Gateway reachable, recent contact.
    Normal,
    /// Gateway route exists but contact is stale.
    Degraded,
    /// No route to any gateway.
    Partitioned,
    /// Partitioned and operating in epidemic gossip mode.
    Epidemic,
}

/// Detects mesh partition state based on gateway reachability and contact age.
pub struct PartitionDetector {
    last_gateway_contact: Option<Timestamp>,
    gateway_timeout_secs: u64,
    in_epidemic: bool,
}

impl PartitionDetector {
    pub fn new() -> Self {
        Self {
            last_gateway_contact: None,
            gateway_timeout_secs: DEFAULT_GATEWAY_TIMEOUT_SECS,
            in_epidemic: false,
        }
    }

    /// Record that we successfully contacted a gateway at `now`.
    pub fn record_gateway_contact(&mut self, now: Timestamp) {
        self.last_gateway_contact = Some(now);
        self.in_epidemic = false;
    }

    /// Evaluate current connectivity mode.
    pub fn check(&mut self, routing_table: &RoutingTable, now: Timestamp) -> ConnectivityMode {
        if !routing_table.has_gateway_route() {
            self.in_epidemic = true;
            return ConnectivityMode::Partitioned;
        }

        match self.last_gateway_contact {
            Some(last) => {
                let elapsed_ms = now.saturating_diff(&last);
                let timeout_ms = self.gateway_timeout_secs * 1000;
                if elapsed_ms > timeout_ms {
                    ConnectivityMode::Degraded
                } else {
                    self.in_epidemic = false;
                    ConnectivityMode::Normal
                }
            }
            None => ConnectivityMode::Degraded,
        }
    }

    /// Whether epidemic gossip mode is active.
    pub fn is_epidemic(&self) -> bool {
        self.in_epidemic
    }
}

impl Default for PartitionDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::{RoutingTable, RoutingUpdate};
    use verdant_core::types::NodeId;

    fn node(id: u8) -> NodeId {
        NodeId([id; 8])
    }

    fn table_with_gateway() -> RoutingTable {
        let mut rt = RoutingTable::new(node(0));
        rt.update_link_quality(node(1), 0.9, 50);
        rt.add_gateway(node(99));
        rt.apply_update(RoutingUpdate {
            from: node(1),
            destination: node(99),
            cost: 1.0,
            hops: 2,
        })
        .unwrap();
        rt
    }

    #[test]
    fn check_returns_partitioned_when_no_gateway_route() {
        let rt = RoutingTable::new(node(0));
        let mut pd = PartitionDetector::new();
        let mode = pd.check(&rt, Timestamp::from_secs(1000));
        assert_eq!(mode, ConnectivityMode::Partitioned);
        assert!(pd.is_epidemic());
    }

    #[test]
    fn check_returns_normal_when_gateway_reachable_and_recent() {
        let rt = table_with_gateway();
        let mut pd = PartitionDetector::new();
        pd.record_gateway_contact(Timestamp::from_secs(990));
        let mode = pd.check(&rt, Timestamp::from_secs(1000));
        assert_eq!(mode, ConnectivityMode::Normal);
        assert!(!pd.is_epidemic());
    }

    #[test]
    fn check_returns_degraded_when_gateway_route_exists_but_stale() {
        let rt = table_with_gateway();
        let mut pd = PartitionDetector::new();
        pd.record_gateway_contact(Timestamp::from_secs(100));
        let mode = pd.check(&rt, Timestamp::from_secs(2000));
        assert_eq!(mode, ConnectivityMode::Degraded);
    }

    #[test]
    fn check_returns_degraded_when_never_contacted() {
        let rt = table_with_gateway();
        let mut pd = PartitionDetector::new();
        let mode = pd.check(&rt, Timestamp::from_secs(1000));
        assert_eq!(mode, ConnectivityMode::Degraded);
    }

    #[test]
    fn epidemic_clears_on_normal_contact() {
        let mut pd = PartitionDetector::new();
        let rt_empty = RoutingTable::new(node(0));
        pd.check(&rt_empty, Timestamp::from_secs(100));
        assert!(pd.is_epidemic());

        pd.record_gateway_contact(Timestamp::from_secs(200));
        assert!(!pd.is_epidemic());
    }
}
