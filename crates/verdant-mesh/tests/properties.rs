use proptest::prelude::*;
use verdant_core::types::NodeId;
use verdant_mesh::routing::{RoutingTable, RoutingUpdate};

fn arb_node_id() -> impl Strategy<Value = NodeId> {
    proptest::array::uniform8(any::<u8>()).prop_map(NodeId)
}

fn arb_cost() -> impl Strategy<Value = f32> {
    (1u32..10000u32).prop_map(|c| c as f32 / 100.0)
}

proptest! {
    /// best_route must always return the lowest-cost route when multiple exist.
    #[test]
    fn best_route_always_lowest_cost(
        costs in proptest::collection::vec(arb_cost(), 2..10)
    ) {
        let self_id = NodeId([0; 8]);
        let dest = NodeId([99; 8]);
        let mut rt = RoutingTable::new(self_id);

        let mut min_total = f32::MAX;

        for (i, &adv_cost) in costs.iter().enumerate() {
            let from = NodeId([(i as u8) + 1; 8]);
            rt.update_link_quality(from, 0.9, 50);
            let _ = rt.apply_update(RoutingUpdate {
                from,
                destination: dest,
                cost: adv_cost,
                hops: 1,
            });
            // link cost = 1/0.9 ≈ 1.11
            let total = (1.0 / 0.9) + adv_cost;
            if total < min_total {
                min_total = total;
            }
        }

        if let Some(best) = rt.best_route(dest) {
            prop_assert!(
                (best.cost - min_total).abs() < 0.1,
                "best_route cost {} != expected min {}",
                best.cost,
                min_total
            );
        }
    }

    /// The routing table must never contain a route to self, regardless
    /// of what updates are applied.
    #[test]
    fn routing_table_never_contains_self_route(
        self_id in arb_node_id(),
        updates in proptest::collection::vec(
            (arb_node_id(), arb_cost(), 1..10u8),
            0..20
        )
    ) {
        let mut rt = RoutingTable::new(self_id);

        for (from, cost, hops) in updates {
            if from == self_id { continue; }
            rt.update_link_quality(from, 0.8, 100);
            // Try to insert a route to self (should be rejected)
            let _ = rt.apply_update(RoutingUpdate {
                from,
                destination: self_id,
                cost,
                hops,
            });
        }

        prop_assert!(
            rt.best_route(self_id).is_none(),
            "Found a route to self!"
        );
    }
}
