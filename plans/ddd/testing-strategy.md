# Verdant Testing Strategy: London School TDD

## Comprehensive Testing Methodology

---

## 1. London School TDD Principles Applied to Verdant

### Core Philosophy

> **Test behavior, not state. Mock collaborators, not the subject under test. Start from the outside, work inward.**

In the London School (Mockist) approach:
- Each unit test isolates the **subject under test** (SUT) by mocking all its **collaborators**
- Tests verify that the SUT **interacts correctly** with collaborators (calls the right methods with the right arguments)
- Tests are written **outside-in**: start with the outermost acceptance test, discover needed interfaces, work inward
- Mocks are verified on drop (Rust `mockall`) or via `verify()` (TypeScript `ts-mockito`)

### Why London School for Verdant Specifically

| Challenge | London School Solution |
|-----------|----------------------|
| ESP32 firmware can't run on dev laptop | Mock hardware traits (`CsiCapture`, `FlashStorage`) — test logic on x86 |
| Mesh protocol requires multiple nodes | Mock `MeshTransport` — test protocol logic with one node |
| Post-quantum crypto is CPU-intensive | Mock `PostQuantumCrypto` — test message flow without real crypto |
| Sensor data varies by physical location | Mock `EnvironmentalSensor` — inject deterministic test data |
| Gateway needs database | Mock `EventStore`, `NodeStatusStore` — test API logic without redb |
| Dashboard needs gateway WebSocket | Mock `MeshDataPort` — test UI rendering without network |

---

## 2. Testing Layers

### Layer 1: London School Unit Tests (Fast, Many)

**Target:** <10 seconds for entire workspace
**Location:** `#[cfg(test)] mod tests` in each source file
**Tool:** `mockall` (Rust), `ts-mockito` or `jest.fn()` (TypeScript)

#### Rust Pattern

