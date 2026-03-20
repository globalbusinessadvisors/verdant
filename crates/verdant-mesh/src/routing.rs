use serde::{Deserialize, Serialize};
use verdant_core::config::MAX_NEIGHBORS;
use verdant_core::types::{NodeId, Timestamp};

/// Routing errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingError {
    /// Attempted to add a route to ourselves.
    RouteToSelf,
    /// Negative or NaN cost.
    InvalidCost,
    /// Table is at capacity.
    TableFull,
}

/// Quality metrics for a link to a direct neighbor.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LinkQuality {
    /// Packet delivery ratio (0.0–1.0).
    pub pdr: f32,
    /// Round-trip time in milliseconds.
    pub rtt_ms: u32,
    /// Derived cost: 1.0 / pdr.
    pub cost: f32,
}

impl LinkQuality {
    pub fn from_measurements(pdr: f32, rtt_ms: u32) -> Self {
        let pdr_clamped = pdr.clamp(0.01, 1.0);
        Self {
            pdr: pdr_clamped,
            rtt_ms,
            cost: 1.0 / pdr_clamped,
        }
    }
}

/// Status of a neighboring node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NeighborStatus {
    Active,
    Suspect,
    Dead,
}

/// A directly connected neighbor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Neighbor {
    pub id: NodeId,
    pub link_quality: LinkQuality,
    pub last_seen: Timestamp,
    pub status: NeighborStatus,
}

/// A route to a (possibly distant) destination.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub destination: NodeId,
    pub next_hop: NodeId,
    pub cost: f32,
    pub hops: u8,
    pub last_updated: Timestamp,
}

/// An incoming routing advertisement from a neighbor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoutingUpdate {
    pub from: NodeId,
    pub destination: NodeId,
    pub cost: f32,
    pub hops: u8,
}

/// Distance-vector routing table with empirically-learned link costs.
pub struct RoutingTable {
    self_id: NodeId,
    routes: heapless::Vec<Route, MAX_NEIGHBORS>,
    neighbors: heapless::Vec<Neighbor, MAX_NEIGHBORS>,
    /// Well-known gateway node IDs (provisioned at setup time).
    gateway_ids: heapless::Vec<NodeId, 4>,
}

impl RoutingTable {
    pub fn new(self_id: NodeId) -> Self {
        Self {
            self_id,
            routes: heapless::Vec::new(),
            neighbors: heapless::Vec::new(),
            gateway_ids: heapless::Vec::new(),
        }
    }

    pub fn self_id(&self) -> NodeId {
        self.self_id
    }

    /// Register a well-known gateway node ID.
    pub fn add_gateway(&mut self, id: NodeId) {
        if !self.gateway_ids.contains(&id) {
            let _ = self.gateway_ids.push(id);
        }
    }

    /// Returns true if any gateway is reachable.
    pub fn has_gateway_route(&self) -> bool {
        self.gateway_ids.iter().any(|gw| self.best_route(*gw).is_some())
    }

    /// Apply a routing update from a neighbor.
    ///
    /// Returns `Ok(true)` if the routing table changed, `Ok(false)` if not.
    /// Enforces: no routes to self, no negative costs, split-horizon.
    pub fn apply_update(&mut self, update: RoutingUpdate) -> Result<bool, RoutingError> {
        if update.destination == self.self_id {
            return Err(RoutingError::RouteToSelf);
        }
        if update.cost < 0.0 || update.cost.is_nan() {
            return Err(RoutingError::InvalidCost);
        }

        // Look up the link cost to the advertising neighbor
        let link_cost = self
            .neighbors
            .iter()
            .find(|n| n.id == update.from)
            .map(|n| n.link_quality.cost)
            .unwrap_or(1.0);

        let total_cost = link_cost + update.cost;

        // Check if we already have a route to this destination
        if let Some(existing) = self.routes.iter_mut().find(|r| r.destination == update.destination)
        {
            if total_cost < existing.cost {
                existing.next_hop = update.from;
                existing.cost = total_cost;
                existing.hops = update.hops.saturating_add(1);
                existing.last_updated = Timestamp(0); // caller should set
                return Ok(true);
            }
            return Ok(false);
        }

        // New route
        let route = Route {
            destination: update.destination,
            next_hop: update.from,
            cost: total_cost,
            hops: update.hops.saturating_add(1),
            last_updated: Timestamp(0),
        };
        self.routes.push(route).map_err(|_| RoutingError::TableFull)?;
        Ok(true)
    }

