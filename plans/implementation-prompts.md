# Verdant Implementation Prompts

## Dependency-Ordered Build Sequence

Each prompt is self-contained: it specifies what to build, what must already exist, the exact files to create, the domain types and traits to implement, and the London School TDD tests to write. Prompts are ordered so that every dependency is satisfied by a prior prompt.

```
Dependency Graph:

  ┌─────────────┐
  │ 01: core     │ ◄── everything depends on this
  └──┬───┬───┬──┘
     │   │   │
     ▼   │   ▼
┌────┐   │  ┌────────┐
│ 02 │   │  │ 03     │
│qudag│   │  │vector  │     ┌──────┐
└──┬─┘   │  └──┬─────┘     │ 05   │
   │     │     │            │sense │
   ▼     │     │            └──┬───┘
┌────┐   │     │               │
│ 04 │◄──┘     │               │
│mesh│         │               │
└──┬─┘         │               │
   │     ┌─────▼───┐           │
   ├────►│ 06      │◄──────────┘
   │     │ safla   │
   │     └────┬────┘
   │          │
   │     ┌────▼────┐
   ├────►│ 07      │
   │     │  sim    │
   │     └─────────┘
   │
   │     ┌─────────┐
   ├────►│ 08      │
   │     │firmware │
   │     └─────────┘
   │
   │     ┌─────────┐
   ├────►│ 09      │
   │     │robotics │
   │     └─────────┘
   │
   │     ┌─────────┐    ┌──────────┐    ┌──────────┐
   └────►│ 10      │───►│ 11       │───►│ 12       │
         │gateway  │    │ UI core  │    │ UI feat  │
         └─────────┘    └──────────┘    └──────────┘
                                             │
                                        ┌────▼─────┐
                                        │ 13 WASM  │
                                        └──────────┘

         ┌──────────┐    ┌──────────┐
         │ 14 integ │    │ 15 deploy│
         └──────────┘    └──────────┘
```

---

## Prompt 01: Workspace Scaffold and `verdant-core`

### Prerequisites
None. This is the foundation.

### Goal
Create the Rust workspace root and the `verdant-core` crate containing all shared types, traits, error types, configuration constants, and the data sovereignty shared kernel. Every other crate in the workspace depends on this. It must be `no_std`-compatible with an optional `std` feature flag.

### What to Build

**1. Workspace root `Cargo.toml`:**
Set up a Cargo workspace with `resolver = "2"`. Define the `crates/*` members list (all 10 crates). Do not add crate-specific dependencies here — each crate manages its own.

**2. `crates/verdant-core/Cargo.toml`:**
- `no_std` by default, `std` feature gates `std`-dependent code
- Dependencies: `serde` (with `derive`, `no_std`), `heapless` (for bounded collections), `postcard` (binary serialization), `defmt` (conditional on feature `defmt`)

**3. `crates/verdant-core/src/lib.rs`:**
Re-export all modules.

**4. `crates/verdant-core/src/types.rs` — Identity and Primitive Types:**
```rust
// All value objects from the DDD domain model:
pub struct NodeId(pub [u8; 8]);        // hardware-derived, eFuse
pub struct ZoneId(pub [u8; 4]);        // zone identifier
pub struct Timestamp(pub u64);         // monotonic milliseconds
pub struct Version(pub u64);           // monotonically increasing graph version
pub struct MessageHash(pub [u8; 32]);  // SHA-256 of QuDAG message
pub struct ProposalHash(pub [u8; 32]); // content-addressed proposal ID
pub struct FirmwareHash(pub [u8; 32]); // firmware image hash
pub struct SemVer { pub major: u8, pub minor: u8, pub patch: u8 }

// Bounded string for no_std governance titles
pub struct BoundedString<const N: usize>(heapless::String<N>);

// Event categories (from domain-events.md)
pub enum EventCategory {
    Pest { species_hint: Option<PestSpecies> },
    Flood { severity: FloodSeverity, upstream_origin: Option<ZoneId> },
    Fire { smoke_density: f32 },
    Infrastructure { sub_type: InfrastructureType },
    Wildlife { movement_type: MovementType },
    Climate { sub_type: ClimateType },
}

pub enum FloodSeverity { Watch, Warning, Emergency }
pub enum PestSpecies { EmeraldAshBorer, HemlockWoollyAdelgid, BeechLeafDisease, Unknown }
pub enum InfrastructureType { PipeFreeze, RoadWashout, PowerLine, BridgeDamage }
pub enum MovementType { Migration, Corridor, Unusual }
pub enum ClimateType { FreezeThaw, Drought, ExtremeHeat, IceStorm }
```

Implement `Eq`, `PartialEq`, `Clone`, `Hash`, `Debug`, `Serialize`, `Deserialize` for all types as appropriate. `NodeId`, `ZoneId`, `MessageHash`, `ProposalHash` should implement `Copy`.

**5. `crates/verdant-core/src/traits.rs` — Port Traits (Bounded Context Boundaries):**

These are the trait definitions from the DDD bounded-contexts document. They define the contracts between crates. Every trait must be `no_std`-compatible (no `Box<dyn>`, use generics or associated types).

```rust
// --- Sensor Abstraction Ports ---
pub trait CsiCapture {
    fn capture(&mut self, duration_ms: u32) -> Result<CsiFrame, SenseError>;
}

pub trait EnvironmentalSensor {
    fn read(&mut self) -> Result<SensorReading, SenseError>;
}

// --- Mesh Communication Ports ---
pub trait MeshTransport {
    fn send(&mut self, frame: &MeshFrame) -> Result<(), TransportError>;
    fn receive(&mut self) -> Result<Option<MeshFrame>, TransportError>;
    fn broadcast(&mut self, frame: &MeshFrame, ttl: u8) -> Result<(), TransportError>;
}

pub trait RadioHardware {
    fn transmit_frame(&mut self, raw: &[u8]) -> Result<(), RadioError>;
    fn receive_frame(&mut self, buf: &mut [u8]) -> Result<usize, RadioError>;
    fn scan_ble_beacons(&mut self) -> Result<heapless::Vec<BleBeacon, 32>, RadioError>;
}

// --- Crypto Ports ---
pub trait PostQuantumCrypto {
    fn sign(&self, data: &[u8]) -> Result<DilithiumSignature, CryptoError>;
    fn verify(&self, data: &[u8], sig: &DilithiumSignature, pk: &PublicKey) -> Result<bool, CryptoError>;
    fn encapsulate(&self, pk: &PublicKey) -> Result<(SharedSecret, Ciphertext), CryptoError>;
    fn decapsulate(&self, ct: &Ciphertext) -> Result<SharedSecret, CryptoError>;
}

// --- Storage Ports ---
pub trait FlashStorage {
    fn read_block(&self, addr: u32, buf: &mut [u8]) -> Result<(), StorageError>;
    fn write_block(&mut self, addr: u32, data: &[u8]) -> Result<(), StorageError>;
}

// --- Power Ports ---
pub trait PowerMonitor {
    fn battery_level(&self) -> BatteryLevel;
    fn solar_output_mw(&self) -> u16;
    fn sleep_duration(&self) -> u32; // milliseconds
}

// --- Intelligence Ports ---
pub trait EmbeddingProjector {
    fn project(&self, raw_features: &RawFeatures) -> Embedding;
}

pub trait SeasonalClock {
    fn current_slot(&self) -> SeasonSlot;
}

// --- SAFLA Ports ---
pub trait AnomalySource {
    fn local_anomalies(&self) -> heapless::Vec<Anomaly, 16>;
    fn neighbor_anomalies(&self, window_secs: u64) -> heapless::Vec<NeighborAnomaly, 64>;
}

pub trait ConfirmedEventSink {
    fn emit_confirmed(&mut self, event: &ConfirmedEvent) -> Result<(), EmitError>;
    fn request_robot_task(&mut self, event: &ConfirmedEvent) -> Result<(), EmitError>;
}

pub trait NetworkHealer {
    fn reroute_around(&mut self, dead: NodeId) -> Result<(), HealError>;
    fn propose_topology_change(&mut self, change: &TopologyChange) -> Result<(), HealError>;
}
```

Define the associated data types referenced by traits: `CsiFrame`, `SensorReading`, `MeshFrame`, `BleBeacon`, `DilithiumSignature`, `PublicKey`, `SharedSecret`, `Ciphertext`, `BatteryLevel`, `Embedding`, `SeasonSlot`, `RawFeatures`, `Anomaly`, `NeighborAnomaly`, `ConfirmedEvent`, `TopologyChange`. Use `heapless` vectors and fixed-size arrays for all collection types to maintain `no_std` compatibility.

**6. `crates/verdant-core/src/sovereignty.rs` — Data Sovereignty Shared Kernel:**
```rust
/// Tier 1: Property-local data. NEVER leaves the node.
/// This type deliberately does NOT implement Serialize for MeshFrame.
pub struct Local<T>(T);

/// Tier 2: Zone-shared data. Encrypted with zone key. Opt-in.
pub struct ZoneEncrypted<T> {
    pub inner: T,
    pub zone: ZoneId,
}

/// Tier 3: Mesh-wide anonymous data. Freely transmittable.
pub struct MeshPublic<T>(pub T);

pub enum DataTier { Local, ZoneShared, MeshWide }

pub trait DataClassifier {
    fn classify(&self) -> DataTier;
}
```

