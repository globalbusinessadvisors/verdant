# Verdant Domain Model

## Domain-Driven Design Specification

> London School TDD: all aggregate interactions are tested by mocking collaborating aggregates/ports. Pure domain logic uses property-based tests.

---

## Ubiquitous Language

| Term | Definition |
|------|-----------|
| **Node** | A physical ESP32 device deployed in the field. Senses, learns, communicates. |
| **Zone** | A group of 10-50 nodes under a single landowner's stewardship. The atomic unit of governance. |
| **Mesh** | The self-organizing network of all nodes and zones. No central coordinator. |
| **Embedding** | A fixed-dimension vector representing the environmental state at a moment in time. |
| **Vector Graph** | A graph of embedding nodes with similarity edges, learned incrementally from observations. |
| **Seasonal Baseline** | The centroid embedding for a specific week-of-year, representing "normal" for that season. |
| **Anomaly** | A deviation from the seasonal baseline, scored by cosine distance. |
| **Confirmed Event** | An anomaly corroborated by spatial-temporal consensus among neighboring nodes. |
| **Pattern Delta** | A compressed diff of vector graph changes, propagated to teach the mesh new signatures. |
| **QuDAG Message** | A message in the DAG-structured, quantum-resistant mesh protocol. |
| **Onion Layer** | One encryption/decryption hop in the anonymous relay path. |
| **SAFLA Cycle** | One execution of the Self-Aware Feedback Loop: health → consensus → propagation → topology. |
| **Proposal** | A governance action proposed by a zone for collective voting. |
| **Credit** | A non-tradeable ledger entry awarded for sharing environmental data. |
| **Gateway** | A Raspberry Pi that bridges the mesh to a local network and serves the dashboard. |
| **Mission** | A task assigned to a robot: navigate, execute action, return. |
| **Presence Reading** | A local-only (never transmitted) detection of human activity via WiFi CSI. |
| **Data Tier** | Classification of data sovereignty: Local (property-only), Zone (encrypted, opt-in), Mesh-Wide (anonymous). |

---

## Core Domain vs. Supporting vs. Generic

| Subdomain | Classification | Rationale |
|-----------|---------------|-----------|
| Environmental Intelligence (VectorGraph, Anomaly, SAFLA) | **Core** | This is Verdant's competitive advantage — self-learning environmental awareness |
| Mesh Communication (QuDAG, Routing, Discovery) | **Core** | Quantum-resistant, anonymous, decentralized — not available off-shelf |
| Governance (DAA, Proposals, Voting) | **Core** | On-mesh decentralized governance is novel |
| Sensor Abstraction (CSI, Environmental) | **Supporting** | Necessary but not differentiating; wraps commodity hardware |
| Robotics (Navigation, Swarm, Missions) | **Supporting** | Important capability but secondary to mesh intelligence |
| Gateway API | **Supporting** | Standard REST/WS bridge; enabling but not novel |
| Dashboard UI | **Generic** | Standard React PWA; off-shelf patterns |
| Power Management | **Supporting** | Critical for field operation but not domain-specific |
| Persistence (Flash, redb) | **Generic** | Standard storage; behind trait boundaries |

---

## Aggregates, Entities, and Value Objects

### Aggregate: `EnvironmentalModel`

**Root Entity:** `VectorGraph`
**Bounded Context:** Environmental Intelligence
**Invariants:**
- Graph never exceeds `MAX_GRAPH_NODES` (2048)
- Every graph node has at least 1 edge (connectivity)
- Seasonal centroids cover all 52 week slots after 1 year of operation
- Version is monotonically increasing

```rust
// Aggregate root
pub struct VectorGraph {
    nodes: BoundedVec<GraphNode, MAX_GRAPH_NODES>,  // Entity
    edges: SparseMatrix<f32>,                        // Value Object
    seasonal_centroids: SeasonalBaselines,           // Entity
    version: Version,                                // Value Object
}

// Entity within aggregate (identity = index in graph)
pub struct GraphNode {
    id: GraphNodeId,          // identity
    embedding: Embedding,     // Value Object
    visit_count: u32,
    last_modified: Version,
}

// Value Objects (identity-less, compared by value)
pub struct Embedding(FixedVec<i16, EMBEDDING_DIM>);
pub struct SeasonSlot { week: u8, year_offset: u8 }
pub struct AnomalyScore(f32);
pub struct Version(u64);

// Entity within aggregate
pub struct SeasonalBaselines {
    centroids: HashMap<SeasonSlot, RunningMean>,
    observation_counts: HashMap<SeasonSlot, u32>,
}
```