```rust
// File: crates/verdant-safla/src/consensus.rs

use crate::ports::{AnomalySource, ConfirmedEventSink};

pub struct ConsensusEngine<A: AnomalySource, S: ConfirmedEventSink> {
    source: A,
    sink: S,
    quorum: u8,
    temporal_window_secs: u64,
    spatial_radius_m: f32,
}

impl<A: AnomalySource, S: ConfirmedEventSink> ConsensusEngine<A, S> {
    pub fn evaluate(&mut self) -> Result<Vec<EventId>, ConsensusError> {
        let local = self.source.local_anomalies();
        let neighbor = self.source.neighbor_anomalies(self.temporal_window_secs);

        let mut confirmed = Vec::new();
        for anomaly in &local {
            let corroborating: Vec<_> = neighbor.iter()
                .filter(|n| {
                    n.spatial_distance(anomaly) < self.spatial_radius_m
                    && n.temporal_distance(anomaly) < self.temporal_window_secs
                    && n.category == anomaly.category
                })
                .collect();

            if corroborating.len() >= self.quorum as usize {
                let event = ConfirmedEvent::from_consensus(anomaly, &corroborating);
                let id = self.sink.emit_confirmed(&event)?;
                confirmed.push(id);
            }
        }
        Ok(confirmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    // Generate mocks from traits
    mock! {
        Source {}
        impl AnomalySource for Source {
            fn local_anomalies(&self) -> Vec<Anomaly>;
            fn neighbor_anomalies(&self, window_secs: u64) -> Vec<NeighborAnomaly>;
        }
    }

    mock! {
        Sink {}
        impl ConfirmedEventSink for Sink {
            fn emit_confirmed(&mut self, event: &ConfirmedEvent) -> Result<EventId, EmitError>;
            fn request_robot_task(&mut self, event: &ConfirmedEvent) -> Result<(), EmitError>;
        }
    }

    fn make_engine(source: MockSource, sink: MockSink) -> ConsensusEngine<MockSource, MockSink> {
        ConsensusEngine {
            source,
            sink,
            quorum: 3,
            temporal_window_secs: 300,
            spatial_radius_m: 500.0,
        }
    }

    #[test]
    fn confirms_event_when_quorum_met() {
        let mut source = MockSource::new();
        let mut sink = MockSink::new();

        source.expect_local_anomalies()
            .returning(|| vec![pest_anomaly_at(zone_a_center())]);

        source.expect_neighbor_anomalies()
            .with(eq(300))
            .returning(|_| vec![
                nearby_pest(zone_a_north(), 60),
                nearby_pest(zone_a_south(), 120),
                nearby_pest(zone_a_east(), 30),
            ]);

        sink.expect_emit_confirmed()
            .withf(|e| e.category == EventCategory::Pest && e.confidence > 0.8)
            .times(1)
            .returning(|_| Ok(EventId::new()));

        let mut engine = make_engine(source, sink);
        let confirmed = engine.evaluate().unwrap();
        assert_eq!(confirmed.len(), 1);
    }

    #[test]
    fn does_not_confirm_below_quorum() {
        let mut source = MockSource::new();
        let mut sink = MockSink::new();

        source.expect_local_anomalies()
            .returning(|| vec![pest_anomaly_at(zone_a_center())]);

        source.expect_neighbor_anomalies()
            .returning(|_| vec![
                nearby_pest(zone_a_north(), 60),
                // Only 1 corroboration, need 3
            ]);

        // Verify: emit_confirmed is NEVER called
        sink.expect_emit_confirmed().times(0);

        let mut engine = make_engine(source, sink);
        let confirmed = engine.evaluate().unwrap();
        assert_eq!(confirmed.len(), 0);
    }

    #[test]
    fn filters_temporally_distant_anomalies() {
        let mut source = MockSource::new();
        let mut sink = MockSink::new();

        source.expect_local_anomalies()
            .returning(|| vec![pest_anomaly_at(zone_a_center())]);

        source.expect_neighbor_anomalies()
            .returning(|_| vec![
                nearby_pest(zone_a_north(), 60),     // within window
                nearby_pest(zone_a_south(), 400),    // OUTSIDE 300s window
                nearby_pest(zone_a_east(), 500),     // OUTSIDE 300s window
            ]);

        // Only 1 valid corroboration, below quorum
        sink.expect_emit_confirmed().times(0);

        let mut engine = make_engine(source, sink);
        engine.evaluate().unwrap();
    }

    #[test]
    fn filters_spatially_distant_anomalies() {
        let mut source = MockSource::new();
        let mut sink = MockSink::new();

        source.expect_local_anomalies()
            .returning(|| vec![pest_anomaly_at(zone_a_center())]);

        source.expect_neighbor_anomalies()
            .returning(|_| vec![
                nearby_pest(zone_a_north(), 60),              // nearby, within window
                far_pest(zone_d_center(), 60),                // FAR (different zone)
                far_pest(Position::km_away(10.0), 60),        // FAR
            ]);

        sink.expect_emit_confirmed().times(0);

        let mut engine = make_engine(source, sink);
        engine.evaluate().unwrap();
    }

    #[test]
    fn filters_different_category_anomalies() {
        let mut source = MockSource::new();
        let mut sink = MockSink::new();

        source.expect_local_anomalies()
            .returning(|| vec![pest_anomaly_at(zone_a_center())]);

        source.expect_neighbor_anomalies()
            .returning(|_| vec![
                nearby_pest(zone_a_north(), 60),              // pest ✓
                nearby_flood(zone_a_south(), 60),             // wrong category
                nearby_fire(zone_a_east(), 60),               // wrong category
            ]);

        sink.expect_emit_confirmed().times(0);

        let mut engine = make_engine(source, sink);
        engine.evaluate().unwrap();
    }
}
```

#### TypeScript Pattern

```typescript
// File: ui/src/hooks/__tests__/useMeshData.test.ts

import { mock, instance, when, verify, anything } from "ts-mockito";

describe("useMeshData", () => {
    let mockMeshPort: MeshDataPort;
    let mockOfflinePort: OfflineSyncPort;

    beforeEach(() => {
        mockMeshPort = mock<MeshDataPort>();
        mockOfflinePort = mock<OfflineSyncPort>();
    });

    it("subscribes to gateway events on mount", () => {
        when(mockMeshPort.subscribeEvents(anything())).thenReturn(() => {});

        renderHook(() =>
            useMeshData(instance(mockMeshPort), instance(mockOfflinePort))
        );

        verify(mockMeshPort.subscribeEvents(anything())).once();
    });

    it("unsubscribes on unmount", () => {
        const unsubscribe = jest.fn();
        when(mockMeshPort.subscribeEvents(anything())).thenReturn(unsubscribe);

        const { unmount } = renderHook(() =>
            useMeshData(instance(mockMeshPort), instance(mockOfflinePort))
        );

        unmount();
        expect(unsubscribe).toHaveBeenCalledTimes(1);
    });

    it("queues failed writes for sync when offline", async () => {
        when(mockMeshPort.fetchNodeStatuses())
            .thenReject(new Error("Network unreachable"));

        when(mockOfflinePort.getLocalState("nodeStatuses"))
            .thenResolve([cachedNode("n1")]);

        const { result } = renderHook(() =>
            useMeshData(instance(mockMeshPort), instance(mockOfflinePort))
        );

        await waitFor(() => {
            expect(result.current.isOffline).toBe(true);
        });
    });
});
```