`Local<T>` must NOT implement `serde::Serialize`. This is the compile-time privacy guarantee. Write a `trybuild` negative compile test to verify this.

**7. `crates/verdant-core/src/error.rs` — Unified Error Types:**
```rust
pub enum VerdantError {
    Sense(SenseError),
    Transport(TransportError),
    Crypto(CryptoError),
    Storage(StorageError),
    Radio(RadioError),
    Governance(GovernanceError),
    Emit(EmitError),
    Heal(HealError),
}
// Plus individual error enums for each domain
```

**8. `crates/verdant-core/src/config.rs` — Compile-Time Constants:**
```rust
pub const DUTY_CYCLE_MS: u32 = 30_000;
pub const CSI_SAMPLE_WINDOW_MS: u32 = 5_000;
pub const ANOMALY_THRESHOLD: f32 = 0.85;
pub const MAX_GRAPH_NODES: usize = 2048;
pub const EMBEDDING_DIM: usize = 32;
pub const PATTERN_PROPAGATION_TTL: u8 = 16;
pub const CONSENSUS_QUORUM: u8 = 3;
pub const TEMPORAL_WINDOW_SECS: u64 = 300;
pub const MAX_NEIGHBORS: usize = 256;
pub const MAX_DAG_TIPS: usize = 8;
pub const CHECKPOINT_INTERVAL_SECS: u64 = 3600;
```

### London School TDD Tests

Since `verdant-core` is mostly types and traits (no business logic), tests are minimal:
- Unit tests for `BoundedString` construction and overflow
- Unit tests for `Timestamp` ordering, `Version` monotonicity
- `trybuild` negative compile test: `Local<SensorReading>` does not implement mesh serialization
- Property test: `NodeId`, `ZoneId` round-trip through `postcard` serialization

### Acceptance Criteria
- `cargo build -p verdant-core` succeeds with `default-features = false` (no_std)
- `cargo build -p verdant-core --features std` succeeds
- `cargo test -p verdant-core` passes
- All types implement `Serialize`/`Deserialize` via postcard
- `Local<T>` provably cannot be serialized for mesh transport (trybuild test)

---

## Prompt 02: `verdant-qudag` — Post-Quantum DAG Protocol

### Prerequisites
Prompt 01 (`verdant-core`) must be complete.

### Goal
Implement the QuDAG protocol: DAG-structured message ordering with CRYSTALS-Dilithium signatures, CRYSTALS-Kyber key encapsulation, and onion-layer anonymous routing. This crate provides the cryptographic and message-ordering foundation that the mesh layer builds on.

### What to Build

**1. `crates/verdant-qudag/Cargo.toml`:**
- Depends on `verdant-core`
- Dependencies: `pqcrypto-dilithium`, `pqcrypto-kyber`, `postcard`, `serde`, `heapless`
- Feature `std` for testing helpers; default is `no_std`
- Dev dependencies: `mockall`, `proptest`

**2. `crates/verdant-qudag/src/crypto.rs` — Post-Quantum Crypto Wrapper:**
Implement `PostQuantumCrypto` trait from `verdant-core` using `pqcrypto-dilithium` (FIPS 204 — ML-DSA) and `pqcrypto-kyber` (FIPS 203 — ML-KEM). Wrap the FFI calls in safe Rust. Provide:
- `DilithiumKeyPair::generate()` → `(PublicKey, SecretKey)`
- `sign(data, secret_key)` → `DilithiumSignature`
- `verify(data, signature, public_key)` → `bool`
- `KyberKeyPair::generate()` → `(PublicKey, SecretKey)`
- `encapsulate(public_key)` → `(SharedSecret, Ciphertext)`
- `decapsulate(ciphertext, secret_key)` → `SharedSecret`

**3. `crates/verdant-qudag/src/dag.rs` — DAG State Machine:**
Implement the `DagState` entity from the domain model:
```rust
pub struct DagState {
    tips: heapless::Vec<MessageHash, MAX_DAG_TIPS>,
    seen: BloomFilter,  // probabilistic seen-set, ~2KB
    sequence: u64,
}

impl DagState {
    pub fn new() -> Self;
    pub fn genesis_hash() -> MessageHash;
    pub fn validate_parents(&self, parents: &[MessageHash]) -> Result<(), DagError>;
    pub fn insert(&mut self, hash: MessageHash, parents: &[MessageHash]) -> Result<(), DagError>;
    pub fn current_tips(&self, max: usize) -> heapless::Vec<MessageHash, MAX_DAG_TIPS>;
    pub fn is_seen(&self, hash: &MessageHash) -> bool;
}
```
Invariants enforced:
- All parent hashes must be seen (no orphans)
- Tip set bounded to `MAX_DAG_TIPS`
- Seen set uses bloom filter with configurable false positive rate

**4. `crates/verdant-qudag/src/message.rs` — Message Types:**
```rust
#[derive(Serialize, Deserialize)]
pub struct QuDagMessage {
    pub dag_parents: heapless::Vec<MessageHash, 3>,
    pub payload: EncryptedPayload,
    pub signature: DilithiumSignature,
    pub ttl: u8,
    pub timestamp: u64,
}

pub struct EncryptedPayload {
    pub ciphertext: heapless::Vec<u8, 4096>,
    pub kem_ciphertext: Option<Ciphertext>, // present for first message in session
}
```
Implement `postcard` serialization/deserialization with round-trip tests.

**5. `crates/verdant-qudag/src/anonymity.rs` — Onion Routing:**
```rust
pub struct OnionLayer {
    pub next_hop: NodeId,
    pub encrypted_inner: heapless::Vec<u8, 8192>,
}

pub fn wrap_onion_layer(inner: &[u8], next_hop_pubkey: &PublicKey, kem: &impl PostQuantumCrypto) -> Result<OnionLayer, CryptoError>;
pub fn unwrap_onion_layer(layer: &OnionLayer, our_secret_key: &SecretKey, kem: &impl PostQuantumCrypto) -> Result<(Vec<u8>, bool), CryptoError>;
// bool = is_final_destination
```
Minimum 3 layers for anonymity.

### London School TDD Tests

**dag.rs tests:**
- `validate_parents_accepts_known_parents` — insert genesis, then a child referencing it → Ok
- `validate_parents_rejects_unknown_parents` — reference a hash never inserted → Err(DagError::OrphanMessage)
- `insert_updates_tips` — after insert, new hash appears in tips, consumed parent removed from tips
- `is_seen_returns_true_after_insert` — insert a message, query its hash → true
- `tips_bounded_to_max` — insert MAX_DAG_TIPS+1 messages → tips.len() <= MAX_DAG_TIPS

**message.rs tests:**
- Round-trip: `QuDagMessage` → postcard bytes → `QuDagMessage` → assert equality

**anonymity.rs tests (London School — mock crypto):**
- `wrap_onion_layer_calls_kem_encapsulate` — mock `PostQuantumCrypto`, verify `encapsulate` called once per layer
- `unwrap_onion_layer_calls_kem_decapsulate` — mock `PostQuantumCrypto`, verify `decapsulate` called
- `three_layer_wrap_unwrap_roundtrip` — wrap 3 layers, unwrap 3 times, verify original payload recovered
- `unwrap_wrong_key_fails` — use different secret key → Err

**Property tests (proptest):**
- `dag_accepts_valid_chains` — generate chains of 2..50 messages, all accepted
- `dag_rejects_all_orphans` — random parent hashes never in DAG → all rejected
- `message_roundtrip_serialization` — arbitrary `QuDagMessage` survives serialize/deserialize

### Acceptance Criteria
- `cargo test -p verdant-qudag` passes
- DAG rejects orphaned messages 100% of the time
- Onion wrap/unwrap round-trips for 3-layer paths
- Message serialization round-trips under proptest
- All crypto operations go through the `PostQuantumCrypto` trait (mockable)
- Crate compiles under `no_std` (verify with `cargo build -p verdant-qudag --no-default-features`)

---

## Prompt 03: `verdant-vector` — Vector Graph Environmental Learning

### Prerequisites
Prompt 01 (`verdant-core`) must be complete.

### Goal
Implement the self-learning vector graph neural network that runs on every node. This is the core intelligence of Verdant — it encodes environmental state into embeddings, learns seasonal baselines, detects anomalies by cosine distance, and computes compressed deltas for cross-mesh pattern propagation. All operations must work within 200 KB of RAM and execute on an ESP32 without FPU (fixed-point math).

### What to Build

**1. `crates/verdant-vector/Cargo.toml`:**
- Depends on `verdant-core`
- Dependencies: `nalgebra` (no_std, no default features), `micromath` (fixed-point trig/sqrt), `heapless`, `postcard`, `serde`, `lz4_flex` (no_std)
- Dev dependencies: `mockall`, `proptest`

