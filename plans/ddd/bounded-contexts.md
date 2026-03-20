# Verdant Bounded Contexts

## London School TDD: Anti-Corruption Layers at Context Boundaries

Every bounded context communicates through explicit ports (traits). London School tests mock these ports to verify interaction contracts at each boundary.

---

## Context Map

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  ┌──────────────────────┐        ┌──────────────────────┐          │
│  │  ENVIRONMENTAL       │        │  MESH COMMUNICATION   │          │
│  │  INTELLIGENCE        │ ──U──► │                       │          │
│  │                      │        │  - QuDAG Protocol     │          │
│  │  - VectorGraph       │        │  - Routing            │          │
│  │  - Anomaly Detection │ ◄──D── │  - Discovery          │          │
│  │  - Seasonal Learning │        │  - Anonymity          │          │
│  │  - Pattern Deltas    │        │  - Partition Recovery  │          │
│  └──────────┬───────────┘        └──────────┬───────────┘          │
│             │                               │                       │
│             │ U = Upstream                  │                       │
│             │ D = Downstream                │                       │
│             │ CF = Conformist               │                       │
│             │ ACL = Anti-Corruption Layer   │                       │
│             │ OHS = Open Host Service       │                       │
│             │ PL = Published Language       │                       │
│             │                               │                       │
│  ┌──────────▼───────────┐        ┌──────────▼───────────┐          │
│  │  SENSOR ABSTRACTION  │        │  SELF-HEALING         │          │
│  │  (Supporting)        │        │  (SAFLA)              │          │
│  │                      │ ──CF── │                       │          │
│  │  - CSI Capture       │        │  - Health Monitoring  │          │
│  │  - Environmental     │        │  - Anomaly Consensus  │          │
│  │  - Sensor Fusion     │        │  - Pattern Propagation│          │
│  │  - HAL               │        │  - Topology Optim.    │          │
│  └──────────────────────┘        └──────────┬───────────┘          │
│                                              │                      │
│  ┌──────────────────────┐        ┌──────────▼───────────┐          │
│  │  DECENTRALIZED       │        │  AGENTIC ROBOTICS    │          │
│  │  GOVERNANCE          │ ──ACL──│  (Supporting)        │          │
│  │                      │        │                       │          │
│  │  - Proposals         │        │  - Swarm Coordination │          │
│  │  - Voting            │        │  - Navigation         │          │
│  │  - Credits           │        │  - Missions           │          │
│  │  - Zone Management   │        │  - Safety Constraints │          │
│  └──────────┬───────────┘        └──────────────────────┘          │
│             │                                                       │
│  ┌──────────▼───────────┐        ┌──────────────────────┐          │
│  │  GATEWAY BRIDGE      │ ──OHS──│  DASHBOARD UI         │          │
│  │  (Supporting)        │   PL   │  (Generic)            │          │
│  │                      │        │                       │          │
│  │  - Mesh Bridge       │        │  - Map Visualization  │          │
│  │  - Local DB          │        │  - Event Timeline     │          │
│  │  - REST/WS API       │        │  - Governance Portal  │          │
│  │  - Gateway Sync      │        │  - Farm Dashboard     │          │
│  └──────────────────────┘        │  - Alert Management   │          │
│                                  └──────────────────────┘          │
│  ┌──────────────────────┐                                          │
│  │  DATA SOVEREIGNTY    │  ── Shared Kernel (used by all) ──       │
│  │  (Cross-cutting)     │                                          │
│  │                      │                                          │
│  │  - Local<T> wrapper  │                                          │
│  │  - ZoneEncrypted<T>  │                                          │
│  │  - Consent Registry  │                                          │
│  │  - Data Classification│                                         │
│  └──────────────────────┘                                          │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Bounded Context Details

### 1. Environmental Intelligence (Core)

**Crate:** `verdant-vector`
**Owns:** VectorGraph, Embedding, SeasonalBaseline, AnomalyScore, GraphDelta
**Language:** embeddings, cosine distance, seasonal centroids, merge threshold, eviction