---

### Layer 2: Property-Based Tests (Algorithmic Correctness)

**Target:** <30 seconds
**Location:** `tests/` directory in algorithmic crates
**Tool:** `proptest` (Rust)

These test **invariants** rather than specific interactions. Used for pure algorithmic code where London School mocking is unnecessary.

```rust
// File: crates/verdant-vector/tests/graph_properties.rs

use proptest::prelude::*;

proptest! {
    // Invariant: graph never exceeds capacity
    #[test]
    fn graph_never_exceeds_capacity(
        embeddings in prop::collection::vec(arb_embedding(), 0..5000)
    ) {
        let mut graph = VectorGraph::new(2048);
        let clock = FixedClock::new(SeasonSlot::new(6, 1));

        for emb in embeddings {
            graph.update(emb, &clock);
        }

        prop_assert!(graph.node_count() <= 2048);
    }

    // Invariant: anomaly score is always in [0.0, 1.0]
    #[test]
    fn anomaly_score_bounded(
        baseline in arb_embedding(),
        sample in arb_embedding()
    ) {
        let mut graph = VectorGraph::new(2048);
        let clock = FixedClock::new(SeasonSlot::new(3, 12));

        // Train baseline
        for _ in 0..50 {
            graph.update(baseline.clone(), &clock);
        }

        let score = graph.anomaly_score(&sample, &clock);
        prop_assert!(score >= 0.0 && score <= 1.0);
    }

    // Invariant: merge is idempotent-ish (adding same embedding many times
    // keeps count growing but node count stable)
    #[test]
    fn repeated_identical_embeddings_merge(
        embedding in arb_embedding(),
        count in 2..100usize
    ) {
        let mut graph = VectorGraph::new(2048);
        let clock = FixedClock::new(SeasonSlot::new(1, 1));

        for _ in 0..count {
            graph.update(embedding.clone(), &clock);
        }

        // Should have merged into 1 node
        prop_assert_eq!(graph.node_count(), 1);
        prop_assert_eq!(graph.nodes()[0].visit_count, count as u32);
    }

    // Invariant: version is monotonically increasing
    #[test]
    fn version_monotonically_increases(
        embeddings in prop::collection::vec(arb_embedding(), 1..100)
    ) {
        let mut graph = VectorGraph::new(2048);
        let clock = FixedClock::new(SeasonSlot::new(6, 1));
        let mut prev_version = graph.version();

        for emb in embeddings {
            graph.update(emb, &clock);
            let new_version = graph.version();
            prop_assert!(new_version >= prev_version);
            prev_version = new_version;
        }
    }
}

// DAG properties
proptest! {
    // Invariant: valid DAG accepts messages with known parents
    #[test]
    fn dag_accepts_messages_with_valid_parents(
        chain_length in 2..50usize
    ) {
        let mut dag = DagState::new();

        // Build a chain of messages
        let mut prev_hash = dag.genesis_hash();
        for _ in 0..chain_length {
            let msg = QuDagMessage {
                dag_parents: vec![prev_hash],
                ..test_message()
            };
            let result = dag.validate_and_insert(&msg);
            prop_assert!(result.is_ok());
            prev_hash = msg.hash();
        }
    }

    // Invariant: DAG rejects orphaned messages
    #[test]
    fn dag_rejects_orphaned_messages(
        unknown_hash in arb_message_hash()
    ) {
        let dag = DagState::new();

        let msg = QuDagMessage {
            dag_parents: vec![unknown_hash],
            ..test_message()
        };

        let result = dag.validate_and_insert(&msg);
        prop_assert!(result.is_err());
    }
}

// Routing properties
proptest! {
    // Invariant: routing table never contains a route to self
    #[test]
    fn routing_table_rejects_self_routes(
        self_id in arb_node_id(),
        updates in prop::collection::vec(arb_routing_update(), 0..100)
    ) {
        let mut table = RoutingTable::new(self_id);

        for mut update in updates {
            update.destination = self_id; // force destination to self
            let _ = table.apply_update(update); // should reject
        }

        prop_assert!(table.route_to(self_id).is_none());
    }

    // Invariant: best route has lowest total cost
    #[test]
    fn best_route_is_lowest_cost(
        routes in prop::collection::vec(arb_route(), 2..20)
    ) {
        let mut table = RoutingTable::new(arb_node_id_fixed());
        let dest = NodeId::fixed_test();

        for route in &routes {
            table.insert_route(dest, route.clone());
        }

        if let Some(best) = table.best_route(dest) {
            let min_cost = routes.iter().map(|r| r.total_cost()).min_by(f32::total_cmp).unwrap();
            prop_assert!((best.total_cost() - min_cost).abs() < f32::EPSILON);
        }
    }
}
```