**2. `crates/verdant-vector/src/graph.rs` — VectorGraph Aggregate Root:**
```rust
pub struct VectorGraph {
    nodes: heapless::Vec<GraphNode, MAX_GRAPH_NODES>,
    edges: SparseEdgeMatrix,
    seasonal_centroids: SeasonalBaselines,
    version: Version,
    merge_threshold: f32,
}

pub struct GraphNode {
    pub id: u16,               // index-based identity
    pub embedding: Embedding,
    pub visit_count: u32,
    pub last_modified: Version,
}

impl VectorGraph {
    pub fn new(capacity: usize) -> Self;
    pub fn node_count(&self) -> usize;
    pub fn version(&self) -> Version;

    pub fn update(&mut self, embedding: Embedding, clock: &impl SeasonalClock);
    pub fn nearest_neighbor(&self, query: &Embedding) -> Option<(u16, f32)>;  // (index, distance)

    pub fn anomaly_score(&self, embedding: &Embedding, clock: &impl SeasonalClock) -> f32;
    pub fn seasonal_baseline(&self, slot: &SeasonSlot) -> Option<&Embedding>;

    pub fn encode_and_update(
        &mut self,
        raw: RawFeatures,
        projector: &impl EmbeddingProjector,
        clock: &impl SeasonalClock,
    ) -> Embedding;
}
```

Implement the update logic from SPARC pseudocode:
1. Find nearest existing node by cosine distance
2. If distance < merge_threshold → exponential moving average merge (0.95/0.05)
3. Else if capacity available → insert new node, add k=5 nearest edges
4. Else → evict least-visited node, replace with new embedding
5. Update seasonal centroid running mean for current slot

**3. `crates/verdant-vector/src/embedding.rs` — Fixed-Point Embeddings:**
```rust
/// EMBEDDING_DIM-dimensional vector using i16 fixed-point representation.
/// Quantized to [-32768, 32767] range. Cosine similarity computed with integer arithmetic.
#[derive(Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub data: [i16; EMBEDDING_DIM],
}

impl Embedding {
    pub fn cosine_distance(&self, other: &Embedding) -> f32;
    pub fn dot_product(&self, other: &Embedding) -> i64;
    pub fn magnitude_squared(&self) -> i64;
    pub fn lerp(&self, other: &Embedding, alpha: f32) -> Embedding; // for EMA merge
}
```
Use integer dot products and `micromath::F32Ext::sqrt()` for magnitude normalization.

**4. `crates/verdant-vector/src/seasonal.rs` — Seasonal Baselines:**
```rust
pub struct SeasonalBaselines {
    centroids: [Option<RunningMean>; 52],  // one per week of year
}

pub struct RunningMean {
    pub mean: Embedding,
    pub count: u32,
}

impl SeasonalBaselines {
    pub fn update(&mut self, slot: SeasonSlot, embedding: &Embedding);
    pub fn get(&self, slot: &SeasonSlot) -> Option<&Embedding>;
    pub fn coverage(&self) -> u8; // how many of 52 slots have data
}
```

**5. `crates/verdant-vector/src/anomaly.rs` — Anomaly Detection:**
```rust
pub fn score_anomaly(
    current: &Embedding,
    baseline: &Embedding,
) -> f32; // cosine distance, 0.0 = identical, 1.0 = orthogonal

pub fn classify_anomaly(
    embedding: &Embedding,
    score: f32,
    sensor_reading: &SensorReading,
) -> EventCategory;
```

Classification logic uses sensor context: high soil moisture + pressure drop → Flood, CSI amplitude variance spike + seasonal mismatch → Pest, etc.

**6. `crates/verdant-vector/src/delta.rs` — Compressed Graph Deltas:**
```rust
#[derive(Serialize, Deserialize)]
pub struct CompressedDelta {
    pub added: heapless::Vec<CompressedNode, 64>,
    pub removed: heapless::Vec<u16, 64>,  // node IDs
    pub base_version: Version,
    pub target_version: Version,
}

pub struct CompressedNode {
    pub embedding: Embedding,
    pub visit_count: u32,
}

impl VectorGraph {
    pub fn compute_delta(&self, since_version: Version) -> CompressedDelta;
    pub fn apply_delta(&mut self, delta: &CompressedDelta) -> Result<(), DeltaError>;
}
```
Use `lz4_flex` compression for wire format. `compute_delta` collects nodes modified after `since_version`.

### London School TDD Tests

**graph.rs (mock SeasonalClock and EmbeddingProjector):**
- `update_merges_into_nearest_when_below_threshold` — mock clock, insert similar embeddings → node_count stays 1, visit_count increments
- `update_inserts_new_node_when_above_threshold` — mock clock, insert orthogonal embeddings → node_count increases
- `update_evicts_least_visited_at_capacity` — fill graph to capacity, insert new → least visited replaced
- `encode_and_update_delegates_to_projector` — mock projector, verify `project()` called once
- `encode_and_update_passes_clock_to_seasonal` — mock clock, verify `current_slot()` called

**anomaly.rs:**
- `high_score_when_far_from_baseline` — orthogonal vectors → score > 0.85
- `low_score_when_near_baseline` — nearly identical vectors → score < 0.1
- `classify_returns_flood_when_moisture_spike` — high soil_moisture + pressure drop → EventCategory::Flood

**delta.rs (mock nothing — pure data):**
- `compute_delta_captures_new_nodes` — insert nodes, compute delta → added contains new nodes
- `apply_delta_adds_nodes` — apply a delta with added nodes → graph grows
- `delta_roundtrip_through_compression` — compute → serialize → compress → decompress → deserialize → apply

**Property tests (proptest):**
- `graph_never_exceeds_capacity` — random embeddings up to 5000, graph ≤ 2048
- `anomaly_score_bounded_0_to_1` — arbitrary pair → score in [0.0, 1.0]
- `identical_embeddings_merge` — same embedding N times → node_count == 1
- `version_monotonically_increases` — version after update ≥ version before
- `cosine_distance_symmetric` — d(a,b) == d(b,a)

### Acceptance Criteria
- `cargo test -p verdant-vector` passes
- Graph capacity invariant holds under proptest
- Anomaly scores correctly bounded
- Delta round-trip through lz4 compression works
- All operations use i16 fixed-point (no f64)
- `no_std` build succeeds

---

## Prompt 04: `verdant-mesh` — Mesh Networking Layer

### Prerequisites
Prompts 01 (`verdant-core`) and 02 (`verdant-qudag`) must be complete.

### Goal
Implement the mesh networking layer: BLE-based neighbor discovery, distance-vector routing with empirically-learned terrain-aware link costs, frame encoding/decoding over WiFi, partition detection with epidemic recovery, and bandwidth-adaptive compression. This crate provides `MeshTransport` — the transport layer all other crates use to communicate.

### What to Build

**1. `crates/verdant-mesh/Cargo.toml`:**
- Depends on `verdant-core`, `verdant-qudag`
- Dependencies: `heapless`, `postcard`, `serde`
- Dev dependencies: `mockall`, `proptest`

**2. `crates/verdant-mesh/src/discovery.rs` — Neighbor Discovery:**
```rust
pub struct DiscoveryService<R: RadioHardware> {
    radio: R,
    self_id: NodeId,
    beacon_interval_ms: u32,
}

pub struct Beacon {
    pub node_id: NodeId,
    pub zone_id: ZoneId,
    pub firmware_version: SemVer,
    pub uptime_secs: u32,
}

impl<R: RadioHardware> DiscoveryService<R> {
    pub fn broadcast_beacon(&mut self, beacon: &Beacon) -> Result<(), RadioError>;
    pub fn scan_neighbors(&mut self) -> Result<heapless::Vec<NeighborBeacon, 32>, RadioError>;
}
```

**3. `crates/verdant-mesh/src/routing.rs` — Terrain-Aware Distance-Vector Routing:**
```rust
pub struct RoutingTable {
    self_id: NodeId,
    routes: heapless::FnvIndexMap<NodeId, Route, MAX_NEIGHBORS>,
    neighbors: heapless::Vec<Neighbor, MAX_NEIGHBORS>,
}

pub struct Route {
    pub destination: NodeId,
    pub next_hop: NodeId,
    pub cost: f32,        // empirical: 1.0/PDR (higher = worse)
    pub hops: u8,
    pub last_updated: Timestamp,
}

pub struct Neighbor {
    pub id: NodeId,
    pub link_quality: LinkQuality,
    pub last_seen: Timestamp,
    pub status: NeighborStatus,
}

pub struct LinkQuality {
    pub pdr: f32,      // packet delivery ratio [0.0, 1.0]
    pub rtt_ms: u32,   // round trip time
    pub cost: f32,     // derived: 1.0 / pdr
}

pub enum NeighborStatus { Active, Suspect, Dead }

pub struct RoutingUpdate {
    pub from: NodeId,
    pub destination: NodeId,
    pub cost: f32,
    pub hops: u8,
}

impl RoutingTable {
    pub fn new(self_id: NodeId) -> Self;
    pub fn apply_update(&mut self, update: RoutingUpdate) -> Result<bool, RoutingError>; // bool = changed
    pub fn best_route(&self, destination: NodeId) -> Option<&Route>;
    pub fn update_link_quality(&mut self, neighbor: NodeId, pdr: f32, rtt_ms: u32);
    pub fn mark_suspect(&mut self, neighbor: NodeId);
    pub fn mark_dead(&mut self, neighbor: NodeId);
    pub fn active_neighbors(&self) -> impl Iterator<Item = &Neighbor>;
    pub fn should_rebalance(&self) -> bool;
}
```

