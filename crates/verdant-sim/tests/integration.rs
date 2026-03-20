use verdant_core::types::{EventCategory, Timestamp, ZoneId};
use verdant_sim::scenario::{Alert, SimConfig, ZoneLayout};
use verdant_sim::sim::{LinearWatershed, Simulation};

fn zone(id: u8) -> ZoneId {
    ZoneId([id, 0, 0, 0])
}

fn default_sim(node_count: usize, zone_count: usize) -> Simulation {
    let config = SimConfig {
        node_count,
        zone_count,
        zone_layout: ZoneLayout::Grid {
            rows: (node_count as f64).sqrt().ceil() as usize,
            cols: (node_count as f64).sqrt().ceil() as usize,
        },
        rf_max_range_m: 500.0,
        training_ticks: 200,
    };
    Simulation::new(config)
}

#[test]
fn simulation_deploys_nodes() {
    let mut sim = default_sim(20, 4);
    sim.deploy_nodes();
    assert!(sim.nodes.len() >= 20);
    assert!(sim.nodes.iter().all(|n| n.alive));
}

#[test]
fn mesh_converges_all_nodes_have_neighbors() {
    let mut sim = default_sim(20, 4);
    sim.deploy_nodes();
    sim.run_until_mesh_converges(10);

    // Every alive node should have at least one active neighbor
    for node in &sim.nodes {
        let neighbor_count = node.routing_table.active_neighbors().count();
        assert!(
            neighbor_count > 0,
            "Node {:?} has no neighbors after convergence",
            node.id
        );
    }
}

#[test]
fn flood_detected_and_downstream_alerted() {
    let mut sim = default_sim(20, 4);
    sim.deploy_nodes();
    sim.run_until_mesh_converges(10);

    // Train with baseline data
    sim.fast_forward_training(200);

    // Inject flood at upstream zone(0)
    sim.inject_flood(zone(0), Timestamp::from_secs(1000));

    // Run a few ticks for consensus
    sim.run_for(5);

    // Check that zone(0) has a confirmed flood event
    let has_flood = sim.has_confirmed_event(zone(0), |cat| {
        matches!(cat, EventCategory::Flood { .. })
    });
    assert!(has_flood, "Zone 0 should have a confirmed flood event");

    // Propagate downstream through watershed
    let watershed = LinearWatershed {
        zone_count: 4,
        flow_time_secs: 2700,
        base_saturation: 0.5,
    };
    sim.propagate_floods(&watershed);

    // Check downstream zones received alerts
    let zone1_alerts = sim.alerts_received_by(zone(1));
    assert!(
        !zone1_alerts.is_empty(),
        "Zone 1 (downstream) should have received a flood alert"
    );
    assert!(
        zone1_alerts.iter().any(|a| matches!(a, Alert::FloodPreemptive(_))),
        "Alert should be a FloodPreemptive"
    );

    let zone2_alerts = sim.alerts_received_by(zone(2));
    assert!(
        !zone2_alerts.is_empty(),
        "Zone 2 (further downstream) should have received a flood alert"
    );
}

#[test]
fn mesh_self_heals_after_node_failure() {
    let mut sim = default_sim(20, 4);
    sim.deploy_nodes();
    sim.run_until_mesh_converges(10);

    // Find and kill a relay node
    let relay = sim.find_relay_node().expect("Should have a relay node");
    sim.kill_node(relay);

    // Re-converge the mesh
    sim.run_until_mesh_converges(10);

    // All surviving nodes should still have neighbors
    for node in &sim.nodes {
        if !node.alive {
            continue;
        }
        let neighbor_count = node.routing_table.active_neighbors().count();
        assert!(
            neighbor_count > 0,
            "Alive node {:?} lost all neighbors after relay failure",
            node.id
        );
    }
}

#[test]
fn pest_detection_across_zone_boundary() {
    let mut sim = default_sim(20, 4);
    sim.deploy_nodes();
    sim.run_until_mesh_converges(10);
    sim.fast_forward_training(200);

    // Inject pest anomaly in zone(0) and zone(1) (crossing boundary)
    let now = Timestamp::from_secs(1000);
    sim.inject_pest(zone(0), now);
    sim.inject_pest(zone(1), now);

    // Run consensus
    sim.run_for(5);

    // Both zones should detect the pest
    let zone0_pest = sim.has_confirmed_event(zone(0), |cat| {
        matches!(cat, EventCategory::Pest { .. })
    });
    let zone1_pest = sim.has_confirmed_event(zone(1), |cat| {
        matches!(cat, EventCategory::Pest { .. })
    });
    assert!(zone0_pest, "Zone 0 should have confirmed pest event");
    assert!(zone1_pest, "Zone 1 should have confirmed pest event");
}

#[test]
fn fifty_node_simulation_runs_without_panic() {
    let mut sim = default_sim(50, 5);
    sim.deploy_nodes();
    sim.run_until_mesh_converges(15);
    sim.fast_forward_training(100);

    // Inject various events
    sim.inject_flood(zone(0), Timestamp::from_secs(500));
    sim.inject_pest(zone(3), Timestamp::from_secs(500));

    sim.run_for(10);

    // Should have some confirmed events
    let total_confirmed: usize = sim.nodes.iter().map(|n| n.confirmed_events.len()).sum();
    assert!(
        total_confirmed > 0,
        "50-node sim should produce confirmed events"
    );
}

#[test]
fn training_creates_seasonal_baselines() {
    let mut sim = default_sim(10, 2);
    sim.deploy_nodes();
    sim.fast_forward_training(200);

    // Every node should have a seasonal baseline
    for node in &sim.nodes {
        let baseline = node.graph.seasonal_baseline(&sim.clock.slot);
        assert!(
            baseline.is_some(),
            "Node {:?} should have a seasonal baseline after training",
            node.id
        );
    }
}

#[test]
fn anomaly_score_low_after_training_with_normal_data() {
    let mut sim = default_sim(10, 2);
    sim.deploy_nodes();
    sim.fast_forward_training(200);

    // Normal observation should have low anomaly score
    for node in &sim.nodes {
        let normal_emb = verdant_sim::node::training_embedding(&node.zone, 100);
        let score = node.graph.anomaly_score(&normal_emb, &sim.clock);
        assert!(
            score < 0.5,
            "Normal embedding should have low anomaly score, got {}",
            score
        );
    }
}

#[test]
fn anomaly_score_high_for_anomalous_data_after_training() {
    let mut sim = default_sim(10, 2);
    sim.deploy_nodes();
    sim.fast_forward_training(200);

    for node in &sim.nodes {
        let anom_emb = verdant_sim::node::anomalous_embedding(&node.zone);
        let score = node.graph.anomaly_score(&anom_emb, &sim.clock);
        assert!(
            score > 0.3,
            "Anomalous embedding should have elevated score, got {}",
            score
        );
    }
}