    /// Get the best (lowest cost) route to a destination.
    pub fn best_route(&self, destination: NodeId) -> Option<&Route> {
        self.routes
            .iter()
            .filter(|r| r.destination == destination)
            .min_by(|a, b| a.cost.partial_cmp(&b.cost).unwrap_or(core::cmp::Ordering::Equal))
    }

    /// Update link quality measurements for a neighbor.
    pub fn update_link_quality(&mut self, neighbor_id: NodeId, pdr: f32, rtt_ms: u32) {
        let lq = LinkQuality::from_measurements(pdr, rtt_ms);
        if let Some(n) = self.neighbors.iter_mut().find(|n| n.id == neighbor_id) {
            n.link_quality = lq;
            n.status = NeighborStatus::Active;
        } else {
            let _ = self.neighbors.push(Neighbor {
                id: neighbor_id,
                link_quality: lq,
                last_seen: Timestamp(0),
                status: NeighborStatus::Active,
            });
        }
    }

    /// Mark a neighbor as suspect (missed heartbeat).
    pub fn mark_suspect(&mut self, neighbor_id: NodeId) {
        if let Some(n) = self.neighbors.iter_mut().find(|n| n.id == neighbor_id) {
            n.status = NeighborStatus::Suspect;
        }
    }

    /// Mark a neighbor as dead and invalidate routes through it.
    pub fn mark_dead(&mut self, neighbor_id: NodeId) {
        if let Some(n) = self.neighbors.iter_mut().find(|n| n.id == neighbor_id) {
            n.status = NeighborStatus::Dead;
        }
        // Remove all routes that use this neighbor as next hop
        self.routes.retain(|r| r.next_hop != neighbor_id);
    }

    /// Iterate over active (non-dead) neighbors.
    pub fn active_neighbors(&self) -> impl Iterator<Item = &Neighbor> {
        self.neighbors.iter().filter(|n| n.status != NeighborStatus::Dead)
    }

    /// Collect routes that should be advertised to a specific neighbor.
    ///
    /// Implements split-horizon: routes learned from `target` are excluded.
    pub fn routes_for_advertisement(&self, target: NodeId) -> impl Iterator<Item = &Route> {
        self.routes.iter().filter(move |r| r.next_hop != target)
    }