Split-horizon: don't advertise a route back to the node it was learned from. Reject routes to self. Reject negative costs.

**4. `crates/verdant-mesh/src/transport.rs` — Frame Encoding and MeshTransport Implementation:**
```rust
pub struct MeshCommunicationLayer<R: RadioHardware, C: PostQuantumCrypto> {
    radio: R,
    crypto: C,
    routing_table: RoutingTable,
    dag_state: DagState,
    outgoing_queue: heapless::Deque<MeshFrame, 16>,
}

impl<R: RadioHardware, C: PostQuantumCrypto> MeshTransport for MeshCommunicationLayer<R, C> {
    fn send(&mut self, frame: &MeshFrame) -> Result<(), TransportError>;
    fn receive(&mut self) -> Result<Option<MeshFrame>, TransportError>;
    fn broadcast(&mut self, frame: &MeshFrame, ttl: u8) -> Result<(), TransportError>;
}
```

`send()` must: sign the frame with Dilithium, build DAG references, construct onion layers if destination specified, then transmit via radio.
`receive()` must: verify DAG parents, verify signature, unwrap onion layer if applicable, forward or deliver locally.

**5. `crates/verdant-mesh/src/partition.rs` — Partition Detection and Recovery:**
```rust
pub enum ConnectivityMode { Normal, Degraded, Partitioned, Epidemic }

pub struct PartitionDetector {
    last_gateway_contact: Option<Timestamp>,
    gateway_timeout_secs: u64,
}

impl PartitionDetector {
    pub fn check(&self, routing_table: &RoutingTable, now: Timestamp) -> ConnectivityMode;
}
```

When partitioned: switch to epidemic gossip mode (flood all messages instead of routing).

**6. `crates/verdant-mesh/src/compression.rs` — Bandwidth-Adaptive Compression:**
```rust
pub enum CompressionLevel { None, Light, Medium, Maximum }

pub fn select_compression(link: &LinkQuality) -> CompressionLevel;
pub fn compress(data: &[u8], level: CompressionLevel) -> Result<heapless::Vec<u8, 8192>, CompressionError>;
pub fn decompress(data: &[u8]) -> Result<heapless::Vec<u8, 8192>, CompressionError>;
```

### London School TDD Tests

**discovery.rs (mock RadioHardware):**
- `broadcast_beacon_calls_radio_transmit` — verify `transmit_frame()` called once with serialized beacon
- `scan_neighbors_calls_radio_scan` — mock `scan_ble_beacons()` returning 3 beacons → returns 3 `NeighborBeacon`

**routing.rs (pure domain — mostly state tests + property tests):**
- `apply_update_selects_lowest_cost_path` — two routes to same destination, different costs → best_route returns cheaper
- `apply_update_rejects_route_to_self` — destination == self_id → Err
- `mark_dead_removes_from_active_neighbors` — mark dead → not in `active_neighbors()`
- `split_horizon_prevents_readvertise` — route learned from node A → not advertised back to A
- Property: `best_route_always_lowest_cost` — random routes, best_route.cost == min(all costs)
- Property: `routing_table_never_contains_self_route`

**transport.rs (mock RadioHardware + PostQuantumCrypto):**
- `send_signs_message_then_transmits` — verify `crypto.sign()` called, then `radio.transmit_frame()` called
- `send_builds_dag_references` — verify DAG tips are included in outgoing message
- `receive_rejects_invalid_signature` — mock `crypto.verify()` → false → message dropped, not delivered
- `receive_rejects_orphan_dag_parents` — message with unknown parents → dropped
- `receive_forwards_relay_with_decremented_ttl` — receive a relay message with ttl=5 → forward with ttl=4
- `receive_delivers_locally_when_for_us` — message addressed to us → returned from `receive()`

**partition.rs:**
- `check_returns_partitioned_when_no_gateway_route` — routing table has no route to any gateway → Partitioned
- `check_returns_normal_when_gateway_reachable` — route to gateway exists, recent contact → Normal

### Acceptance Criteria
- `cargo test -p verdant-mesh` passes
- Transport signs and verifies every message (London School: mock crypto, verify interaction)
- Routing rejects self-routes and selects lowest cost (proptest)
- DAG orphan rejection works end-to-end through transport layer
- Partition detection triggers epidemic mode
- `no_std` build succeeds

---

## Prompt 05: `verdant-sense` — Sensor Abstraction Layer

### Prerequisites
Prompt 01 (`verdant-core`) must be complete.

### Goal
Implement the sensor abstraction layer: WiFi CSI feature extraction, environmental sensor drivers (temperature, humidity, soil moisture, barometric pressure, light), and multi-sensor fusion into the `RawFeatures` format consumed by `verdant-vector`. This crate provides concrete implementations of `CsiCapture` and `EnvironmentalSensor` for ESP32 hardware, behind trait boundaries that enable London School testing.

### What to Build

**1. `crates/verdant-sense/Cargo.toml`:**
- Depends on `verdant-core`
- Dependencies: `heapless`, `micromath`
- Feature `esp32`: depends on `esp-hal`, `esp-wifi` (hardware implementations)
- Feature `mock`: test doubles for CI (default for tests)
- Dev dependencies: `mockall`

**2. `crates/verdant-sense/src/hal.rs` — Hardware Abstraction Traits:**
```rust
pub trait CsiHardware {
    fn configure_csi(&mut self, config: &CsiConfig) -> Result<(), HalError>;
    fn capture_raw(&mut self, duration_ms: u32) -> Result<RawCsiBuffer, HalError>;
}

pub trait SensorHardware {
    fn read_temperature(&mut self) -> Result<i16, HalError>;   // celsius * 100
    fn read_humidity(&mut self) -> Result<u16, HalError>;      // RH% * 100
    fn read_soil_moisture(&mut self) -> Result<u16, HalError>; // 0-10000
    fn read_pressure(&mut self) -> Result<u32, HalError>;      // Pascals
    fn read_light(&mut self) -> Result<u16, HalError>;         // lux
}

pub struct CsiConfig {
    pub channel: u8,
    pub bandwidth: CsiBandwidth,
}

pub struct RawCsiBuffer {
    pub subcarriers: heapless::Vec<SubcarrierData, 64>,
    pub duration_ms: u32,
}

pub struct SubcarrierData {
    pub amplitude: i16,
    pub phase: i16,
}
```

**3. `crates/verdant-sense/src/csi.rs` — CSI Feature Extraction:**
```rust
pub struct CsiFeatureExtractor;

impl CsiFeatureExtractor {
    /// Extract features from raw CSI data:
    /// - amplitude variance across subcarriers (movement detection)
    /// - phase shift standard deviation (object size estimation)
    /// - temporal autocorrelation (periodic vs transient activity)
    pub fn extract(buffer: &RawCsiBuffer) -> CsiFeatures;
}

pub struct CsiFeatures {
    pub amplitude_variance: f32,
    pub phase_std_dev: f32,
    pub temporal_autocorrelation: f32,
    pub dominant_frequency: f32,
    pub subcarrier_count: u8,
}
```

**4. `crates/verdant-sense/src/environmental.rs` — Environmental Sensor Wrapper:**
```rust
pub struct EnvironmentalReader<S: SensorHardware> {
    hardware: S,
    pressure_history: heapless::HistoryBuffer<u32, 8>, // for delta calculation
}

impl<S: SensorHardware> EnvironmentalSensor for EnvironmentalReader<S> {
    fn read(&mut self) -> Result<SensorReading, SenseError>;
}
```

`SensorReading` includes `pressure_delta` (rate of change) computed from the history buffer.

**5. `crates/verdant-sense/src/fusion.rs` — Multi-Sensor Fusion:**
```rust
pub struct SensorFusion<C: CsiHardware, S: SensorHardware> {
    csi_hw: C,
    sensor_hw: S,
    csi_extractor: CsiFeatureExtractor,
}

impl<C: CsiHardware, S: SensorHardware> SensorFusion<C, S> {
    pub fn capture_and_fuse(&mut self, csi_duration_ms: u32) -> Result<RawFeatures, SenseError>;
}
```

`RawFeatures` concatenates CSI features + normalized sensor readings into the vector that `EmbeddingProjector` consumes.

### London School TDD Tests

**fusion.rs (mock CsiHardware + SensorHardware):**
- `capture_and_fuse_reads_all_sensors` — verify each sensor method called exactly once
- `capture_and_fuse_captures_csi_with_correct_duration` — verify `capture_raw(5000)` called
- `capture_and_fuse_returns_correct_dimension` — output has `FEATURE_DIM` elements
- `capture_and_fuse_propagates_sensor_error` — mock temperature returns Err → SenseError returned

**csi.rs (pure computation — state tests):**
- `extract_high_variance_for_movement` — inject subcarrier data with high amplitude variance → amplitude_variance > threshold
- `extract_low_variance_for_still` — inject uniform data → amplitude_variance near 0

**environmental.rs (mock SensorHardware):**
- `read_computes_pressure_delta` — two reads with different pressures → `pressure_delta` is nonzero
- `read_normalizes_to_expected_ranges` — mock returns raw values → SensorReading fields in [0.0, 1.0]