**Upstream of:** Mesh Communication (publishes anomaly reports and pattern deltas)
**Downstream of:** Sensor Abstraction (consumes raw features)

**Context Boundary Ports:**

```rust
// --- INBOUND PORT (from Sensor Abstraction) ---
pub trait FeatureSource {
    fn extract_features(&self, csi: &CsiFrame, sensors: &SensorReading) -> RawFeatures;
}

// --- OUTBOUND PORT (to Mesh Communication) ---
pub trait AnomalyPublisher {
    fn publish_anomaly(&mut self, report: &AnomalyReport) -> Result<(), PublishError>;
    fn publish_pattern_delta(&mut self, delta: &CompressedDelta) -> Result<(), PublishError>;
}

// --- OUTBOUND PORT (to Persistence) ---
pub trait GraphPersistence {
    fn save_graph(&mut self, graph: &VectorGraph) -> Result<(), PersistError>;
    fn load_graph(&self) -> Result<VectorGraph, PersistError>;
}
```

**London School Test at Boundary:**

```rust
#[test]
fn intelligence_publishes_anomaly_when_features_diverge_from_baseline() {
    let mut mock_features = MockFeatureSource::new();
    let mut mock_publisher = MockAnomalyPublisher::new();

    // GIVEN: feature source returns anomalous data
    mock_features.expect_extract_features()
        .returning(|_, _| RawFeatures::anomalous());

    // THEN: anomaly is published to mesh
    mock_publisher.expect_publish_anomaly()
        .withf(|report| report.score > 0.85 && report.category == EventCategory::Pest)
        .times(1)
        .returning(|_| Ok(()));

    let mut intelligence = EnvironmentalIntelligence::new(
        trained_graph(), mock_features, mock_publisher, mock_persistence, mock_clock
    );
    intelligence.process_cycle(csi_frame, sensor_reading);
}
```

---

### 2. Mesh Communication (Core)

**Crate:** `verdant-mesh`, `verdant-qudag`
**Owns:** MeshNode, RoutingTable, QuDagMessage, OnionLayer, Neighbor
**Language:** hops, TTL, DAG tips, relay, link quality, PDR, partition, epidemic

**Upstream of:** All contexts (provides transport)
**Downstream of:** Environmental Intelligence (carries anomaly data), Governance (carries proposals/votes)

**Context Boundary Ports:**

```rust
// --- INBOUND PORT (from all contexts that need to send) ---
pub trait MeshTransport {
    fn send(&mut self, frame: &MeshFrame) -> Result<(), TransportError>;
    fn receive(&mut self) -> Result<Option<MeshFrame>, TransportError>;
    fn broadcast(&mut self, frame: &MeshFrame, ttl: u8) -> Result<(), TransportError>;
}

// --- INBOUND PORT (from firmware for neighbor info) ---
pub trait NeighborRegistry {
    fn neighbors(&self) -> &[Neighbor];
    fn mark_suspect(&mut self, id: NodeId);
    fn mark_dead(&mut self, id: NodeId);
}

// --- OUTBOUND PORT (to radio hardware) ---
pub trait RadioHardware {
    fn transmit_frame(&mut self, raw: &[u8]) -> Result<(), RadioError>;
    fn receive_frame(&mut self, buf: &mut [u8]) -> Result<usize, RadioError>;
    fn scan_ble_beacons(&mut self) -> Result<Vec<BleBeacon>, RadioError>;
}

// --- OUTBOUND PORT (to crypto) ---
pub trait PostQuantumCrypto {
    fn sign(&self, data: &[u8]) -> Result<DilithiumSignature, CryptoError>;
    fn verify(&self, data: &[u8], sig: &DilithiumSignature, pk: &PublicKey) -> Result<bool, CryptoError>;
    fn encapsulate(&self, pk: &PublicKey) -> Result<(SharedSecret, Ciphertext), CryptoError>;
    fn decapsulate(&self, ct: &Ciphertext) -> Result<SharedSecret, CryptoError>;
}
```

**London School Test at Boundary:**