---

### Layer 3: Integration / Simulation Tests (End-to-End)

**Target:** <5 minutes
**Location:** `crates/verdant-sim/tests/`
**Tool:** `verdant-sim` with `bevy_ecs`

These are the **acceptance tests** that drive outside-in development.

```rust
// File: crates/verdant-sim/tests/flood_detection.rs

#[test]
fn flood_detected_and_downstream_alerted_within_60_seconds() {
    let mut sim = Simulation::new(SimConfig {
        terrain: load_vermont_dem("tests/fixtures/winooski_watershed.dem"),
        node_count: 50,
        zone_layout: ZoneLayout::Watershed,
        ..default()
    });

    // Deploy nodes across watershed
    sim.deploy_nodes();
    sim.run_until_mesh_converges(Duration::from_secs(120));

    // Train vector graphs with 90 days of normal data
    sim.fast_forward_training(Duration::from_days(90));

    // Inject flood event at upstream zone
    sim.inject_event(FloodEvent {
        zone: upstream_zone,
        severity: FloodSeverity::Emergency,
        soil_saturation: 0.85,
    });

    // Run simulation for 60 seconds
    sim.run_for(Duration::from_secs(60));

    // Verify: all downstream zones received preemptive alerts
    let downstream_zones = sim.watershed_downstream(upstream_zone);
    for zone in &downstream_zones {
        let alerts = sim.alerts_received_by(*zone);
        assert!(
            alerts.iter().any(|a| matches!(a, Alert::FloodPreemptive { .. })),
            "Zone {:?} did not receive flood alert within 60s", zone
        );
    }
}

#[test]
fn mesh_self_heals_after_node_failure_within_30_seconds() {
    let mut sim = Simulation::new(SimConfig {
        node_count: 20,
        zone_layout: ZoneLayout::Grid { rows: 4, cols: 5 },
        ..default()
    });

    sim.deploy_nodes();
    sim.run_until_mesh_converges(Duration::from_secs(60));

    // Kill a relay node (not an edge node)
    let relay_node = sim.find_relay_node(); // node with >2 dependents
    let dependents = sim.nodes_depending_on(relay_node);
    sim.kill_node(relay_node);

    // Run for 30 seconds
    sim.run_for(Duration::from_secs(30));

    // Verify: all dependent nodes found alternative routes
    for dep in &dependents {
        assert!(
            sim.can_reach_gateway(*dep),
            "Node {:?} lost gateway connectivity after relay failure", dep
        );
    }
}

#[test]
fn governance_vote_completes_across_partitioned_mesh() {
    let mut sim = Simulation::new(SimConfig {
        node_count: 100,
        zone_count: 10,
        ..default()
    });

    sim.deploy_nodes();
    sim.run_until_mesh_converges(Duration::from_secs(120));

    // Submit a proposal
    let proposal = sim.submit_proposal(
        zone_1,
        "Add monitoring nodes",
        GovernanceAction::AddNode { zone: zone_5, count: 10 },
        0.5, // quorum
    );

    // Create a temporary partition (simulating ice storm)
    sim.partition_zones(vec![zone_3, zone_4], Duration::from_secs(3600));

    // Zones 1,2,5,6,7,8,9,10 can vote (8 of 10 — quorum met)
    for zone in [zone_1, zone_2, zone_5, zone_6, zone_7, zone_8, zone_9, zone_10] {
        sim.cast_vote(zone, proposal.id, Vote::Yes);
    }

    // Wait for votes to propagate
    sim.run_for(Duration::from_secs(300));

    // Verify: proposal resolved even though 2 zones were partitioned
    let result = sim.proposal_status(proposal.id);
    assert_eq!(result, ProposalStatus::Passed);
}

#[test]
fn pest_detection_across_zone_boundary() {
    let mut sim = Simulation::new(SimConfig {
        node_count: 30,
        zone_count: 3,
        ..default()
    });

    sim.deploy_nodes();
    sim.run_until_mesh_converges(Duration::from_secs(60));
    sim.fast_forward_training(Duration::from_days(90));

    // Inject pest pattern that crosses zone boundary
    sim.inject_pest_spread(PestSpread {
        species: PestSpecies::EmeraldAshBorer,
        origin: zone_1_eastern_edge(),
        spread_rate_m_per_day: 50.0,
    });

    // Run for 2 simulated weeks
    sim.run_for(Duration::from_days(14));

    // Verify: both zone 1 and zone 2 detect and confirm the pest event
    assert!(sim.has_confirmed_event(zone_1, EventCategory::Pest));
    assert!(sim.has_confirmed_event(zone_2, EventCategory::Pest));

    // Verify: detection happened before visual symptoms would be apparent
    let detection_time = sim.first_detection_time(EventCategory::Pest);
    let visual_symptom_time = Duration::from_days(30); // typical for EAB
    assert!(detection_time < visual_symptom_time);
}
```