### Acceptance Criteria
- `cargo test -p verdant-sense` passes
- Fusion reads all sensor channels (London School verified)
- CSI feature extraction produces bounded outputs
- Pressure delta computation correct over history buffer
- `no_std` build succeeds (with `--no-default-features`)

---

## Prompt 06: `verdant-safla` — Self-Aware Feedback Loop

### Prerequisites
Prompts 01 (`verdant-core`), 03 (`verdant-vector`), and 04 (`verdant-mesh`) must be complete.

### Goal
Implement the SAFLA four-phase cycle: Health Assessment (detect dead neighbors, reroute), Anomaly Consensus (cross-node spatial-temporal quorum), Pattern Propagation (epidemic broadcast of vector graph deltas), and Topology Optimization (route rebalancing). This is the "autonomic nervous system" of the mesh.

### What to Build

**1. `crates/verdant-safla/Cargo.toml`:**
- Depends on `verdant-core`, `verdant-vector` (for `CompressedDelta`), `verdant-mesh` (for `RoutingTable` types, but communicates via traits, not direct dependency on mesh internals)
- Dev dependencies: `mockall`

**2. `crates/verdant-safla/src/health.rs` — Neighbor Health Monitoring:**
```rust
pub struct HealthAssessor {
    timeout_threshold_secs: u64,
    max_consecutive_misses: u8,
}

pub struct HealthReport {
    pub newly_suspect: heapless::Vec<NodeId, 16>,
    pub newly_dead: heapless::Vec<NodeId, 16>,
    pub reroute_results: heapless::Vec<(NodeId, bool), 16>, // (dead_node, reroute_success)
}

impl HealthAssessor {
    pub fn assess(
        &self,
        neighbors: &[Neighbor],
        now: Timestamp,
        healer: &mut impl NetworkHealer,
    ) -> HealthReport;
}
```

Logic: if `now - neighbor.last_seen > timeout_threshold` → mark suspect. If `consecutive_misses > max` → mark dead, call `healer.reroute_around()`.

**3. `crates/verdant-safla/src/consensus.rs` — Anomaly Consensus Engine:**
```rust
pub struct ConsensusEngine {
    quorum: u8,
    temporal_window_secs: u64,
    spatial_radius_m: f32,
}

impl ConsensusEngine {
    pub fn evaluate(
        &self,
        source: &impl AnomalySource,
        sink: &mut impl ConfirmedEventSink,
    ) -> Result<heapless::Vec<ConfirmedEvent, 8>, ConsensusError>;
}
```

Full consensus logic from SPARC pseudocode and testing-strategy.md: filter neighbor anomalies by spatial proximity AND temporal window AND category match. If corroborating count >= quorum → emit confirmed event.

**4. `crates/verdant-safla/src/propagation.rs` — Pattern Delta Propagation:**
```rust
pub struct PatternPropagator {
    last_propagated_version: Version,
}

impl PatternPropagator {
    pub fn check_and_propagate(
        &mut self,
        graph: &VectorGraph,
        transport: &mut impl MeshTransport,
    ) -> Result<bool, PropagationError>; // bool = did propagate
}
```

Compute delta from `last_propagated_version`, if non-empty broadcast with `TTL = PATTERN_PROPAGATION_TTL`.

**5. `crates/verdant-safla/src/events.rs` — Event Classification and Flood Propagation:**
```rust
pub trait WatershedGraph {
    fn downstream_neighbors(&self, zone: ZoneId) -> heapless::Vec<(ZoneId, u32), 8>; // (zone, flow_time_secs)
    fn current_soil_saturation(&self, zone: ZoneId) -> f32;
}

pub trait AlertEmitter {
    fn emit_preemptive_alert(&mut self, alert: &FloodPreemptiveAlert) -> Result<(), EmitError>;
}

pub struct FloodPropagationHandler;

impl FloodPropagationHandler {
    pub fn propagate(
        event: &ConfirmedEvent,
        watershed: &impl WatershedGraph,
        emitter: &mut impl AlertEmitter,
    ) -> Result<u32, PropagationError>; // count of zones alerted
}
```

Implement Dijkstra-based flood propagation from SPARC pseudocode: priority queue by arrival time, adjust for soil saturation.

**6. `crates/verdant-safla/src/topology.rs` — Topology Optimization:**
```rust
pub struct TopologyOptimizer {
    rebalance_interval_secs: u64,
    last_rebalance: Timestamp,
}

impl TopologyOptimizer {
    pub fn should_rebalance(&self, now: Timestamp) -> bool;
    pub fn propose_changes(
        &self,
        routing_table: &RoutingTable,
        healer: &mut impl NetworkHealer,
    ) -> Result<u8, HealError>; // count of proposals
}
```

### London School TDD Tests

**health.rs (mock NetworkHealer):**
- `marks_dead_and_reroutes_after_threshold` — neighbor with 4 misses → `healer.mark_dead()` + `healer.reroute_around()` called
- `marks_suspect_but_not_dead_at_threshold` — neighbor with 2 misses → suspect only, `reroute_around()` NOT called
- `ignores_healthy_neighbors` — recent `last_seen` → no calls to healer

**consensus.rs (mock AnomalySource + ConfirmedEventSink):**
- `confirms_when_quorum_met` — 3 corroborating neighbors → `sink.emit_confirmed()` called once
- `does_not_confirm_below_quorum` — 1 corroboration → `emit_confirmed()` never called
- `filters_temporally_distant` — 3 neighbors but 2 outside window → not confirmed
- `filters_spatially_distant` — 3 neighbors but 2 in different zone → not confirmed
- `filters_different_category` — 3 neighbors but different categories → not confirmed

**propagation.rs (mock MeshTransport):**
- `propagates_when_new_patterns_exist` — graph version > last_propagated → `transport.broadcast()` called with TTL=16
- `does_not_propagate_when_no_changes` — graph version == last_propagated → `broadcast()` not called

**events.rs (mock WatershedGraph + AlertEmitter):**
- `flood_alerts_all_downstream_zones` — 2 downstream neighbors with further downstream → 3 alerts total
- `ignores_non_flood_events` — pest event → 0 alerts, `emit_preemptive_alert()` never called
- `adjusts_arrival_time_for_soil_saturation` — high saturation → shorter arrival time in alert

### Acceptance Criteria
- `cargo test -p verdant-safla` passes
- Consensus correctly applies spatial + temporal + category filters (London School verified)
- Flood propagation implements Dijkstra with saturation adjustment
- Health assessment reroutes around dead nodes
- Pattern propagation only fires when graph version advances

---

## Prompt 07: `verdant-sim` — Network Simulator

### Prerequisites
Prompts 01-06 must be complete (all library crates).

### Goal
Build a discrete-event network simulator that instantiates hundreds of virtual nodes using the real `verdant-vector`, `verdant-mesh`, `verdant-qudag`, and `verdant-safla` crate code — with mock radio/sensor hardware. This is the integration test harness and the acceptance test environment for outside-in TDD.

### What to Build

**1. `crates/verdant-sim/Cargo.toml`:**
- Depends on all `verdant-*` crates (with `std` feature)
- Dependencies: `bevy_ecs`, `rand`, `serde`, `serde_json`
- This crate is `std` only (never targets ESP32)

**2. `crates/verdant-sim/src/main.rs` — Simulation Harness:**
```rust
pub struct Simulation {
    world: bevy_ecs::World,
    schedule: bevy_ecs::Schedule,
    config: SimConfig,
    clock: SimClock,
}

pub struct SimConfig {
    pub node_count: usize,
    pub zone_count: usize,
    pub zone_layout: ZoneLayout,
    pub terrain: Option<TerrainModel>,
    pub rf_propagation: RfModel,
}

pub enum ZoneLayout {
    Grid { rows: usize, cols: usize },
    Watershed,
    Random { seed: u64 },
}

impl Simulation {
    pub fn new(config: SimConfig) -> Self;
    pub fn deploy_nodes(&mut self);
    pub fn run_for(&mut self, duration: Duration);
    pub fn run_until_mesh_converges(&mut self, timeout: Duration);
    pub fn fast_forward_training(&mut self, duration: Duration);

    // Event injection
    pub fn inject_event(&mut self, event: impl SimEvent);
    pub fn kill_node(&mut self, node: NodeId);
    pub fn partition_zones(&mut self, zones: Vec<ZoneId>, duration: Duration);

    // Queries
    pub fn can_reach_gateway(&self, node: NodeId) -> bool;
    pub fn alerts_received_by(&self, zone: ZoneId) -> Vec<Alert>;
    pub fn has_confirmed_event(&self, zone: ZoneId, category: EventCategory) -> bool;
    pub fn proposal_status(&self, id: ProposalHash) -> ProposalStatus;
}
```

Each simulated node is a bevy ECS entity with components: `VectorGraph`, `RoutingTable`, `DagState`, `SimPosition`, `SimRadio` (mock that delivers to neighbors within range).

**3. `crates/verdant-sim/src/terrain.rs` — Vermont Terrain Model:**
RF propagation model: given two positions and terrain profile, compute expected PDR. Simplified: use distance + elevation difference + forest density factor.