**London School Tests:**

```rust
#[cfg(test)]
mod tests {
    // VectorGraph is mostly pure, so we test state.
    // But its interactions with EmbeddingProjector and SeasonalClock are mocked.

    #[test]
    fn encode_delegates_to_projector_then_stores() {
        let mut mock_projector = MockEmbeddingProjector::new();
        let mut mock_clock = MockSeasonalClock::new();

        mock_projector.expect_project()
            .times(1)
            .returning(|_| Embedding::new(&[1, 0, 0]));

        mock_clock.expect_current_slot()
            .returning(|| SeasonSlot::new(3, 12));

        let mut graph = VectorGraph::new(2048);
        let raw = RawFeatures::test_data();

        graph.encode_and_update(raw, &mock_projector, &mock_clock);

        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.version().0, 1);
    }
}
```

---

### Aggregate: `MeshNode`

**Root Entity:** `MeshNode`
**Bounded Context:** Mesh Communication
**Invariants:**
- Node always has a valid `NodeId` (derived from hardware eFuse)
- Routing table is consistent (no routes to self, no negative costs)
- DAG tip set is bounded (max 8 tips tracked)

```rust
// Aggregate root
pub struct MeshNode {
    id: NodeId,                           // Value Object (hardware-derived)
    routing_table: RoutingTable,          // Entity
    dag_state: DagState,                  // Entity
    neighbors: BoundedVec<Neighbor, 256>, // Entity collection
}

// Entity (identity = neighbor's NodeId)
pub struct Neighbor {
    id: NodeId,
    link_quality: LinkQuality,            // Value Object
    last_seen: Timestamp,                 // Value Object
    status: NeighborStatus,               // Value Object (enum)
}

// Value Objects
pub struct NodeId([u8; 8]);
pub struct LinkQuality { pdr: f32, rtt_ms: u32, cost: f32 }
pub struct Timestamp(u64);
pub enum NeighborStatus { Active, Suspect, Dead }

// Entity
pub struct RoutingTable {
    routes: HashMap<NodeId, Route>,       // Entity collection
}

pub struct Route {
    destination: NodeId,
    next_hop: NodeId,
    cost: f32,
    hops: u8,
    last_updated: Timestamp,
}

// Entity
pub struct DagState {
    tips: BoundedVec<MessageHash, 8>,
    seen_hashes: BloomFilter,             // Value Object (probabilistic seen-set)
    sequence: u64,
}
```

**London School Tests:**

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn mesh_node_updates_routing_and_notifies_transport() {
        let mut mock_transport = MockMeshTransport::new();
        let mut mock_persistence = MockRoutingTablePersistence::new();

        // WHEN: receiving a routing update from neighbor
        let update = RoutingUpdate {
            from: node_a,
            destination: gateway,
            cost: 2.0,
            hops: 3,
        };

        // THEN: node rebroadcasts its updated route to other neighbors
        mock_transport.expect_send()
            .withf(|msg| matches!(msg.payload, Payload::RoutingUpdate(_)))
            .times(1)
            .returning(|_| Ok(()));

        let mut mesh_node = MeshNode::new(self_id, mock_transport, mock_persistence);
        mesh_node.handle_routing_update(update);
    }

    #[test]
    fn mesh_node_rejects_route_to_self() {
        let mut mock_transport = MockMeshTransport::new();
        // THEN: no broadcast occurs
        mock_transport.expect_send().times(0);

        let mut mesh_node = MeshNode::new(self_id, mock_transport, mock_persistence);
        let update = RoutingUpdate {
            from: node_a,
            destination: self_id, // route to ourself — invalid
            cost: 1.0,
            hops: 1,
        };

        let result = mesh_node.handle_routing_update(update);
        assert!(result.is_err());
    }
}
```

---

### Aggregate: `GovernanceState`

**Root Entity:** `GovernanceState`
**Bounded Context:** Decentralized Governance
**Invariants:**
- A zone can only vote once per proposal
- Proposals have immutable deadlines (no extensions)
- Governance actions execute exactly once upon passing
- Credits cannot go negative

```rust
// Aggregate root
pub struct GovernanceState {
    proposals: HashMap<ProposalHash, Proposal>,  // Entity collection
    credit_ledger: CreditLedger,                 // Entity
    zone_keys: HashMap<ZoneId, PublicKey>,        // Value Object map
}