---

### Layer 4: Hardware-in-Loop Tests

**Target:** Manual, pre-merge for firmware changes
**Location:** `crates/verdant-firmware/tests/hil/`
**Tool:** `probe-rs`, `defmt`, physical ESP32-S3 dev boards

```rust
// These tests run ON REAL HARDWARE via probe-rs
// They verify the HAL implementations (the parts we can't mock)

#[test]
fn csi_capture_returns_valid_subcarrier_data() {
    // This test runs on ESP32, not in CI
    let mut csi = Esp32CsiCapture::new();
    let frame = csi.capture(5000).unwrap();

    assert!(frame.subcarrier_count() >= 52); // 20MHz channel
    assert!(frame.duration_ms() >= 4900 && frame.duration_ms() <= 5100);

    // Verify amplitude values are in valid range
    for amp in frame.amplitudes() {
        assert!(amp >= 0 && amp <= 4095); // 12-bit ADC
    }
}

#[test]
fn flash_persistence_survives_power_cycle() {
    let mut storage = Esp32FlashStorage::new();

    let test_data = vec![0xDE, 0xAD, 0xBE, 0xEF];
    storage.write_block(0x10_0000, &test_data).unwrap();

    // Read back
    let mut buf = vec![0u8; 4];
    storage.read_block(0x10_0000, &mut buf).unwrap();
    assert_eq!(buf, test_data);

    // Note: power cycle verification requires manual intervention
    // (flash the test, power cycle, flash a read-back test)
}
```

---

## 3. Test Organization by Crate

| Crate | London School | Property | Integration | HIL |
|-------|:---:|:---:|:---:|:---:|
| `verdant-core` | - | Types only | - | - |
| `verdant-vector` | Projector, Clock, Compressor | Graph capacity, scores | - | - |
| `verdant-mesh` | Radio, Link, Persistence | Routing table invariants | Sim convergence | Radio TX/RX |
| `verdant-qudag` | Crypto, Transport | DAG validity, no orphans | Sim message delivery | - |
| `verdant-safla` | Source, Sink, Healer | - | Sim consensus | - |
| `verdant-sense` | CSI HW, Sensor HW | Feature bounds | - | CSI capture, I2C |
| `verdant-firmware` | All traits | - | Sim full loop | Power, OTA |
| `verdant-gateway` | All stores, bridge | - | API integration | - |
| `verdant-robotics` | Nav, Safety, Swarm | State machine | Sim mission | Magnetometer |
| `verdant-sim` | - | - | All scenarios | - |
| `ui/` | All ports | - | Playwright E2E | - |

---

## 4. Outside-In Development Walkthrough

**Feature: "When a flood is detected upstream, downstream farmers receive alerts"**