**4. `crates/verdant-sim/src/weather.rs` — Weather Replay:**
Load historical Vermont weather data (temperature, precipitation, soil moisture) and replay it as sensor readings for simulated nodes.

**5. `crates/verdant-sim/src/scenario.rs` — Pre-Built Scenarios:**
- `FloodScenario` — inject rising water upstream, verify downstream alerts
- `PestSpreadScenario` — inject pest signature at boundary, verify cross-zone detection
- `PartitionScenario` — partition zones, verify mesh self-heals
- `GovernanceScenario` — submit proposal, cast votes across zones, verify tally

### Integration Tests (Acceptance Tests)

These are the tests from `testing-strategy.md`:
- `flood_detected_and_downstream_alerted_within_60_seconds`
- `mesh_self_heals_after_node_failure_within_30_seconds`
- `governance_vote_completes_across_partitioned_mesh`
- `pest_detection_across_zone_boundary`

### Acceptance Criteria
- `cargo test -p verdant-sim` passes all 4 integration tests
- Simulation runs 50+ nodes without panic
- Mesh converges (all nodes reachable) within 120 simulated seconds
- Anomaly consensus works end-to-end with real `verdant-safla` code

---

## Prompt 08: `verdant-firmware` — ESP32 Node Firmware

### Prerequisites
Prompts 01-06 must be complete.

### Goal
Build the ESP32-S3 firmware binary that wires together all library crates into the main sense-learn-detect-communicate-heal-sleep loop. This is the `NodeFirmware` orchestrating aggregate from the domain model.

### What to Build

**1. `crates/verdant-firmware/Cargo.toml`:**
- Depends on `verdant-core`, `verdant-vector`, `verdant-qudag`, `verdant-mesh`, `verdant-sense`, `verdant-safla`
- Dependencies: `esp-hal`, `esp-wifi`, `embassy-executor`, `embassy-time`, `defmt`, `defmt-rtt`
- Target: `xtensa-esp32s3-none-elf`

**2. `crates/verdant-firmware/src/main.rs` — Entry Point:**
```rust
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialize hardware
    // Load vector graph from flash
    // Load routing table from flash
    // Enter main loop
    loop {
        node.run_cycle().await;
        embassy_time::Timer::after(Duration::from_millis(sleep_ms)).await;
    }
}
```

Wire concrete ESP32 implementations of all traits:
- `Esp32CsiCapture` implements `CsiCapture`
- `Esp32Sensors` (BME280 over I2C, soil moisture ADC, BH1750) implements `EnvironmentalSensor`
- `Esp32Radio` (esp-wifi) implements `RadioHardware`
- `Esp32Flash` implements `FlashStorage`
- `Esp32Power` (ADC on battery + solar pins) implements `PowerMonitor`
- `Esp32Crypto` (wraps pqcrypto) implements `PostQuantumCrypto`

**3. `crates/verdant-firmware/src/power.rs` — Power Management:**
- Read battery voltage via ADC
- Read solar panel output via ADC
- Adaptive duty cycling: low battery → longer sleep, no solar → emergency mode
- Deep sleep entry via `esp-hal` RTC

**4. `crates/verdant-firmware/src/storage.rs` — Flash Persistence:**
- Wear-leveled write to vector graph partition
- Delta-write strategy: only write changed sectors
- Checkpoint every `CHECKPOINT_INTERVAL_SECS` (1 hour)
- Partition map: Firmware A (1.5MB), Firmware B (1.5MB), VectorGraph (768KB), Config (64KB), EventLog (128KB)

**5. `crates/verdant-firmware/src/ota.rs` — Over-The-Air Updates:**
- Receive firmware image chunks via mesh messages
- Write to inactive partition (B if running A)
- Verify hash against `FirmwareHash` from governance vote
- Swap active partition on next boot

### London School TDD Tests

The firmware crate's tests run on **x86** (not ESP32) by mocking all hardware traits:
- `full_cycle_sense_learn_detect_communicate_heal` — all mocks, verify orchestration order
- `cycle_broadcasts_anomaly_when_detected` — mock CSI returns anomalous data → `transport.send()` called
- `cycle_does_not_broadcast_on_normal_reading` — mock CSI returns normal → `transport.send()` NOT called
- `cycle_enters_deep_sleep_on_low_battery` — mock power returns 0.05 → extended sleep duration
- `checkpoint_fires_after_interval` — mock clock advances 1 hour → `storage.write_block()` called
- `checkpoint_does_not_fire_before_interval` — mock clock advances 30 min → `storage.write_block()` NOT called

### Acceptance Criteria
- `cargo test -p verdant-firmware` passes on x86 (all mocked)
- `cargo build -p verdant-firmware --target xtensa-esp32s3-none-elf` succeeds (cross-compilation)
- Binary size < 1.5 MB (fits in one flash partition)
- All hardware access goes through mockable traits
- Main loop implements all 6 phases in order

---

## Prompt 09: `verdant-robotics` — Agentic Robotics Coordination

### Prerequisites
Prompts 01 (`verdant-core`) and 04 (`verdant-mesh`) must be complete.

### Goal
Implement the agentic robotics layer: mission state machine, quantum magnetometer navigation interface, swarm coordination protocol, and hard safety constraints. Robots are mobile mesh relay nodes that can physically respond to confirmed events.

### What to Build

**1. `crates/verdant-robotics/Cargo.toml`:**
- Depends on `verdant-core`, `verdant-mesh` (for mesh relay behavior)
- Dev dependencies: `mockall`

**2. `crates/verdant-robotics/src/mission.rs` — Mission State Machine:**
```rust
pub struct MissionStateMachine {
    state: RobotState,
    active_mission: Option<Mission>,
    battery: BatteryLevel,
    min_battery_for_mission: f32,
}

pub enum RobotState { Idle, Tasked, Navigating, Executing, Returning, SafetyAbort }

impl MissionStateMachine {
    pub fn assign(&mut self, mission: Mission) -> Result<(), MissionError>;
    pub fn begin_navigation(&mut self) -> Result<(), MissionError>;
    pub fn arrive_at_target(&mut self) -> Result<(), MissionError>;
    pub fn complete_execution(&mut self, result: MissionResult) -> Result<(), MissionError>;
    pub fn begin_return(&mut self) -> Result<(), MissionError>;
    pub fn arrive_at_base(&mut self) -> Result<(), MissionError>;
    pub fn safety_abort(&mut self, reason: AbortReason) -> Result<(), MissionError>;
    pub fn state(&self) -> RobotState;
}
```

Invariants: only one active mission; battery check before accepting; SafetyAbort overrides any state.

**3. `crates/verdant-robotics/src/navigation.rs` — Magnetometer Navigation Interface:**
```rust
pub trait MagneticNavigator {
    fn current_position(&self) -> Result<Position, NavError>;
    fn navigate_to(&mut self, target: Position) -> Result<NavCommand, NavError>;
    fn magnetic_fix_quality(&self) -> FixQuality;
}

pub enum NavCommand { Heading(f32), Hover, Land }
pub enum FixQuality { Excellent, Good, Degraded, Lost }
```

**4. `crates/verdant-robotics/src/safety.rs` — Safety Constraints:**
```rust
pub trait SafetyConstraintChecker {
    fn check_no_fly_zone(&self, position: &Position) -> bool;  // true = in no-fly zone
    fn check_weather(&self) -> WeatherStatus;
    fn check_battery(&self) -> BatteryStatus;
    fn check_wildlife_corridor(&self, position: &Position, time: Timestamp) -> bool;
}

pub enum WeatherStatus { Safe, Marginal, Unsafe }
pub enum BatteryStatus { Sufficient, Low, Critical }
```

Safety check runs every cycle. Any constraint violation → immediate `safety_abort()`.

**5. `crates/verdant-robotics/src/swarm.rs` — Swarm Coordination:**
```rust
pub trait SwarmCoordinator {
    fn request_task(&mut self, event: &ConfirmedEvent) -> Result<TaskAssignment, SwarmError>;
    fn resolve_conflict(&mut self, task_a: TaskId, task_b: TaskId) -> Resolution;
    fn report_completion(&mut self, task_id: TaskId, result: TaskResult) -> Result<(), SwarmError>;
}
```

**6. `crates/verdant-robotics/src/relay.rs` — Mobile Relay Node:**
Robot acts as a mesh relay while in transit: receives mesh messages, forwards them.

### London School TDD Tests

**mission.rs (state machine — state tests, no mocks needed):**
- `state_machine_idle_to_tasked_to_navigating` — assign → Tasked, begin_navigation → Navigating
- `assign_rejects_when_already_tasked` — assign twice → Err(MissionError::AlreadyBusy)
- `assign_rejects_low_battery` — battery 0.05 → Err(MissionError::InsufficientBattery)
- `safety_abort_overrides_any_state` — from Navigating → SafetyAbort
- `complete_returns_to_idle` — full cycle → back to Idle
- Property test: `no_invalid_state_transitions` — random sequence of valid transitions never panics

**safety.rs (mock SafetyConstraintChecker):**
- `abort_on_no_fly_zone` — `check_no_fly_zone()` returns true → mission aborted
- `abort_on_unsafe_weather` — `check_weather()` returns Unsafe → aborted
- `abort_on_critical_battery` — `check_battery()` returns Critical → aborted
- `no_abort_when_all_safe` — all checks pass → mission continues