```rust
#[test]
fn mesh_signs_outgoing_message_and_transmits_via_radio() {
    let mut mock_crypto = MockPostQuantumCrypto::new();
    let mut mock_radio = MockRadioHardware::new();

    // THEN: message is signed with Dilithium
    mock_crypto.expect_sign()
        .times(1)
        .returning(|_| Ok(test_dilithium_sig()));

    // AND: signed frame is transmitted via radio
    mock_radio.expect_transmit_frame()
        .times(1)
        .returning(|_| Ok(()));

    let mut mesh = MeshCommunication::new(mock_crypto, mock_radio);
    mesh.send(&test_frame()).unwrap();
}
```

---

### 3. Self-Healing / SAFLA (Core)

**Crate:** `verdant-safla`
**Owns:** SaflaCycle, ConfirmedEvent, HealthAssessment, ConsensusResult
**Language:** quorum, corroboration, spatial proximity, temporal window, suspect, dead, reroute

**Relationship:** Conformist to Mesh Communication (uses its transport without transforming)
**Consumes from:** Environmental Intelligence (anomaly reports), Mesh Communication (neighbor health)
**Produces to:** Mesh Communication (confirmed events, pattern updates), Robotics (task requests)

**Context Boundary Ports:**

```rust
// --- INBOUND PORT ---
pub trait AnomalySource {
    fn local_anomalies(&self) -> Vec<Anomaly>;
    fn neighbor_anomalies(&self, window_secs: u64) -> Vec<NeighborAnomaly>;
}

// --- OUTBOUND PORT ---
pub trait ConfirmedEventSink {
    fn emit_confirmed(&mut self, event: &ConfirmedEvent) -> Result<(), EmitError>;
    fn request_robot_task(&mut self, event: &ConfirmedEvent) -> Result<(), EmitError>;
}

// --- OUTBOUND PORT ---
pub trait NetworkHealer {
    fn reroute_around(&mut self, dead: NodeId) -> Result<(), HealError>;
    fn propose_topology_change(&mut self, change: &TopologyChange) -> Result<(), HealError>;
}
```

**London School Test at Boundary:**

```rust
#[test]
fn safla_emits_confirmed_event_and_requests_robot_when_flood_consensus() {
    let mut mock_source = MockAnomalySource::new();
    let mut mock_sink = MockConfirmedEventSink::new();

    mock_source.expect_local_anomalies()
        .returning(|| vec![flood_anomaly()]);

    mock_source.expect_neighbor_anomalies()
        .returning(|_| vec![
            neighbor_flood(zone_a_n, 30),
            neighbor_flood(zone_a_s, 60),
            neighbor_flood(zone_a_e, 90),
        ]);

    // THEN: confirmed event emitted
    mock_sink.expect_emit_confirmed()
        .withf(|e| e.category == EventCategory::Flood)
        .times(1)
        .returning(|_| Ok(()));

    // AND: robot task requested for flood response
    mock_sink.expect_request_robot_task()
        .withf(|e| e.category == EventCategory::Flood)
        .times(1)
        .returning(|_| Ok(()));

    let safla = SaflaCycle::new(mock_source, mock_sink, mock_healer);
    safla.run();
}
```

---

### 4. Decentralized Governance (Core)

**Crate:** `verdant-gateway` (governance module)
**Owns:** Proposal, Vote, CreditLedger, GovernanceAction
**Language:** proposal, quorum, tally, zone vote, credits, firmware supermajority

**Relationship:** Anti-Corruption Layer to Robotics (translates governance decisions into mission parameters)
**Upstream of:** Gateway Bridge (governance state is served via API)
**Transport:** Uses Mesh Communication

**Context Boundary Ports:**