// Entity (identity = ProposalHash)
pub struct Proposal {
    id: ProposalHash,                  // Value Object (content-addressed)
    proposer: ZoneId,                  // Value Object
    title: BoundedString<128>,         // Value Object
    action: GovernanceAction,          // Value Object (enum)
    quorum: Quorum,                    // Value Object
    voting_deadline: Timestamp,        // Value Object
    votes: HashMap<ZoneId, SignedVote>, // Value Object map
    status: ProposalStatus,            // Value Object (enum)
}

// Value Objects
pub struct ProposalHash([u8; 32]);
pub struct ZoneId([u8; 4]);
pub struct Quorum(f32);  // invariant: 0.0 < quorum <= 1.0
pub struct SignedVote { vote: Vote, signature: DilithiumSignature }
pub enum Vote { Yes, No, Abstain }
pub enum ProposalStatus { Active, Passed, Rejected, FailedQuorum }
pub enum GovernanceAction {
    AddNode { zone: ZoneId, count: u16 },
    RemoveNode { zone: ZoneId, node: NodeId },
    UpdatePolicy { key: String, value: String },
    AllocateCredits { zone: ZoneId, amount: u64 },
    FirmwareUpdate { version: SemVer, hash: FirmwareHash },
}

// Entity
pub struct CreditLedger {
    balances: HashMap<ZoneId, u64>,
    history: Vec<CreditTransaction>,
}
```

---

### Aggregate: `RobotAgent`

**Root Entity:** `RobotAgent`
**Bounded Context:** Agentic Robotics
**Invariants:**
- Robot can have at most 1 active mission
- Safety constraints override all other state transitions
- Battery level must be above minimum before accepting mission
- State machine transitions are deterministic

```rust
// Aggregate root
pub struct RobotAgent {
    id: RobotId,                       // Value Object
    state: RobotState,                 // Value Object (state machine)
    active_mission: Option<Mission>,   // Entity (at most one)
    position: Position,                // Value Object
    battery: BatteryLevel,             // Value Object
    safety_config: SafetyConfig,       // Value Object
}

// Value Objects
pub struct RobotId([u8; 8]);
pub struct Position { lat: f64, lon: f64, alt_m: f32 }
pub struct BatteryLevel(f32);  // 0.0 to 1.0

pub enum RobotState {
    Idle,
    Tasked,
    Navigating,
    Executing,
    Returning,
    SafetyAbort,
}

// Entity (identity = MissionId)
pub struct Mission {
    id: MissionId,
    mission_type: MissionType,
    target: Position,
    payload: MissionPayload,
    assigned_at: Timestamp,
    status: MissionStatus,
}

pub enum MissionType {
    SeedDispersal,
    WaterSensorDeploy,
    SupplyDelivery,
    VisualInspection,
    EmergencyRelay,
}
```

---

### Aggregate: `NodeFirmware` (Orchestrating Aggregate)

**Root Entity:** `NodeFirmware`
**Bounded Context:** Firmware Orchestration
**Invariants:**
- Main loop phases execute in order: Sense → Learn → Detect → Communicate → Heal → Sleep
- Power state is checked every cycle
- Checkpoint occurs at most once per hour

```rust
// This is the top-level orchestrating aggregate that wires everything together.
// It depends on traits (ports), not concrete implementations.
pub struct NodeFirmware<C, S, T, F, P>
where
    C: CsiCapture,
    S: EnvironmentalSensor,
    T: MeshTransport,
    F: FlashStorage,
    P: PowerMonitor,
{
    csi: C,
    sensor: S,
    transport: T,
    storage: F,
    power: P,
    vector_graph: VectorGraph,
    mesh_node: MeshNode,
    safla: SaflaCycle,
    last_checkpoint: Timestamp,
}