**relay.rs (mock MeshTransport):**
- `forwards_mesh_messages_while_navigating` — receive relay message → `transport.send()` called

### Acceptance Criteria
- `cargo test -p verdant-robotics` passes
- State machine enforces all invariants (single mission, battery check, safety override)
- Safety abort reachable from every state
- Robot relays mesh messages during transit

---

## Prompt 10: `verdant-gateway` — Raspberry Pi Gateway Daemon

### Prerequisites
Prompts 01, 04, and 06 must be complete.

### Goal
Build the gateway daemon that bridges the mesh to a local network: mesh radio bridge, redb persistence, axum REST/WebSocket API, gateway-to-gateway CRDT sync, and governance state machine. This is the `std` Rust service running on Raspberry Pi.

### What to Build

**1. `crates/verdant-gateway/Cargo.toml`:**
- Depends on `verdant-core` (with `std` feature), `verdant-mesh`, `verdant-safla`
- Dependencies: `tokio`, `axum`, `redb`, `serde`, `serde_json`, `tracing`, `tower-http`

**2. `crates/verdant-gateway/src/db.rs` — redb Schema:**
```rust
pub trait EventStore: Send + Sync {
    fn store_event(&self, event: &ConfirmedEvent) -> Result<EventId, DbError>;
    fn events_since(&self, since: Timestamp) -> Result<Vec<ConfirmedEvent>, DbError>;
    fn events_in_zone(&self, zone: ZoneId, limit: usize) -> Result<Vec<ConfirmedEvent>, DbError>;
}

pub trait NodeStatusStore: Send + Sync {
    fn update_status(&self, node_id: NodeId, status: &NodeStatus) -> Result<(), DbError>;
    fn all_statuses(&self) -> Result<Vec<(NodeId, NodeStatus)>, DbError>;
}

pub trait GovernanceStore: Send + Sync {
    fn store_proposal(&self, proposal: &Proposal) -> Result<(), DbError>;
    fn active_proposals(&self) -> Result<Vec<Proposal>, DbError>;
    fn record_vote(&self, proposal_id: &ProposalHash, vote: &SignedVote) -> Result<(), DbError>;
}
```

Implement these traits with redb. Tables: `events`, `node_statuses`, `proposals`, `votes`, `credits`.

**3. `crates/verdant-gateway/src/api.rs` — axum REST + WebSocket API:**
Endpoints:
- `GET /api/events?since={timestamp}` — recent events
- `GET /api/events/zone/{zone_id}` — events for a zone
- `GET /api/nodes` — all node statuses
- `GET /api/nodes/{node_id}` — single node status
- `POST /api/governance/proposals` — submit proposal
- `POST /api/governance/proposals/{id}/vote` — cast vote
- `GET /api/governance/proposals` — list active proposals
- `GET /ws` — WebSocket for real-time event stream

All handlers take store traits as injected dependencies (axum state).

**4. `crates/verdant-gateway/src/bridge.rs` — Mesh-to-Network Bridge:**
```rust
pub trait MeshBridge: Send {
    fn poll_mesh_events(&mut self) -> Result<Vec<MeshEvent>, BridgeError>;
    fn send_to_mesh(&mut self, msg: MeshFrame) -> Result<(), BridgeError>;
}
```

Background task: poll mesh → deserialize events → store in redb → push to WebSocket subscribers.

**5. `crates/verdant-gateway/src/governance.rs` — Governance State Machine:**
Implement `GovernanceEngine` from ADR-004. Receives proposals and votes from mesh, tallies when deadline passes, executes passed actions.

**6. `crates/verdant-gateway/src/sync.rs` — Gateway-to-Gateway CRDT Sync:**
When two gateways can communicate (LAN or occasional internet), merge state using CRDTs: events (grow-only set), node statuses (last-writer-wins register), proposals (grow-only set + vote merge).

### London School TDD Tests

**api.rs (mock all store traits):**
- `get_events_queries_store_with_since_param` — mock `EventStore`, verify `events_since()` called with correct timestamp
- `post_proposal_stores_and_returns_201` — mock `GovernanceStore`, verify `store_proposal()` called
- `get_nodes_returns_all_statuses` — mock `NodeStatusStore`, verify response JSON
- `websocket_pushes_new_events` — mock `MeshBridge`, inject event → WebSocket client receives it

**governance.rs (mock GovernanceBroadcaster + GovernancePersistence):**
- `tally_passes_with_majority` — 6/10 yes → action executes
- `tally_fails_quorum_with_too_few_votes` — 4/10 voted → FailedQuorum, action NOT executed
- `tally_pending_before_deadline` — deadline in future → Pending
- `rejects_vote_from_unknown_zone` — vote from unregistered zone → error

**bridge.rs (mock MeshBridge + EventStore):**
- `bridge_stores_incoming_events` — mesh delivers event → `event_store.store_event()` called

### Acceptance Criteria
- `cargo test -p verdant-gateway` passes
- API handlers tested with mocked stores (no real redb in unit tests)
- Governance tally logic matches ADR-004 specification
- WebSocket push verified via London School mocks
- `cargo build -p verdant-gateway --target aarch64-unknown-linux-gnu` cross-compiles for Raspberry Pi

---

## Prompt 11: Dashboard UI — Foundation (TypeScript)

### Prerequisites
Prompt 10 (`verdant-gateway`) must be complete (API contract defined).

### Goal
Scaffold the TypeScript PWA: Vite + React project, shared types mirroring the gateway API, port interfaces for London School testing, offline-first infrastructure with IndexedDB and Service Worker, and the WebSocket connection hook.

### What to Build

**1. `ui/` project scaffold:**
- `package.json` with React 18, TypeScript 5, Vite, Vitest, Zustand, ts-mockito
- `tsconfig.json` with strict mode
- `vite.config.ts` with PWA plugin (`vite-plugin-pwa`)
- `public/manifest.json` — PWA manifest

**2. `ui/src/lib/types.ts` — Shared Types (mirror gateway API):**
All domain types as TypeScript interfaces: `NodeId`, `ZoneId`, `ConfirmedEvent`, `EventCategory`, `NodeStatus`, `Proposal`, `Vote`, `FloodPreemptiveAlert`, etc.

**3. `ui/src/lib/ports.ts` — Port Interfaces for London School Testing:**
```typescript
export interface MeshDataPort {
    fetchNodeStatuses(): Promise<NodeStatus[]>;
    fetchEvents(since: number): Promise<ConfirmedEvent[]>;
    subscribeEvents(callback: (event: ConfirmedEvent) => void): Unsubscribe;
}

export interface OfflineSyncPort {
    getLocalState<T>(key: string): Promise<T | null>;
    setLocalState<T>(key: string, value: T): Promise<void>;
    queueForSync(operation: SyncOperation): Promise<void>;
    processSyncQueue(): Promise<SyncResult>;
}

export interface GovernancePort {
    fetchProposals(): Promise<Proposal[]>;
    submitProposal(draft: ProposalDraft): Promise<string>;
    castVote(proposalId: string, vote: Vote): Promise<void>;
    verifyVoteIntegrity(proposalId: string): Promise<VerificationResult>;
}

export interface AlertPort {
    fetchAlerts(): Promise<Alert[]>;
    configureRule(rule: AlertRule): Promise<void>;
    dismissAlert(alertId: string): Promise<void>;
}
```

**4. `ui/src/lib/adapters/` — Concrete Implementations:**
- `gateway-adapter.ts` — implements ports using `fetch()` + WebSocket to gateway API
- `indexeddb-adapter.ts` — implements `OfflineSyncPort` using IndexedDB

**5. `ui/src/hooks/useMeshData.ts` — Core Data Hook:**
Fetches from gateway, caches to IndexedDB, falls back to cache when offline.

**6. `ui/src/hooks/useOfflineSync.ts` — Offline Sync Hook:**
Background sync when gateway becomes reachable.

**7. `ui/src/hooks/useGovernance.ts` — Governance State Hook:**

**8. `ui/src/workers/sync-worker.ts` — Service Worker:**
Cache-first for static assets, network-first for API, background sync queue.

**9. `ui/src/App.tsx` — Root Component with Router:**
Routes: `/` (map), `/events`, `/alerts`, `/governance`, `/farm`, `/robots`

### London School TDD Tests

**useMeshData.test.ts:**
- `fetches_from_gateway_and_caches` — mock `MeshDataPort`, verify `setLocalState` called after fetch
- `falls_back_to_cache_when_offline` — mock fetch rejects, mock cache returns data → `isOffline = true`
- `subscribes_on_mount_unsubscribes_on_unmount` — verify `subscribeEvents` called, unsubscribe called on unmount

**useGovernance.test.ts:**
- `submit_proposal_calls_port` — mock `GovernancePort`, verify `submitProposal()` called
- `cast_vote_calls_port` — verify `castVote()` called with correct args

### Acceptance Criteria
- `npm test` passes all hook tests
- `npm run build` produces a working PWA build
- Types match gateway API contract
- All data access goes through port interfaces (no direct fetch in components)
- Service Worker registers and caches static assets