```rust
// --- INBOUND PORT (from UI via Gateway API) ---
pub trait GovernanceCommands {
    fn submit_proposal(&mut self, draft: ProposalDraft, zone: ZoneId) -> Result<ProposalHash, GovError>;
    fn cast_vote(&mut self, proposal: &ProposalHash, vote: Vote, zone: ZoneId) -> Result<(), GovError>;
}

// --- OUTBOUND PORT (to mesh for broadcast) ---
pub trait GovernanceBroadcaster {
    fn broadcast_proposal(&mut self, proposal: &Proposal) -> Result<(), BroadcastError>;
    fn broadcast_vote(&mut self, proposal_id: &ProposalHash, vote: &SignedVote) -> Result<(), BroadcastError>;
}

// --- OUTBOUND PORT (to persistence) ---
pub trait GovernancePersistence {
    fn save_proposal(&mut self, proposal: &Proposal) -> Result<(), PersistError>;
    fn load_active_proposals(&self) -> Result<Vec<Proposal>, PersistError>;
    fn save_credit_transaction(&mut self, tx: &CreditTransaction) -> Result<(), PersistError>;
}

// --- ACL PORT (to Robotics — translates governance into missions) ---
pub trait RoboticsAdapter {
    fn dispatch_firmware_update(&mut self, version: &SemVer, hash: &FirmwareHash) -> Result<(), DispatchError>;
}
```

---

### 5. Sensor Abstraction (Supporting)

**Crate:** `verdant-sense`
**Owns:** CsiFrame, SensorReading, RawFeatures
**Language:** subcarrier, amplitude, phase shift, ADC, calibration curve

**Relationship:** Conformist to Environmental Intelligence (provides data in the format intelligence expects)

```rust
// --- OUTBOUND PORT (to hardware) ---
pub trait CsiHardware {
    fn configure_csi(&mut self, config: &CsiConfig) -> Result<(), HalError>;
    fn capture_raw(&mut self, duration_ms: u32) -> Result<RawCsiBuffer, HalError>;
}

pub trait SensorHardware {
    fn read_temperature(&mut self) -> Result<i16, HalError>;   // celsius * 100
    fn read_humidity(&mut self) -> Result<u16, HalError>;      // % * 100
    fn read_soil_moisture(&mut self) -> Result<u16, HalError>; // 0-10000
    fn read_pressure(&mut self) -> Result<u32, HalError>;      // Pa
    fn read_light(&mut self) -> Result<u16, HalError>;         // lux
}
```

**London School Test:**

```rust
#[test]
fn sensor_fusion_reads_all_sensors_and_combines() {
    let mut mock_csi = MockCsiHardware::new();
    let mut mock_sensors = MockSensorHardware::new();

    mock_csi.expect_capture_raw()
        .with(eq(5000))
        .times(1)
        .returning(|_| Ok(RawCsiBuffer::test_normal()));

    mock_sensors.expect_read_temperature().times(1).returning(|| Ok(2150));
    mock_sensors.expect_read_humidity().times(1).returning(|| Ok(6500));
    mock_sensors.expect_read_soil_moisture().times(1).returning(|| Ok(4200));
    mock_sensors.expect_read_pressure().times(1).returning(|| Ok(101325));
    mock_sensors.expect_read_light().times(1).returning(|| Ok(850));

    let fusion = SensorFusion::new(mock_csi, mock_sensors);
    let features = fusion.capture_and_fuse(5000).unwrap();

    assert_eq!(features.dimension(), FEATURE_DIM);
}
```

---

### 6. Agentic Robotics (Supporting)

**Crate:** `verdant-robotics`
**Owns:** RobotAgent, Mission, SwarmTask, SafetyConfig, MagneticFix
**Language:** mission, waypoint, abort, geofence, swarm, magnetometer fix

**Receives from:** SAFLA (confirmed events that need physical response)
**Anti-Corruption Layer from:** Governance (firmware updates dispatched as robot missions)

---

### 7. Gateway Bridge (Supporting)

**Crate:** `verdant-gateway`
**Owns:** GatewayState, SyncState, ApiEndpoints
**Language:** bridge, sync, CRDT merge, WebSocket subscription

**Relationship:** Open Host Service with Published Language to Dashboard UI
**Published Language:** JSON over REST/WebSocket (OpenAPI spec)

