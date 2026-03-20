# ADR-008: Terrain-Aware Mesh Topology and Routing

**Status:** Accepted
**Date:** 2026-03-20
**Deciders:** Verdant Core Team
**SPARC Refs:** FR-M1, FR-M2, FR-M6, FR-M7, G4, G6, NFR-3, NFR-4, NFR-6

## Context

Vermont's terrain — mountains, valleys, dense forests — creates severe RF propagation challenges. Line-of-sight between nodes is rare. Standard mesh routing protocols (AODV, OLSR) assume relatively flat terrain with predictable signal propagation. Verdant needs a routing protocol that accounts for elevation, forest density, seasonal foliage changes, and weather-dependent signal attenuation.

Candidate approaches:
1. **Standard AODV/OLSR** — well-studied but terrain-unaware; wastes energy on paths that don't work
2. **RPL (IoT standard)** — designed for constrained networks but assumes a root/gateway hierarchy
3. **Custom distance-vector with terrain cost** — adapts to Vermont's reality; nodes learn link quality empirically
4. **Geographic routing (GPSR)** — requires GPS, which is unreliable in mountain valleys

## Decision

Use a **custom distance-vector routing protocol** with empirically-learned, terrain-aware link costs, BLE-based neighbor discovery, and epidemic partition recovery.

## Rationale

- **Empirical link costs** replace theoretical RF models. Each node measures actual packet delivery ratio (PDR) and round-trip time to each neighbor. These measurements naturally account for terrain, foliage, weather, and interference without needing a terrain model.
- **Seasonal adaptation.** Deciduous forests attenuate WiFi differently in summer (full canopy) vs. winter (bare branches). Link costs update continuously, so routing adapts to seasonal changes.
- **BLE for discovery, WiFi for data.** BLE beacons are low-power and penetrate obstacles better than WiFi. Nodes discover neighbors via BLE, then establish WiFi data links only with viable neighbors.
- **Epidemic partition recovery.** When a partition is detected (no path to gateway or to a known distant zone), nodes switch to an epidemic protocol — flooding gossip messages that carry both data and routing table fragments. This enables reconnection without manual intervention.
- **No GPS dependency.** Geographic routing requires GPS, which is unreliable in Vermont's mountain valleys. Distance-vector routing only needs neighbor-to-neighbor communication.

## Consequences

- **Positive:** Self-calibrating for terrain; adapts seasonally; no GPS; self-healing partitions
- **Negative:** Convergence time for large meshes (>1000 nodes) may be slow (minutes); count-to-infinity problem in distance-vector
- **Mitigated by:** Split-horizon and path-vector extensions to prevent count-to-infinity; zone-level route aggregation to reduce table size

## London School TDD Approach

```rust
pub trait NeighborDiscovery {
    fn scan_neighbors(&mut self) -> Result<Vec<NeighborBeacon>, DiscoveryError>;
    fn broadcast_beacon(&mut self, beacon: &Beacon) -> Result<(), DiscoveryError>;
}

pub trait LinkQualityMonitor {
    fn measure_link(&self, neighbor: NodeId) -> LinkQuality;
    fn update_measurement(&mut self, neighbor: NodeId, pdr: f32, rtt_ms: u32);
}

pub trait RoutingTablePersistence {
    fn save(&mut self, table: &RoutingTable) -> Result<(), StorageError>;
    fn load(&self) -> Result<RoutingTable, StorageError>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn routing_update_selects_lowest_cost_path() {
        let mut mock_link = MockLinkQualityMonitor::new();

        // GIVEN: two neighbors with different link qualities
        mock_link.expect_measure_link()
            .with(eq(node_a))
            .returning(|_| LinkQuality { pdr: 0.95, rtt_ms: 50, cost: 1.05 });

        mock_link.expect_measure_link()
            .with(eq(node_b))
            .returning(|_| LinkQuality { pdr: 0.60, rtt_ms: 200, cost: 3.33 });

        let mut router = MeshRouter::new(mock_link, mock_persistence);

        // AND: both advertise a route to the gateway
        router.handle_routing_update(RoutingUpdate {
            from: node_a,
            destination: gateway,
            cost: 2.0,
            hops: 3,
        });
        router.handle_routing_update(RoutingUpdate {
            from: node_b,
            destination: gateway,
            cost: 1.0,  // cheaper advertised cost
            hops: 2,
        });

        // WHEN: we look up the route to gateway
        let route = router.best_route(gateway);

        // THEN: node_a is selected (total cost: 1.05+2.0=3.05 < 3.33+1.0=4.33)
        assert_eq!(route.unwrap().next_hop, node_a);
    }

    #[test]
    fn partition_detected_triggers_epidemic_mode() {
        let mut mock_discovery = MockNeighborDiscovery::new();
        let mut mock_persistence = MockRoutingTablePersistence::new();

        // GIVEN: all neighbors have gone silent
        mock_discovery.expect_scan_neighbors()
            .returning(|| Ok(vec![]));

        // THEN: routing table is persisted (survive reboot during partition)
        mock_persistence.expect_save()
            .times(1)
            .returning(|_| Ok(()));

        let mut router = MeshRouter::new(mock_link, mock_persistence);
        router.add_neighbor(node_a);
        router.mark_dead(node_a);

        // WHEN: SAFLA cycle checks for partition
        let mode = router.check_connectivity();

        assert_eq!(mode, ConnectivityMode::Partitioned);
        assert!(router.is_epidemic_mode());
    }

    #[test]
    fn bandwidth_adaptive_compression_degrades_gracefully() {
        let mut mock_link = MockLinkQualityMonitor::new();

        // GIVEN: a very poor link (high loss, low bandwidth)
        mock_link.expect_measure_link()
            .with(eq(node_a))
            .returning(|_| LinkQuality { pdr: 0.30, rtt_ms: 800, cost: 10.0 });

        let router = MeshRouter::new(mock_link, mock_persistence);
        let compression = router.compression_level_for(node_a);

        // THEN: maximum compression is selected (trade CPU for bandwidth)
        assert_eq!(compression, CompressionLevel::Maximum);
    }
}
```