impl<C, S, T, F, P> NodeFirmware<C, S, T, F, P>
where
    C: CsiCapture,
    S: EnvironmentalSensor,
    T: MeshTransport,
    F: FlashStorage,
    P: PowerMonitor,
{
    pub fn run_cycle(&mut self) -> CycleResult {
        // Phase 1: Sense
        let csi_data = self.csi.capture(CSI_SAMPLE_WINDOW_MS)?;
        let sensor_reading = self.sensor.read()?;

        // Phase 2: Learn
        let embedding = self.vector_graph.encode_and_update(
            csi_data, sensor_reading, &self.clock
        );

        // Phase 3: Detect
        let anomaly_score = self.vector_graph.anomaly_score(&embedding, &self.clock);
        if anomaly_score > ANOMALY_THRESHOLD {
            let event = self.classify(embedding, anomaly_score);
            self.transport.send(&event.to_mesh_frame())?;
        }

        // Phase 4: Communicate
        while let Some(msg) = self.transport.receive()? {
            self.mesh_node.handle_message(msg)?;
        }

        // Phase 5: Self-heal
        self.safla.run_cycle(&mut self.mesh_node, &self.vector_graph)?;

        // Phase 6: Power + Checkpoint
        self.maybe_checkpoint()?;
        Ok(self.power.sleep_duration())
    }
}
```

**London School Tests (Orchestration):**

```rust
#[cfg(test)]
mod tests {
    // This is the OUTER test — the acceptance test that drives outside-in development.
    // All collaborators are mocked. We verify the orchestration.

    #[test]
    fn full_cycle_sense_learn_detect_communicate_heal() {
        let mut mock_csi = MockCsiCapture::new();
        let mut mock_sensor = MockEnvironmentalSensor::new();
        let mut mock_transport = MockMeshTransport::new();
        let mut mock_storage = MockFlashStorage::new();
        let mut mock_power = MockPowerMonitor::new();

        // Sense: CSI and sensors are read
        mock_csi.expect_capture()
            .with(eq(5000))
            .times(1)
            .returning(|_| Ok(CsiFrame::normal()));

        mock_sensor.expect_read()
            .times(1)
            .returning(|| Ok(SensorReading::normal()));

        // Communicate: incoming messages are drained
        mock_transport.expect_receive()
            .times(1)
            .returning(|| Ok(None)); // no incoming

        // No anomaly => no broadcast (normal reading)
        mock_transport.expect_send().times(0);

        // Power: returns normal sleep duration
        mock_power.expect_sleep_duration()
            .returning(|| Duration::from_millis(30_000));

        mock_power.expect_battery_level()
            .returning(|| BatteryLevel(0.8));

        let mut firmware = NodeFirmware::new(
            mock_csi, mock_sensor, mock_transport, mock_storage, mock_power
        );
        let result = firmware.run_cycle();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Duration::from_millis(30_000));
    }

    #[test]
    fn cycle_broadcasts_anomaly_when_detected() {
        let mut mock_csi = MockCsiCapture::new();
        let mut mock_sensor = MockEnvironmentalSensor::new();
        let mut mock_transport = MockMeshTransport::new();

        // Sense: CSI returns anomalous pattern
        mock_csi.expect_capture()
            .returning(|_| Ok(CsiFrame::anomalous()));

        mock_sensor.expect_read()
            .returning(|| Ok(SensorReading::with_spike()));

        // THEN: anomaly is broadcast
        mock_transport.expect_send()
            .withf(|msg| matches!(msg.payload, Payload::AnomalyReport(_)))
            .times(1)
            .returning(|_| Ok(()));

        mock_transport.expect_receive()
            .returning(|| Ok(None));

        let mut firmware = NodeFirmware::new(
            mock_csi, mock_sensor, mock_transport, mock_storage, mock_power
        );
        firmware.seed_trained_graph(); // pre-load a trained graph so anomaly detection works
        firmware.run_cycle().unwrap();
    }

    #[test]
    fn cycle_enters_deep_sleep_on_low_battery() {
        let mut mock_power = MockPowerMonitor::new();

        mock_power.expect_battery_level()
            .returning(|| BatteryLevel(0.05)); // critically low

        // THEN: sleep duration is extended
        mock_power.expect_sleep_duration()
            .returning(|| Duration::from_secs(300)); // 5 minutes instead of 30s

        let mut firmware = NodeFirmware::new(
            mock_csi, mock_sensor, mock_transport, mock_storage, mock_power
        );
        let duration = firmware.run_cycle().unwrap();

        assert_eq!(duration, Duration::from_secs(300));
    }
}
```