---

## Prompt 12: Dashboard UI — Feature Components (TypeScript)

### Prerequisites
Prompt 11 (UI foundation) must be complete.

### Goal
Build all dashboard view components: topographic mesh map, event timeline, alert manager, governance portal, farm dashboard, flood tracker, and robot overview. Each component consumes data from hooks (which use port interfaces) — enabling London School testing with mocked ports.

### What to Build

**1. `ui/src/components/MeshMap.tsx`:**
Leaflet map with OpenTopoMap tiles. Node overlay: green=active, yellow=suspect, red=dead, blue=robot. Click node → detail panel. Event heatmap layer. Offline tile caching.

**2. `ui/src/components/EventTimeline.tsx`:**
Chronological event stream. Filter by category (pest/flood/fire/etc), zone, severity. Real-time updates via WebSocket. Timestamp formatting.

**3. `ui/src/components/AlertManager.tsx`:**
Configure alert rules: "notify me when [category] in [zone] above [severity]". Show active alerts with dismiss. Notification permission request.

**4. `ui/src/components/GovernancePortal.tsx`:**
List active proposals with vote counts. Create new proposal form. Cast vote. Show results for completed proposals. Verify button → calls WASM QuDAG verifier (Prompt 13).

**5. `ui/src/components/FarmDashboard.tsx`:**
Per-sugarbush view: temperature, humidity, soil moisture, pressure over time (line charts). Sap flow prediction indicator. Tap timing recommendation. Data from zone-level aggregated readings.

**6. `ui/src/components/FloodTracker.tsx`:**
Animated flood propagation overlay on map. Shows estimated arrival time contours. Color-coded severity. Evacuation action recommendations.

**7. `ui/src/components/RobotOverview.tsx`:**
List of robots with status (Idle/Navigating/Executing). Current position on map. Active mission details. Battery level. Safety status.

**8. `ui/src/components/NodeStatus.tsx`:**
Detail view for a single node: sensor readings, vector graph health (node count, version), neighbor list, link qualities, uptime.

### London School TDD Tests (React Testing Library + ts-mockito)

For each component, mock the relevant port and verify rendering:
- `MeshMap renders nodes from useMeshData` — mock returns 3 nodes → 3 markers rendered
- `EventTimeline filters by category` — mock returns mixed events → click "Flood" → only flood events shown
- `GovernancePortal shows vote counts` — mock returns proposal with 6 yes / 2 no → "6 / 2" displayed
- `FarmDashboard shows temperature chart` — mock returns sensor data → chart component rendered
- `FloodTracker shows severity colors` — mock returns Emergency alert → red zone displayed
- `AlertManager creates rule` — fill form, submit → `alertPort.configureRule()` called with correct args
- `RobotOverview shows mission status` — mock returns 2 robots → both displayed with correct state

### Acceptance Criteria
- `npm test` passes all component tests
- All components render with mocked data (no gateway required)
- Map works with offline-cached tiles
- Governance portal has verify button (wired in Prompt 13)
- Responsive design: works on phone, tablet, laptop

---

## Prompt 13: WASM Bridge — QuDAG Verification in Browser

### Prerequisites
Prompts 02 (`verdant-qudag`) and 11 (UI foundation) must be complete.

### Goal
Compile `verdant-qudag`'s signature verification and DAG validation to WebAssembly so the dashboard can verify governance vote integrity without trusting the gateway. This is a trust boundary: users can independently verify that votes are cryptographically valid.

### What to Build

**1. `crates/verdant-qudag-wasm/Cargo.toml`:**
- New crate, depends on `verdant-qudag` (with `std` feature)
- Dependencies: `wasm-bindgen`, `serde-wasm-bindgen`
- Crate type: `cdylib`

**2. `crates/verdant-qudag-wasm/src/lib.rs`:**
```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn verify_dilithium_signature(data: &[u8], signature: &[u8], public_key: &[u8]) -> bool;

#[wasm_bindgen]
pub fn verify_dag_parents(message_json: &str, known_tips_json: &str) -> bool;

#[wasm_bindgen]
pub fn verify_proposal_votes(proposal_json: &str) -> JsValue; // returns VerificationResult
```

**3. Build integration:**
- `wasm-pack build --target web` produces `pkg/` with `.wasm` + JS bindings
- Vite config imports WASM module
- `ui/src/lib/qudag-verify.ts` wraps WASM functions with TypeScript types

**4. Wire into GovernancePortal:**
"Verify Votes" button calls `verifyProposalVotes()` via WASM → displays verification result.

### Tests
- Rust unit tests for WASM functions (standard `#[test]`, not in-browser)
- TypeScript test: mock WASM module, verify GovernancePortal calls `verifyVoteIntegrity()` when button clicked

### Acceptance Criteria
- `wasm-pack build` succeeds, produces < 500 KB WASM
- Signature verification works in browser
- GovernancePortal shows verification result after button click

---

## Prompt 14: End-to-End Integration Testing

### Prerequisites
All prior prompts (01-13) must be complete.

### Goal
Wire up the full pipeline — firmware → mesh → gateway → dashboard — in a comprehensive integration test suite. Run the simulator with a gateway and verify events flow through to the API. Run Playwright E2E tests against the dashboard connected to a test gateway.

### What to Build

**1. Integration test: `crates/verdant-sim/tests/full_pipeline.rs`:**
- Simulate 50 nodes
- Start a real `verdant-gateway` instance (in-process, with redb on tmpdir)
- Inject flood event → verify event appears in `GET /api/events`
- Inject pest spread → verify confirmed event in dashboard API
- Submit governance proposal → cast votes → verify tally via API

**2. E2E tests: `ui/e2e/` with Playwright:**
- Start gateway with test data
- Open dashboard in browser
- Verify map shows nodes
- Verify event timeline updates
- Verify governance proposal creation and voting flow
- Verify offline mode: disconnect gateway, verify cached data displayed

**3. Performance benchmarks: `crates/verdant-sim/benches/`:**
- 1000-node mesh convergence time
- Message throughput under load
- Vector graph update rate
- Crypto operation benchmarks (Dilithium sign/verify on host)

**4. Fuzz tests: `crates/verdant-qudag/fuzz/`:**
- Fuzz `QuDagMessage` deserialization
- Fuzz `DagState::validate_and_insert` with random inputs

### Acceptance Criteria
- Full pipeline integration test passes
- Playwright E2E tests pass
- 1000-node sim completes in < 5 minutes
- Fuzz tests run for 5 minutes without crash
- All verification criteria from SPARC spec confirmed

---

## Prompt 15: Deployment Tooling and Documentation

### Prerequisites
All prior prompts must be complete.

### Goal
Build the tooling needed to flash firmware to ESP32 nodes, deploy the gateway, and package the dashboard. Create deployment scripts, not documentation. This is the bridge from "code that compiles" to "system that runs in a field."

### What to Build

**1. `tools/flash.sh` — ESP32 Flashing Script:**
```bash
# Usage: ./tools/flash.sh /dev/ttyUSB0 [--zone ZONE_ID] [--node-index N]
# Builds firmware, flashes to ESP32-S3, provisions node identity
```
- Build with `cargo build --release -p verdant-firmware --target xtensa-esp32s3-none-elf`
- Flash with `espflash`
- Write zone ID and node index to config partition
- Generate and store Dilithium keypair in eFuse-protected flash

**2. `tools/deploy-gateway.sh` — Gateway Deployment:**
```bash
# Usage: ./tools/deploy-gateway.sh [--host raspberrypi.local]
# Cross-compiles gateway, copies to Pi, sets up systemd service
```
- Cross-compile with `cross build --release -p verdant-gateway --target aarch64-unknown-linux-gnu`
- SCP binary to Pi
- Install systemd unit file
- Initialize redb database

**3. `tools/build-dashboard.sh` — Dashboard Build and Deploy:**
```bash
# Usage: ./tools/build-dashboard.sh [--gateway-url http://192.168.1.100:8080]
# Builds PWA, copies to gateway's static file directory
```
- `npm run build` in `ui/`
- Copy `dist/` to gateway's static serving directory

**4. `tools/sim-scenario.sh` — Run Simulation Scenarios:**
```bash
# Usage: ./tools/sim-scenario.sh flood|pest|partition|governance [--nodes 100]
```
Wraps `cargo run -p verdant-sim` with pre-configured scenarios.

**5. `tools/generate-terrain.py` — Vermont DEM Processing:**
Download USGS DEM data for Vermont, process into the format `verdant-sim` expects.

**6. `.github/workflows/ci.yml` — CI Pipeline:**
Implement the PR pipeline from testing-strategy.md:
1. `cargo fmt --check`
2. `cargo clippy -- -D warnings`
3. `cargo test --workspace` (London School + property tests)
4. `cargo test -p verdant-sim` (integration)
5. `npm test` (UI)
6. Fuzz budget (5 min)

### Acceptance Criteria
- `flash.sh` provisions a real ESP32-S3 (if hardware available)
- `deploy-gateway.sh` produces a running service on Raspberry Pi
- `build-dashboard.sh` produces deployable PWA
- CI pipeline runs in < 7 minutes
- All scripts are executable and self-documenting (`--help`)