```rust
// --- INBOUND PORT (from mesh) ---
pub trait MeshBridge {
    fn poll_mesh_events(&mut self) -> Result<Vec<MeshEvent>, BridgeError>;
    fn send_to_mesh(&mut self, msg: MeshFrame) -> Result<(), BridgeError>;
}

// --- OUTBOUND PORT (to dashboard via WebSocket) ---
pub trait DashboardNotifier {
    fn notify_event(&self, event: &ConfirmedEvent) -> Result<(), NotifyError>;
    fn notify_node_status(&self, status: &NodeStatus) -> Result<(), NotifyError>;
}
```

---

### 8. Dashboard UI (Generic)

**Directory:** `ui/`
**Owns:** UI components, offline cache, WASM bridge
**Language:** component, hook, sync, offline-first, PWA

**Relationship:** Conformist to Gateway Bridge's Published Language (JSON API)
**WASM Bridge:** Compiles `verdant-qudag` crypto verification to WASM for in-browser vote verification

---

### 9. Data Sovereignty (Shared Kernel)

**Crate:** `verdant-core` (sovereignty module)
**Owns:** `Local<T>`, `ZoneEncrypted<T>`, DataTier, ConsentRegistry
**Language:** tier, local, zone-shared, mesh-wide, consent, sovereignty

This is a **Shared Kernel** used by all bounded contexts. It defines the type-level constraints that enforce data sovereignty rules at compile time.

```rust
// Shared Kernel types — used by all contexts
pub struct Local<T>(T);       // Tier 1: never serializable for mesh
pub struct ZoneEncrypted<T> { // Tier 2: requires zone key
    inner: T,
    zone: ZoneId,
}
pub struct MeshPublic<T>(T);  // Tier 3: freely transmittable

pub enum DataTier { Local, ZoneShared, MeshWide }
```

---

## Context Relationship Summary

| Upstream | Downstream | Relationship | Integration Pattern |
|----------|-----------|-------------|-------------------|
| Environmental Intelligence | Mesh Communication | Customer-Supplier | AnomalyPublisher trait |
| Sensor Abstraction | Environmental Intelligence | Conformist | FeatureSource trait |
| Mesh Communication | All contexts | Open Host Service | MeshTransport trait |
| SAFLA | Mesh Communication | Conformist | Uses MeshTransport directly |
| SAFLA | Agentic Robotics | Customer-Supplier | ConfirmedEventSink trait |
| Governance | Agentic Robotics | ACL | RoboticsAdapter trait |
| Gateway Bridge | Dashboard UI | OHS + Published Language | JSON REST/WS API |
| Data Sovereignty | All contexts | Shared Kernel | `Local<T>`, `ZoneEncrypted<T>` types |

---

## Anti-Corruption Layer Examples

### Governance → Robotics ACL

Governance speaks in proposals and actions. Robotics speaks in missions and waypoints. The ACL translates:

```rust
pub struct GovernanceToRoboticsAdapter {
    swarm: Box<dyn SwarmCoordinator>,
}

impl RoboticsAdapter for GovernanceToRoboticsAdapter {
    fn dispatch_firmware_update(&mut self, version: &SemVer, hash: &FirmwareHash) -> Result<(), DispatchError> {
        // Translate governance action into a robot mission
        let mission = Mission {
            mission_type: MissionType::EmergencyRelay, // use relay mission to carry firmware
            payload: MissionPayload::FirmwareUpdate {
                version: version.clone(),
                hash: hash.clone(),
            },
            // target: each zone gateway
            ..Mission::default()
        };
        self.swarm.request_task(&ConfirmedEvent::firmware_update(version))?;
        Ok(())
    }
}
```

**London School Test for ACL:**

```rust
#[test]
fn governance_acl_translates_firmware_update_to_robot_mission() {
    let mut mock_swarm = MockSwarmCoordinator::new();

    // THEN: swarm receives a task request framed as a confirmed event
    mock_swarm.expect_request_task()
        .withf(|event| matches!(event, ConfirmedEvent { category: EventCategory::FirmwareUpdate, .. }))
        .times(1)
        .returning(|_| Ok(TaskAssignment::new(robot_1)));

    let mut adapter = GovernanceToRoboticsAdapter::new(mock_swarm);
    adapter.dispatch_firmware_update(&SemVer::new(2, 1, 0), &firmware_hash()).unwrap();
}
```