    /// Heuristic: should we rebalance topology? True if any active neighbor
    /// has significantly degraded link quality.
    pub fn should_rebalance(&self) -> bool {
        self.active_neighbors().any(|n| n.link_quality.pdr < 0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u8) -> NodeId {
        NodeId([id; 8])
    }

    fn table_with_neighbors() -> RoutingTable {
        let mut rt = RoutingTable::new(node(0));
        rt.update_link_quality(node(1), 0.95, 50);
        rt.update_link_quality(node(2), 0.60, 200);
        rt
    }

    #[test]
    fn apply_update_selects_lowest_cost_path() {
        let mut rt = table_with_neighbors();

        // Route to node(10) via node(1): link_cost=1/0.95≈1.05, adv_cost=2.0, total≈3.05
        rt.apply_update(RoutingUpdate {
            from: node(1),
            destination: node(10),
            cost: 2.0,
            hops: 3,
        })
        .unwrap();

        // Route to node(10) via node(2): link_cost=1/0.60≈1.67, adv_cost=1.0, total≈2.67
        rt.apply_update(RoutingUpdate {
            from: node(2),
            destination: node(10),
            cost: 1.0,
            hops: 2,
        })
        .unwrap();

        let best = rt.best_route(node(10)).unwrap();
        assert_eq!(best.next_hop, node(2)); // cheaper total
    }

    #[test]
    fn apply_update_rejects_route_to_self() {
        let mut rt = RoutingTable::new(node(0));
        let result = rt.apply_update(RoutingUpdate {
            from: node(1),
            destination: node(0),
            cost: 1.0,
            hops: 1,
        });
        assert_eq!(result, Err(RoutingError::RouteToSelf));
    }

    #[test]
    fn apply_update_rejects_negative_cost() {
        let mut rt = RoutingTable::new(node(0));
        let result = rt.apply_update(RoutingUpdate {
            from: node(1),
            destination: node(5),
            cost: -1.0,
            hops: 1,
        });
        assert_eq!(result, Err(RoutingError::InvalidCost));
    }

    #[test]
    fn apply_update_rejects_nan_cost() {
        let mut rt = RoutingTable::new(node(0));
        let result = rt.apply_update(RoutingUpdate {
            from: node(1),
            destination: node(5),
            cost: f32::NAN,
            hops: 1,
        });
        assert_eq!(result, Err(RoutingError::InvalidCost));
    }

    #[test]
    fn mark_dead_removes_from_active_neighbors() {
        let mut rt = table_with_neighbors();
        assert_eq!(rt.active_neighbors().count(), 2);

        rt.mark_dead(node(1));
        let active: heapless::Vec<NodeId, 8> = rt.active_neighbors().map(|n| n.id).collect();
        assert!(!active.contains(&node(1)));
        assert!(active.contains(&node(2)));
    }

    #[test]
    fn mark_dead_invalidates_routes_through_dead_node() {
        let mut rt = table_with_neighbors();
        rt.apply_update(RoutingUpdate {
            from: node(1),
            destination: node(10),
            cost: 1.0,
            hops: 1,
        })
        .unwrap();
        assert!(rt.best_route(node(10)).is_some());

        rt.mark_dead(node(1));
        assert!(rt.best_route(node(10)).is_none());
    }

    #[test]
    fn split_horizon_excludes_learned_routes() {
        let mut rt = table_with_neighbors();
        rt.apply_update(RoutingUpdate {
            from: node(1),
            destination: node(10),
            cost: 1.0,
            hops: 1,
        })
        .unwrap();
        rt.apply_update(RoutingUpdate {
            from: node(2),
            destination: node(20),
            cost: 1.0,
            hops: 1,
        })
        .unwrap();

        // Routes to advertise to node(1): exclude routes learned from node(1)
        let for_node1: heapless::Vec<NodeId, 8> =
            rt.routes_for_advertisement(node(1)).map(|r| r.destination).collect();
        assert!(!for_node1.contains(&node(10))); // learned from node(1)
        assert!(for_node1.contains(&node(20))); // learned from node(2), ok to share
    }

    #[test]
    fn update_link_quality_creates_or_updates_neighbor() {
        let mut rt = RoutingTable::new(node(0));
        assert_eq!(rt.active_neighbors().count(), 0);

        rt.update_link_quality(node(5), 0.80, 100);
        assert_eq!(rt.active_neighbors().count(), 1);

        // Update again
        rt.update_link_quality(node(5), 0.90, 80);
        assert_eq!(rt.active_neighbors().count(), 1);
        let n = rt.active_neighbors().next().unwrap();
        assert!((n.link_quality.pdr - 0.90).abs() < 0.001);
    }

    #[test]
    fn should_rebalance_when_poor_link() {
        let mut rt = RoutingTable::new(node(0));
        rt.update_link_quality(node(1), 0.95, 50);
        assert!(!rt.should_rebalance());

        rt.update_link_quality(node(2), 0.30, 500);
        assert!(rt.should_rebalance());
    }

    #[test]
    fn has_gateway_route() {
        let mut rt = table_with_neighbors();
        rt.add_gateway(node(99));
        assert!(!rt.has_gateway_route());

        rt.apply_update(RoutingUpdate {
            from: node(1),
            destination: node(99),
            cost: 1.0,
            hops: 2,
        })
        .unwrap();
        assert!(rt.has_gateway_route());
    }

    #[test]
    fn apply_update_replaces_with_lower_cost() {
        let mut rt = table_with_neighbors();
        rt.apply_update(RoutingUpdate {
            from: node(1),
            destination: node(10),
            cost: 5.0,
            hops: 3,
        })
        .unwrap();

        let cost_before = rt.best_route(node(10)).unwrap().cost;

        // Better route arrives
        let changed = rt
            .apply_update(RoutingUpdate {
                from: node(1),
                destination: node(10),
                cost: 1.0,
                hops: 1,
            })
            .unwrap();

        assert!(changed);
        assert!(rt.best_route(node(10)).unwrap().cost < cost_before);
    }

    #[test]
    fn apply_update_ignores_worse_cost() {
        let mut rt = table_with_neighbors();
        rt.apply_update(RoutingUpdate {
            from: node(1),
            destination: node(10),
            cost: 1.0,
            hops: 1,
        })
        .unwrap();

        let changed = rt
            .apply_update(RoutingUpdate {
                from: node(1),
                destination: node(10),
                cost: 10.0,
                hops: 5,
            })
            .unwrap();

        assert!(!changed);
    }

    #[test]
    fn mark_suspect_preserves_in_active() {
        let mut rt = table_with_neighbors();
        rt.mark_suspect(node(1));
        // Suspect neighbors are still visible in active_neighbors
        let active: heapless::Vec<NodeId, 8> = rt.active_neighbors().map(|n| n.id).collect();
        assert!(active.contains(&node(1)));
    }
}