```
Step 1: ACCEPTANCE TEST (verdant-sim)
    "flood_detected_and_downstream_alerted_within_60_seconds"
    → This test FAILS (nothing implemented)
    → Tells us we need: EventConfirmed, FloodPreemptiveAlert, WatershedGraph

Step 2: SAFLA CONSENSUS (verdant-safla)
    Mock AnomalySource, ConfirmedEventSink
    "confirms_event_when_quorum_met" — with category Flood
    → Implement ConsensusEngine.evaluate()
    → Discover we need AnomalySource.local_anomalies() and .neighbor_anomalies()

Step 3: FLOOD PROPAGATION (verdant-safla)
    Mock WatershedGraph, AlertEmitter
    "flood_handler_alerts_all_downstream_zones"
    → Implement FloodPropagationHandler.handle()
    → Discover we need WatershedGraph.downstream_neighbors()

Step 4: ANOMALY DETECTION (verdant-vector)
    Mock EmbeddingProjector, SeasonalClock
    "anomaly_score_is_high_when_embedding_far_from_seasonal_baseline"
    → Implement VectorGraph.anomaly_score()
    → Property test: "anomaly_score_bounded"

Step 5: MESH TRANSPORT (verdant-mesh)
    Mock RadioHardware, PostQuantumCrypto
    "mesh_signs_outgoing_message_and_transmits_via_radio"
    → Implement MeshCommunication.send()

Step 6: GATEWAY API (verdant-gateway)
    Mock EventStore, DashboardNotifier
    "confirmed_flood_event_pushed_to_dashboard_via_websocket"
    → Implement WebSocket push in API layer

Step 7: DASHBOARD (ui/)
    Mock MeshDataPort
    "FloodTracker renders alert when flood event received"
    → Implement FloodTracker.tsx

Step 8: RUN ACCEPTANCE TEST AGAIN
    → Now it PASSES end-to-end in simulation
```

---

## 5. Mocking Guidelines

### Do Mock

- Hardware interfaces (`CsiCapture`, `FlashStorage`, `RadioHardware`)
- Network I/O (`MeshTransport`, `WebSocket`)
- Databases (`EventStore`, `GovernanceStore`)
- Clocks and timers (`SeasonalClock`, `Clock`)
- Crypto operations (when testing protocol logic, not crypto correctness)

### Do NOT Mock

- Value objects (`Embedding`, `AnomalyScore`, `Position`) — just construct them
- Pure functions (`cosine_distance`, `compute_cost`) — just call them
- The SUT itself — never mock what you're testing
- Simple data transformations — not worth the mock overhead

### Mock Smell: If you need >5 mocks in one test

This indicates the SUT has too many collaborators. Extract a mediator or facade that aggregates interactions, then mock the facade.

```rust
// BAD: 6 mocks
fn test() {
    let mock_a = MockA::new();
    let mock_b = MockB::new();
    let mock_c = MockC::new();
    let mock_d = MockD::new();
    let mock_e = MockE::new();
    let mock_f = MockF::new();
    // ... painful setup
}

// GOOD: extract SensePhase, then mock it
pub trait SensePhase {
    fn execute(&mut self) -> Result<(CsiFrame, SensorReading), SenseError>;
}

fn test() {
    let mock_sense = MockSensePhase::new();
    let mock_communicate = MockCommunicatePhase::new();
    // 2 mocks, clear responsibilities
}
```

---

## 6. Continuous Integration Pipeline

```
┌──────────────────────────────────────────────────────┐
│  PR Pipeline                                          │
│                                                       │
│  1. cargo fmt --check              (< 5s)            │
│  2. cargo clippy -- -D warnings    (< 30s)           │
│  3. cargo test --workspace         (< 30s)           │
│     ├── London School mocks                           │
│     └── Property-based tests                          │
│  4. cargo test -p verdant-sim      (< 5min)          │
│     └── Integration/simulation                        │
│  5. npm test (ui/)                 (< 30s)           │
│     └── React Testing Library + ts-mockito            │
│  6. cargo +nightly fuzz (qudag)    (5min budget)     │
│                                                       │
│  Total: < 7 minutes                                   │
└──────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────┐
│  Nightly Pipeline                                     │
│                                                       │
│  1. All PR checks                                     │
│  2. Extended fuzz testing (1 hour)                    │
│  3. cargo build --target xtensa-esp32s3-none-elf     │
│  4. Hardware-in-loop (if ESP32 boards on CI runner)  │
│  5. Long-duration simulation (1000 nodes, 1 sim-year)│
│  6. Memory profiling (peak RAM analysis)             │
│  7. Binary size check (< 1.5MB per partition)        │
└──────────────────────────────────────────────────────┘
```
