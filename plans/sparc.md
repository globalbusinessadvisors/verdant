# Verdant: The Vermont Living Forest Mesh

## SPARC Specification

> A self-healing, peer-to-peer neural fabric for Vermont's forests, farms, watersheds, and rural communities — ambient intelligence that lives in the land itself.

**Primary Language:** Rust (embedded, mesh, core services)
**UI Language:** TypeScript (dashboard, governance, visualization)

---

## S — Specification

### 1. Problem Statement

Vermont faces three existential challenges: climate stress on its sugar maple industry, catastrophic flooding with failing communication infrastructure, and rural isolation affecting ~623,000 people across 9,600 square miles of rugged terrain. No existing system combines ambient RF sensing, swarm AI, quantum-resistant mesh networking, self-learning vector graphs, and agentic robotics into a sovereign, off-grid intelligence layer designed for this reality.

### 2. System Goals

| ID | Goal | Success Metric |
|----|------|----------------|
| G1 | Ambient environmental sensing without cameras | RF-based detection of wildlife, pest swarms, ice damage, smoke density via ESP32 nodes |
| G2 | Self-learning local environment models | Each node builds a vector graph of its microenvironment within 90 days of deployment |
| G3 | Quantum-resistant mesh communication | All inter-node communication uses DAG-based quantum-resistant protocol; zero central servers |
| G4 | Autonomous self-healing network | Node failure detected and routes rerouted within 30 seconds |
| G5 | Decentralized governance | Landowners collectively own and govern the mesh via DAA; no corporate or government control |
| G6 | Flood-resilient communication | Mesh operates independently of cell towers; functional within first 72 hours of major flood |
| G7 | Early pest/disease detection | Detect forest infestations 2+ weeks before human scout programs |
| G8 | Rural presence monitoring | Detect inactivity in isolated homes within 36 hours without cameras or smartphones |
| G9 | Agentic robotic response | Drones/rovers navigate GPS-denied terrain via quantum magnetometers for physical interventions |
| G10 | Multigenerational persistence | System outlives any single operator; gets smarter each season without subscriptions |

### 3. Stakeholders

- **Maple farmers** — micro-climate prediction per sugarbush
- **Conservation districts** — biodiversity monitoring, invasive species detection
- **Rural residents** — presence monitoring, flood alerts, emergency communication
- **Town governance** — collective mesh ownership via town meeting democracy model
- **Emergency services** — flood propagation tracking, road washout prediction

### 4. Functional Requirements

#### 4.1 Node Firmware (Rust — `no_std` on ESP32)

| ID | Requirement |
|----|-------------|
| FR-N1 | WiFi CSI (Channel State Information) capture and local processing for RF-based activity sensing |
| FR-N2 | Onboard sensor fusion: soil moisture, temperature, humidity, barometric pressure, light level |
| FR-N3 | Local vector graph maintenance — incremental learning of environmental signatures |
| FR-N4 | Mesh protocol participation: neighbor discovery, routing table maintenance, data relay |
| FR-N5 | QuDAG message construction and verification |
| FR-N6 | SAFLA feedback loop execution: anomaly detection, pattern propagation, self-rerouting |
| FR-N7 | OTA firmware update reception and verification |
| FR-N8 | Power management: solar/battery monitoring, sleep scheduling, duty cycling |

#### 4.2 Mesh Network Layer (Rust)

| ID | Requirement |
|----|-------------|
| FR-M1 | Peer discovery via BLE beacons and WiFi probe frames |
| FR-M2 | Multi-hop routing with terrain-aware path cost calculation |
| FR-M3 | DAG-based message ordering with quantum-resistant signatures (CRYSTALS-Dilithium) |
| FR-M4 | Anonymous relay: no node knows both sender and final recipient |
| FR-M5 | Epidemic protocol for pattern/signature updates across the mesh |
| FR-M6 | Partition detection and autonomous reconnection strategies |
| FR-M7 | Bandwidth-adaptive compression: degrade gracefully under constrained links |

#### 4.3 Intelligence Layer (Rust)

| ID | Requirement |
|----|-------------|
| FR-I1 | Vector graph neural network per node: encode local environmental state as embeddings |
| FR-I2 | Anomaly detection: compare current state vector against learned seasonal baselines |
| FR-I3 | Cross-node consensus: aggregate anomaly signals from neighboring nodes to confirm events |
| FR-I4 | Event classification: categorize detected anomalies (pest, flood, fire, infrastructure, human) |
| FR-I5 | Predictive modeling: freeze-thaw cycles, sap flow timing, flood propagation |
| FR-I6 | Pattern propagation: when a new threat signature is learned, distribute compressed model delta |

#### 4.4 Agentic Robotics Layer (Rust)

| ID | Requirement |
|----|-------------|
| FR-R1 | Quantum magnetometer-based navigation for GPS-denied mountain valleys |
| FR-R2 | Swarm coordination protocol: task assignment, conflict resolution, return-to-base |
| FR-R3 | Mission types: seed dispersal, water sensor deployment, supply delivery, visual inspection |
| FR-R4 | Mesh integration: robots act as mobile relay nodes when in transit |
| FR-R5 | Safety constraints: no-fly zones, wildlife corridors, weather abort thresholds |

#### 4.5 Governance & UI Layer (TypeScript)

| ID | Requirement |
|----|-------------|
| FR-G1 | Web dashboard: mesh health visualization, node status, event timeline |
| FR-G2 | Governance interface: proposal creation, voting, credit ledger |
| FR-G3 | Farmer portal: per-sugarbush micro-climate view, tap timing recommendations |
| FR-G4 | Alert management: configurable notification rules, escalation chains |
| FR-G5 | Data sovereignty controls: per-landowner data sharing permissions |
| FR-G6 | Offline-first PWA: functions without internet, syncs when gateway available |
| FR-G7 | Map visualization: topographic mesh overlay, event heatmaps, robot positions |

### 5. Non-Functional Requirements

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-1 | Node memory footprint | < 512 KB RAM, < 4 MB flash |
| NFR-2 | Node power consumption | < 50 mW average (solar sustainable) |
| NFR-3 | Message latency (single hop) | < 500 ms |
| NFR-4 | Message latency (10 hops) | < 10 seconds |
| NFR-5 | Node boot time | < 3 seconds |
| NFR-6 | Mesh partition recovery | < 60 seconds |
| NFR-7 | Data retention per node | 2+ years of local environmental history |
| NFR-8 | Operating temperature | -40°C to +60°C (Vermont winters) |
| NFR-9 | Uptime per node | 99.5% annually |
| NFR-10 | Quantum resistance | Post-quantum secure against CRQC with 4096+ logical qubits |

### 6. Constraints

- **No cloud dependency** — all intelligence runs on-mesh or on local gateways
- **No cameras** — all sensing is RF/environmental sensor based
- **No PII storage** — presence detection is local-only, never leaves the property
- **Open source** — all firmware and protocols are publicly auditable
- **Hardware budget** — target < $25 per node at scale (ESP32 + sensors + solar + enclosure)

---

## P — Pseudocode

### 1. Node Main Loop (ESP32 Firmware)

```
PROGRAM verdant_node

CONSTANTS:
    DUTY_CYCLE_MS = 30_000          // wake every 30 seconds
    CSI_SAMPLE_WINDOW_MS = 5_000    // 5s of WiFi CSI capture
    ANOMALY_THRESHOLD = 0.85        // cosine distance from baseline
    MAX_GRAPH_NODES = 2048          // vector graph size limit
    PATTERN_PROPAGATION_TTL = 16    // max hops for pattern updates

STATE:
    vector_graph: VectorGraph        // learned environmental model
    routing_table: RoutingTable      // mesh routing state
    sensor_buffer: RingBuffer        // last 24h of sensor readings
    power_state: PowerState          // battery/solar monitor
    node_id: NodeId                  // derived from hardware, not identity

FUNCTION main_loop():
    initialize_hardware()
    load_vector_graph_from_flash()
    load_routing_table()

    LOOP forever:
        // Phase 1: Sense
        csi_data = capture_wifi_csi(CSI_SAMPLE_WINDOW_MS)
        sensor_reading = read_sensors()  // temp, humidity, soil, pressure, light
        sensor_buffer.push(sensor_reading)

        // Phase 2: Learn
        current_embedding = vector_graph.encode(csi_data, sensor_reading)
        baseline_embedding = vector_graph.seasonal_baseline(current_timestamp())
        anomaly_score = cosine_distance(current_embedding, baseline_embedding)
        vector_graph.update(current_embedding)  // incremental learning

        // Phase 3: Detect
        IF anomaly_score > ANOMALY_THRESHOLD:
            event = classify_anomaly(current_embedding, anomaly_score)
            broadcast_anomaly(event, routing_table)

        // Phase 4: Communicate
        incoming_messages = receive_mesh_messages()
        FOR msg IN incoming_messages:
            MATCH msg.type:
                AnomalyReport => handle_neighbor_anomaly(msg)
                PatternUpdate => merge_pattern_delta(msg.payload, vector_graph)
                RoutingUpdate => routing_table.update(msg)
                GovernanceAction => handle_governance(msg)
                RobotTaskRequest => handle_robot_dispatch(msg)

        // Phase 5: Self-heal (SAFLA)
        run_safla_cycle(routing_table, vector_graph)

        // Phase 6: Power management
        IF power_state.battery_low():
            enter_deep_sleep(extended_duration())
        ELSE:
            sleep(DUTY_CYCLE_MS)

        // Periodic: persist learned state
        IF should_checkpoint():
            save_vector_graph_to_flash()
            save_routing_table()
```

### 2. Vector Graph Learning

```
STRUCTURE VectorGraph:
    nodes: Array<EnvironmentVector>    // embedding nodes
    edges: SparseMatrix<f32>           // similarity edges
    seasonal_centroids: HashMap<SeasonSlot, Vector>  // learned baselines
    version: u64                       // for delta propagation

FUNCTION VectorGraph.encode(csi_data, sensor_reading) -> Vector:
    // Extract features from WiFi CSI
    csi_features = extract_csi_features(csi_data)
        // amplitude variance across subcarriers -> movement detection
        // phase shift patterns -> object size estimation
        // temporal correlation -> periodic vs. transient activity

    // Normalize sensor readings to unit vectors
    sensor_features = normalize([
        sensor_reading.temperature,
        sensor_reading.humidity,
        sensor_reading.soil_moisture,
        sensor_reading.pressure_delta,  // rate of change
        sensor_reading.light_level,
    ])

    // Concatenate and project to fixed-dimension embedding
    raw = concatenate(csi_features, sensor_features)
    embedding = project_to_embedding(raw, self.projection_matrix)

    RETURN embedding

FUNCTION VectorGraph.update(embedding):
    // Find nearest existing node
    nearest, distance = self.nearest_neighbor(embedding)

    IF distance < MERGE_THRESHOLD:
        // Merge into existing node (exponential moving average)
        nearest.vector = 0.95 * nearest.vector + 0.05 * embedding
        nearest.count += 1
    ELSE IF self.nodes.len() < MAX_GRAPH_NODES:
        // Add new node
        new_node = self.nodes.push(embedding)
        self.add_edges(new_node, k_nearest=5)
    ELSE:
        // Replace least-visited node
        victim = self.nodes.min_by(|n| n.count)
        victim.vector = embedding
        victim.count = 1
        self.rebuild_edges(victim)

    // Update seasonal centroid
    season_slot = current_season_slot()  // e.g., "march_week_3"
    self.seasonal_centroids.update_running_mean(season_slot, embedding)

FUNCTION VectorGraph.compute_delta(since_version) -> CompressedDelta:
    changed_nodes = self.nodes.filter(|n| n.last_modified > since_version)
    delta = CompressedDelta {
        added: changed_nodes.map(|n| n.compressed()),
        removed: self.tombstones_since(since_version),
        version: self.version,
    }
    RETURN lz4_compress(delta)
```

### 3. QuDAG Mesh Protocol

```
STRUCTURE QuDAGMessage:
    dag_parents: Vec<MessageHash>      // references to prior messages (DAG structure)
    payload: EncryptedPayload
    signature: DilithiumSignature      // post-quantum signature
    ttl: u8
    timestamp: u64

FUNCTION send_message(payload, destination_zone, routing_table):
    // Encrypt with recipient zone's public key (anonymous routing)
    encrypted = encrypt_for_zone(payload, destination_zone)

    // Build DAG reference
    recent_tips = routing_table.get_dag_tips(max=3)

    msg = QuDAGMessage {
        dag_parents: recent_tips,
        payload: encrypted,
        signature: sign_dilithium(encrypted, self.private_key),
        ttl: compute_ttl(destination_zone),
        timestamp: monotonic_timestamp(),
    }

    // Select relay path (onion-style, 3 hops minimum)
    path = select_anonymous_path(routing_table, destination_zone, min_hops=3)

    FOR next_hop IN path:
        msg = wrap_relay_layer(msg, next_hop.public_key)

    transmit_to_neighbor(path[0], msg)

FUNCTION receive_and_relay(msg):
    // Verify DAG consistency
    IF NOT verify_dag_parents(msg.dag_parents):
        DROP msg  // orphaned or replayed

    // Verify post-quantum signature
    IF NOT verify_dilithium(msg.signature, msg.payload):
        DROP msg

    // Try to unwrap our relay layer
    inner = try_unwrap_relay_layer(msg, self.private_key)

    IF inner.is_for_us():
        decrypted = decrypt_payload(inner.payload, self.private_key)
        process_local(decrypted)
    ELSE IF msg.ttl > 0:
        msg.ttl -= 1
        forward_to_next_hop(inner)
```

### 4. SAFLA Self-Healing Cycle

```
FUNCTION run_safla_cycle(routing_table, vector_graph):
    // 1. Health assessment
    FOR neighbor IN routing_table.neighbors():
        IF neighbor.last_seen > TIMEOUT_THRESHOLD:
            mark_suspect(neighbor)
            IF neighbor.consecutive_misses > 3:
                mark_dead(neighbor)
                reroute_around(neighbor, routing_table)

    // 2. Anomaly consensus
    local_anomalies = vector_graph.recent_anomalies()
    neighbor_anomalies = collect_neighbor_anomalies()

    FOR anomaly IN local_anomalies:
        corroborating = neighbor_anomalies.filter(|a|
            spatial_proximity(a, anomaly) < ZONE_RADIUS
            AND temporal_proximity(a, anomaly) < 300_SECONDS
            AND category_match(a, anomaly)
        )
        IF corroborating.len() >= CONSENSUS_QUORUM:
            confirmed_event = ConfirmedEvent {
                category: anomaly.category,
                confidence: compute_confidence(anomaly, corroborating),
                affected_zone: compute_affected_zone(anomaly, corroborating),
                recommended_action: determine_action(anomaly.category),
            }
            propagate_confirmed_event(confirmed_event)

    // 3. Pattern learning propagation
    IF vector_graph.has_new_patterns():
        delta = vector_graph.compute_delta(last_propagated_version)
        IF delta.is_significant():
            broadcast_pattern_update(delta, ttl=PATTERN_PROPAGATION_TTL)

    // 4. Network topology optimization
    IF routing_table.should_rebalance():
        propose_topology_change(routing_table)
```

### 5. Flood Propagation Prediction

```
FUNCTION predict_flood_propagation(upstream_event, watershed_graph):
    // watershed_graph: learned topology of water flow between nodes
    affected_nodes = priority_queue()
    affected_nodes.push(upstream_event.origin, time=0)

    visited = set()

    WHILE NOT affected_nodes.is_empty():
        current, arrival_time = affected_nodes.pop()
        IF current IN visited: CONTINUE
        visited.add(current)

        // Alert this node
        send_preemptive_alert(current, FloodAlert {
            estimated_arrival: arrival_time,
            severity: estimate_severity(upstream_event, current),
            recommended_action: evacuation_or_prepare(current),
        })

        // Propagate downstream
        FOR downstream IN watershed_graph.downstream_neighbors(current):
            flow_time = watershed_graph.flow_time(current, downstream)
            // Adjust for current soil saturation
            saturation = downstream.vector_graph.current_soil_moisture()
            adjusted_time = flow_time * saturation_factor(saturation)
            affected_nodes.push(downstream, arrival_time + adjusted_time)
```

### 6. Governance Protocol

```
STRUCTURE Proposal:
    id: ProposalHash
    proposer_zone: ZoneId       // anonymous — zone, not individual
    title: String
    action: GovernanceAction    // enum: AddNode, RemoveNode, UpdatePolicy, AllocateCredits
    voting_deadline: Timestamp
    votes: HashMap<ZoneId, Vote>
    quorum: f32                 // fraction of zones that must vote

FUNCTION submit_proposal(proposal):
    proposal.id = hash(proposal)
    broadcast_to_mesh(GovernanceMessage::NewProposal(proposal))

FUNCTION cast_vote(proposal_id, vote, zone_private_key):
    signed_vote = sign_dilithium(vote, zone_private_key)
    broadcast_to_mesh(GovernanceMessage::Vote(proposal_id, signed_vote))

FUNCTION tally_votes(proposal):
    IF current_time() < proposal.voting_deadline:
        RETURN Pending

    total_zones = mesh.active_zone_count()
    votes_cast = proposal.votes.len()

    IF votes_cast < total_zones * proposal.quorum:
        RETURN FailedQuorum

    yes_count = proposal.votes.values().filter(|v| v == Yes).count()
    IF yes_count > votes_cast / 2:
        execute_governance_action(proposal.action)
        RETURN Passed
    ELSE:
        RETURN Rejected
```

---

## A — Architecture

### 1. System Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        VERDANT SYSTEM ARCHITECTURE                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                  │
│  │  ESP32 Node   │  │  ESP32 Node   │  │  ESP32 Node   │  ... (1000s)  │
│  │  ┌──────────┐ │  │  ┌──────────┐ │  │  ┌──────────┐ │               │
│  │  │ Firmware  │ │  │  │ Firmware  │ │  │  │ Firmware  │ │               │
│  │  │ (Rust)    │ │  │  │ (Rust)    │ │  │  │ (Rust)    │ │               │
│  │  └──────────┘ │  │  └──────────┘ │  │  └──────────┘ │               │
│  │  WiFi│BLE│GPIO│  │  WiFi│BLE│GPIO│  │  WiFi│BLE│GPIO│               │
│  └──────┼───┼────┘  └──────┼───┼────┘  └──────┼───┼────┘               │
│         │   │              │   │              │   │                      │
│    ─────┴───┴──────────────┴───┴──────────────┴───┴──────               │
│              MESH NETWORK LAYER (QuDAG Protocol)                        │
│    ──────────────────────────────────────────────────────               │
│         │                                    │                          │
│  ┌──────┴───────┐                    ┌───────┴──────┐                   │
│  │ Gateway Node  │                    │ Gateway Node  │                  │
│  │ (Rust on RPi) │                    │ (Rust on RPi) │                  │
│  │ ┌───────────┐ │                    │ ┌───────────┐ │                  │
│  │ │ Local DB   │ │                    │ │ Local DB   │ │                  │
│  │ │ (sled/     │ │                    │ │ (sled/     │ │                  │
│  │ │  redb)     │ │                    │ │  redb)     │ │                  │
│  │ └───────────┘ │                    │ └───────────┘ │                  │
│  └──────┬───────┘                    └───────┬──────┘                   │
│         │ (optional, when available)          │                          │
│  ┌──────┴────────────────────────────────────┴──────┐                   │
│  │              LOCAL NETWORK / SNEAKERNET           │                   │
│  │         (sync when connectivity exists)           │                   │
│  └──────────────────────┬───────────────────────────┘                   │
│                         │                                               │
│  ┌──────────────────────┴───────────────────────────┐                   │
│  │            DASHBOARD (TypeScript PWA)             │                   │
│  │  ┌────────┐ ┌──────────┐ ┌──────┐ ┌───────────┐  │                  │
│  │  │ Map    │ │ Governance│ │ Alerts│ │ Farm View │  │                  │
│  │  │ View   │ │ Portal   │ │ Mgmt │ │           │  │                  │
│  │  └────────┘ └──────────┘ └──────┘ └───────────┘  │                  │
│  └──────────────────────────────────────────────────┘                   │
│                                                                         │
│  ┌──────────────────────────────────────────────────┐                   │
│  │          AGENTIC ROBOTICS LAYER (Rust)            │                  │
│  │  ┌─────────┐  ┌────────────┐  ┌───────────────┐  │                  │
│  │  │ Swarm   │  │ Quantum Mag│  │ Mission       │  │                  │
│  │  │ Coord.  │  │ Navigation │  │ Execution     │  │                  │
│  │  └─────────┘  └────────────┘  └───────────────┘  │                  │
│  └──────────────────────────────────────────────────┘                   │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2. Crate Architecture (Rust Workspace)

```
verdant/
├── Cargo.toml                          # workspace root
│
├── crates/
│   ├── verdant-core/                   # shared types, traits, constants
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── types.rs                # NodeId, ZoneId, Timestamp, etc.
│   │   │   ├── traits.rs               # Sensor, Transport, Storage traits
│   │   │   ├── config.rs               # compile-time and runtime config
│   │   │   └── error.rs                # unified error types
│   │   └── Cargo.toml                  # no_std compatible
│   │
│   ├── verdant-vector/                 # vector graph neural network
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── graph.rs                # VectorGraph structure
│   │   │   ├── embedding.rs            # feature extraction & projection
│   │   │   ├── anomaly.rs              # anomaly detection & scoring
│   │   │   ├── seasonal.rs             # seasonal baseline learning
│   │   │   ├── delta.rs                # compressed delta computation
│   │   │   └── alloc.rs                # custom allocator for no_std
│   │   └── Cargo.toml                  # no_std, uses nalgebra
│   │
│   ├── verdant-mesh/                   # mesh networking layer
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── discovery.rs            # BLE/WiFi peer discovery
│   │   │   ├── routing.rs              # multi-hop terrain-aware routing
│   │   │   ├── transport.rs            # frame encoding/decoding
│   │   │   ├── partition.rs            # partition detection & recovery
│   │   │   └── compression.rs          # bandwidth-adaptive compression
│   │   └── Cargo.toml                  # no_std
│   │
│   ├── verdant-qudag/                  # quantum-resistant DAG protocol
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── dag.rs                  # DAG construction & validation
│   │   │   ├── crypto.rs               # CRYSTALS-Dilithium, CRYSTALS-Kyber
│   │   │   ├── anonymity.rs            # onion relay wrapping
│   │   │   └── message.rs              # message types & serialization
│   │   └── Cargo.toml                  # uses pqcrypto crate
│   │
│   ├── verdant-safla/                  # self-aware feedback loop
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── health.rs               # node/neighbor health monitoring
│   │   │   ├── consensus.rs            # cross-node anomaly consensus
│   │   │   ├── propagation.rs          # pattern update propagation
│   │   │   ├── topology.rs             # network topology optimization
│   │   │   └── events.rs               # confirmed event types & actions
│   │   └── Cargo.toml
│   │
│   ├── verdant-sense/                  # sensor abstraction layer
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── csi.rs                  # WiFi CSI capture & feature extraction
│   │   │   ├── environmental.rs        # temp, humidity, soil, pressure, light
│   │   │   ├── fusion.rs               # multi-sensor fusion
│   │   │   └── hal.rs                  # hardware abstraction (ESP32 peripherals)
│   │   └── Cargo.toml                  # esp-hal dependency
│   │
│   ├── verdant-firmware/               # ESP32 node firmware binary
│   │   ├── src/
│   │   │   ├── main.rs                 # entry point, main loop
│   │   │   ├── power.rs                # power management, sleep scheduling
│   │   │   ├── storage.rs              # flash persistence (vector graph, routing)
│   │   │   └── ota.rs                  # over-the-air update handler
│   │   └── Cargo.toml                  # target = xtensa-esp32-none-elf
│   │
│   ├── verdant-gateway/                # gateway daemon (Raspberry Pi)
│   │   ├── src/
│   │   │   ├── main.rs                 # gateway service entry point
│   │   │   ├── bridge.rs               # mesh-to-local-network bridge
│   │   │   ├── db.rs                   # local database (redb)
│   │   │   ├── api.rs                  # REST/WebSocket API for dashboard
│   │   │   ├── sync.rs                 # gateway-to-gateway sync
│   │   │   └── governance.rs           # governance state machine
│   │   └── Cargo.toml                  # axum, redb, tokio
│   │
│   ├── verdant-robotics/               # agentic robotics coordination
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── swarm.rs                # swarm coordination protocol
│   │   │   ├── navigation.rs           # quantum magnetometer navigation
│   │   │   ├── mission.rs              # mission planning & execution
│   │   │   ├── safety.rs               # geofencing, weather abort, corridors
│   │   │   └── relay.rs                # mobile relay node behavior
│   │   └── Cargo.toml
│   │
│   └── verdant-sim/                    # network simulator for testing
│       ├── src/
│       │   ├── main.rs                 # simulation harness
│       │   ├── terrain.rs              # Vermont terrain model
│       │   ├── weather.rs              # weather pattern simulation
│       │   └── scenario.rs             # test scenarios (flood, pest, partition)
│       └── Cargo.toml
│
├── ui/                                 # TypeScript PWA dashboard
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── src/
│   │   ├── main.ts
│   │   ├── App.tsx
│   │   ├── components/
│   │   │   ├── MeshMap.tsx             # topographic mesh overlay (Mapbox/Leaflet)
│   │   │   ├── NodeStatus.tsx          # individual node detail view
│   │   │   ├── EventTimeline.tsx       # confirmed event stream
│   │   │   ├── AlertManager.tsx        # alert configuration & display
│   │   │   ├── GovernancePortal.tsx    # proposals, voting, credits
│   │   │   ├── FarmDashboard.tsx       # per-sugarbush micro-climate
│   │   │   ├── FloodTracker.tsx        # real-time flood propagation map
│   │   │   └── RobotOverview.tsx       # drone/rover status & missions
│   │   ├── hooks/
│   │   │   ├── useMeshData.ts          # WebSocket connection to gateway
│   │   │   ├── useOfflineSync.ts       # IndexedDB offline-first sync
│   │   │   └── useGovernance.ts        # governance state management
│   │   ├── lib/
│   │   │   ├── qudag-verify.ts         # WASM-compiled QuDAG verification
│   │   │   ├── terrain.ts              # terrain rendering utilities
│   │   │   └── types.ts                # shared type definitions
│   │   └── workers/
│   │       └── sync-worker.ts          # background sync service worker
│   └── public/
│       └── manifest.json               # PWA manifest
│
└── tools/
    ├── flash.sh                        # ESP32 flashing script
    ├── sim-scenario.sh                 # run simulation scenarios
    └── generate-terrain.py             # generate Vermont terrain mesh from DEM data
```

### 3. Data Flow Architecture

```
┌─────────────────── SINGLE NODE DATA FLOW ───────────────────┐
│                                                              │
│  WiFi Antenna ──► CSI Capture ──► Feature Extraction ──┐    │
│                                                         │    │
│  Sensors ──► ADC Readings ──► Normalization ──────────┐│    │
│                                                        ││    │
│                                    ┌───────────────────┘│    │
│                                    │  Sensor Fusion     │    │
│                                    └────────┬───────────┘    │
│                                             │                │
│                                    ┌────────▼────────┐       │
│                                    │  Vector Graph    │       │
│                                    │  Encode          │       │
│                                    └────────┬────────┘       │
│                                             │                │
│                              ┌──────────────┼──────────┐     │
│                              │              │          │     │
│                    ┌─────────▼───┐  ┌───────▼──┐  ┌───▼───┐ │
│                    │ Update Graph│  │ Anomaly  │  │ Store │ │
│                    │ (learn)     │  │ Detect   │  │ Flash │ │
│                    └─────────────┘  └────┬─────┘  └───────┘ │
│                                          │                   │
│                                    ┌─────▼──────┐           │
│                                    │ Classify    │           │
│                                    └─────┬──────┘           │
│                                          │                   │
│                              ┌───────────▼───────────┐      │
│                              │ SAFLA Consensus Check  │      │
│                              │ (with neighbor data)   │      │
│                              └───────────┬───────────┘      │
│                                          │                   │
│                         ┌────────────────┼────────────┐      │
│                         │                │            │      │
│                  ┌──────▼──────┐  ┌──────▼─────┐ ┌───▼───┐  │
│                  │ Local Alert │  │ Mesh Broad- │ │ Robot │  │
│                  │ (presence)  │  │ cast Event  │ │ Task  │  │
│                  └─────────────┘  └────────────┘ └───────┘  │
└──────────────────────────────────────────────────────────────┘
```

### 4. Mesh Topology Model

```
                    Zone A (Sugarbush)
                   ╱    │    ╲
                 N1 ── N2 ── N3
                 │      │      │
    Zone B ──── N4 ── N5 ── N6 ──── Zone D
   (Watershed)   │      │      │    (Town Center)
                 N7 ── N8 ── N9
                   ╲    │    ╱
                    Zone C (Forest)
                         │
                    ┌────┴────┐
                    │ Gateway  │ ←── optional internet
                    │  (RPi)   │     uplink
                    └─────────┘

    Each zone: 10-50 nodes, self-organizing
    Inter-zone: relay nodes with overlapping range
    Gateway: 1 per community, aggregates & serves dashboard
```

### 5. Technology Stack

| Layer | Technology | Justification |
|-------|-----------|---------------|
| Node firmware | Rust (`no_std`) + `esp-hal` | Memory safety on embedded, zero-cost abstractions |
| Vector math | `nalgebra` (no_std) + `micromath` | Fixed-point math for ESP32 without FPU |
| Post-quantum crypto | `pqcrypto` (CRYSTALS-Dilithium/Kyber) | NIST-standardized PQ algorithms |
| Mesh transport | Custom over WiFi/BLE (`esp-wifi`) | No dependency on external protocols |
| Gateway runtime | `tokio` + `axum` | Async Rust for API serving |
| Gateway DB | `redb` | Embedded, ACID, single-file, pure Rust |
| Serialization | `postcard` (no_std) | Compact binary, serde-compatible |
| Compression | `lz4_flex` (no_std) | Fast compression for constrained devices |
| UI framework | React + TypeScript | Component-based PWA with offline support |
| UI maps | Leaflet + OpenTopoMap | Open-source topo maps, works offline |
| UI state | Zustand | Lightweight, works well with offline-first |
| WASM bridge | `wasm-bindgen` | QuDAG verification in browser |
| Simulation | Rust + `bevy_ecs` | ECS for simulating 1000s of nodes efficiently |

### 6. Security Architecture

```
┌─────────────── SECURITY LAYERS ──────────────────┐
│                                                    │
│  Layer 1: Node Identity                            │
│  ├── Hardware-derived keys (ESP32 eFuse)           │
│  ├── No PII — nodes identified by zone + index     │
│  └── Key rotation via governance vote               │
│                                                    │
│  Layer 2: Transport Security                       │
│  ├── CRYSTALS-Kyber for key encapsulation          │
│  ├── CRYSTALS-Dilithium for signatures             │
│  ├── Onion routing (3+ hops) for anonymity         │
│  └── DAG ordering prevents replay attacks           │
│                                                    │
│  Layer 3: Data Sovereignty                         │
│  ├── Presence data NEVER leaves property            │
│  ├── Environmental data shared only with consent    │
│  ├── Per-zone encryption: only zone members decrypt │
│  └── No central server — no single point to breach  │
│                                                    │
│  Layer 4: Governance Security                      │
│  ├── Zone-level voting (not individual)             │
│  ├── Quorum requirements for all changes            │
│  ├── Firmware updates require supermajority vote    │
│  └── Transparent, auditable decision log (DAG)      │
│                                                    │
│  Layer 5: Physical Security                        │
│  ├── Tamper detection (accelerometer/case switch)   │
│  ├── Secure boot chain on ESP32                     │
│  └── Node compromise doesn't compromise mesh        │
│       (compartmentalized zone keys)                 │
└────────────────────────────────────────────────────┘
```

---

## R — Refinement

### 1. Edge Cases & Mitigations

| Edge Case | Impact | Mitigation |
|-----------|--------|------------|
| Deep winter: solar panels covered by snow | Node power loss for weeks | Supercapacitor buffer + aggressive duty cycling; neighbors detect silence and increase their own monitoring of the silent node's zone |
| Black bear damages node | Physical destruction | Enclosure rated for wildlife; mesh self-heals by rerouting; redundant coverage (3+ nodes per zone) |
| Flash flood destroys cluster of nodes | Large partition, exactly when flood alerting is needed | Upstream nodes detect anomaly before destruction propagates; alerts sent pre-emptively; robots can act as emergency relay |
| False positive: logging trucks detected as "invasion" | Alert fatigue | Temporal + spatial consensus: a single node anomaly that neighbors don't corroborate gets scored down. Seasonal calendars (logging season) reduce sensitivity |
| Adversarial node injection | Mesh poisoning | Nodes must be provisioned by governance vote; unknown hardware IDs rejected at mesh layer; onion routing limits damage of compromised relay |
| Vector graph fills flash storage | Node stops learning | LRU eviction of least-visited graph nodes; seasonal compaction merges similar nodes |
| Vermont ice storm: all gateways offline | Dashboard inaccessible | Mesh continues autonomously; alerts propagate node-to-node; dashboard syncs when gateway recovers |
| Gradual climate shift: baselines drift | Anomaly detection becomes insensitive | Seasonal centroids use 3-year sliding window; annual recalibration cycle compares to historical norms |

### 2. Performance Optimizations

#### 2.1 ESP32 Compute Budget

```
Available per wake cycle (30s interval, 50mW average):
  - Active time: ~5 seconds at 80MHz = 400M cycles
  - Budget allocation:
    - CSI capture & feature extraction:  100M cycles (25%)
    - Vector graph encode + update:       80M cycles (20%)
    - Anomaly detection:                  40M cycles (10%)
    - Mesh protocol (send/receive):       80M cycles (20%)
    - SAFLA cycle:                        40M cycles (10%)
    - Crypto operations:                  40M cycles (10%)
    - Overhead (OS, scheduling):          20M cycles ( 5%)
```

#### 2.2 Memory Layout (512 KB SRAM)

```
Stack:          32 KB
Heap:           64 KB
WiFi buffers:  128 KB (ESP-IDF requirement)
CSI buffer:     16 KB (ring buffer, last 5s)
Sensor buffer:   8 KB (last 24h at 30s intervals = 2880 readings)
Vector graph:  200 KB (2048 nodes × ~100 bytes each)
Routing table:  32 KB (up to 256 known peers)
Message queue:  16 KB (outgoing)
Crypto state:    8 KB (keys, nonces)
Free:            8 KB
```

#### 2.3 Flash Persistence Strategy

```
Partition map (4 MB flash):
  - Firmware A:      1.5 MB  (active)
  - Firmware B:      1.5 MB  (OTA staging)
  - Vector graph:    768 KB  (wear-leveled)
  - Config/keys:      64 KB  (encrypted)
  - Event log:       128 KB  (circular)
  - Reserved:         64 KB

Write strategy:
  - Vector graph checkpoint: every 1 hour (not every cycle)
  - Use delta writes to minimize flash wear
  - Estimated flash lifetime: 10+ years at this write rate
```

### 3. Testing Strategy

| Level | Approach | Tools |
|-------|----------|-------|
| Unit | Property-based testing of vector graph operations, DAG validation, routing algorithms | `proptest`, `quickcheck` |
| Integration | Multi-node simulation: spawn 100+ virtual nodes, inject events, verify consensus | `verdant-sim` crate with `bevy_ecs` |
| Hardware-in-loop | ESP32 dev boards on bench with RF shielding, inject known CSI patterns | `probe-rs`, `defmt` logging |
| Scenario | Simulate specific Vermont events: 2023 flood, emerald ash borer spread pattern, ice storm | `verdant-sim` with historical data |
| Adversarial | Attempt mesh poisoning, replay attacks, Sybil attacks against test network | Custom fuzzer targeting `verdant-qudag` |
| Long-duration | 30-day soak test: real nodes on a Vermont hillside logging all behavior | Manual deployment + `verdant-gateway` telemetry |

### 4. Deployment Strategy

```
Phase 1: SIMULATION (Months 1-4)
├── Build verdant-sim with Vermont terrain data
├── Simulate 1000-node mesh, validate routing convergence
├── Inject historical weather data, verify anomaly detection
└── Load test QuDAG protocol for message throughput

Phase 2: BENCH (Months 5-7)
├── 10 ESP32 nodes on a bench
├── Validate power budget on real hardware
├── Test CSI feature extraction with controlled RF environment
├── Verify flash persistence and OTA updates
└── Gateway + dashboard integration

Phase 3: PILOT (Months 8-12)
├── Deploy 50 nodes across one Vermont sugarbush (~40 acres)
├── One gateway at farmhouse
├── Monitor through first maple season
├── Collect ground-truth for vector graph calibration
└── Iterate on anomaly thresholds

Phase 4: EXPANSION (Year 2)
├── Scale to 500 nodes across a watershed
├── Multi-gateway sync testing
├── First governance votes
├── Integrate first robotic platform (ground rover)
└── Partner with Vermont conservation district

Phase 5: STATEWIDE (Year 3+)
├── Open-source toolkit for community self-deployment
├── Training materials for Vermont Extension Service
├── Drone integration for mountain terrain
└── Multigenerational handoff documentation
```

### 5. Regulatory & Ethical Considerations

| Area | Consideration | Approach |
|------|--------------|----------|
| FCC | WiFi CSI capture operates within existing WiFi allocations | No additional licensing needed for passive CSI |
| FAA | Drone operations in rural areas | Part 107 compliance; geofencing built into `verdant-robotics/safety.rs` |
| Privacy | RF sensing could theoretically detect human activity | Presence data is LOCAL ONLY; never transmitted; configurable opt-out per node |
| Wildlife | Nodes in forests must not harm habitat | Enclosures designed with VT Fish & Wildlife input; no light/sound emissions |
| Data sovereignty | Vermont culture demands local control | No cloud, no corporate API; governance is on-mesh |
| Right to repair | Farmers must be able to fix their own nodes | Open hardware designs; standard connectors; documented repair procedures |

---

## C — Completion

### 1. Implementation Checklist

#### Phase 1: Foundation (Weeks 1-8)

- [ ] **`verdant-core`**: Define all shared types, traits, error types, config
- [ ] **`verdant-vector`**: Implement VectorGraph with `no_std` nalgebra
  - [ ] `graph.rs`: Insert, merge, evict, nearest-neighbor
  - [ ] `embedding.rs`: Feature projection with fixed-point math
  - [ ] `seasonal.rs`: Running mean seasonal centroids
  - [ ] `delta.rs`: Compressed delta serialization with postcard + lz4
  - [ ] `anomaly.rs`: Cosine distance anomaly scoring
  - [ ] Property tests for all graph operations
- [ ] **`verdant-qudag`**: Implement post-quantum DAG protocol
  - [ ] `crypto.rs`: Dilithium sign/verify, Kyber encapsulate/decapsulate
  - [ ] `dag.rs`: DAG parent tracking, orphan rejection, tip selection
  - [ ] `message.rs`: Serialize/deserialize with postcard
  - [ ] `anonymity.rs`: Onion wrapping/unwrapping
  - [ ] Fuzz testing of message parsing
- [ ] **`verdant-mesh`**: Basic mesh networking
  - [ ] `discovery.rs`: BLE beacon broadcast/scan
  - [ ] `routing.rs`: Distance-vector routing with terrain cost
  - [ ] `transport.rs`: WiFi frame encode/decode
  - [ ] Integration test: 10-node virtual mesh converges

#### Phase 2: Intelligence (Weeks 9-14)

- [ ] **`verdant-sense`**: Sensor abstraction
  - [ ] `csi.rs`: WiFi CSI capture on ESP32, subcarrier amplitude/phase extraction
  - [ ] `environmental.rs`: I2C/SPI sensor drivers (BME280, soil moisture, BH1750)
  - [ ] `fusion.rs`: Multi-sensor feature vector construction
  - [ ] `hal.rs`: ESP32 peripheral initialization
- [ ] **`verdant-safla`**: Self-healing feedback loop
  - [ ] `health.rs`: Neighbor timeout detection, suspect/dead marking
  - [ ] `consensus.rs`: Spatial-temporal anomaly correlation
  - [ ] `propagation.rs`: Pattern delta broadcast with TTL
  - [ ] `topology.rs`: Route rebalancing proposals
  - [ ] `events.rs`: Event classification (pest, flood, fire, infrastructure, human)
- [ ] **`verdant-sim`**: Network simulator
  - [ ] `terrain.rs`: Load Vermont DEM data, compute RF propagation
  - [ ] `weather.rs`: Historical weather replay
  - [ ] `scenario.rs`: Flood, pest, partition scenarios
  - [ ] Benchmark: simulate 1000 nodes with anomaly injection

#### Phase 3: Firmware (Weeks 15-20)

- [ ] **`verdant-firmware`**: ESP32 binary
  - [ ] `main.rs`: Wake → Sense → Learn → Detect → Communicate → Sleep loop
  - [ ] `power.rs`: Battery/solar monitor, adaptive duty cycling
  - [ ] `storage.rs`: Flash wear-leveled persistence for vector graph
  - [ ] `ota.rs`: Firmware update reception, dual-partition swap
  - [ ] Hardware-in-loop tests on ESP32-S3 dev boards
- [ ] **`verdant-robotics`**: Robotic coordination (stub for Phase 1)
  - [ ] `swarm.rs`: Task assignment protocol definition
  - [ ] `navigation.rs`: Magnetometer integration interface
  - [ ] `mission.rs`: Mission state machine
  - [ ] `safety.rs`: Geofence data structure and abort conditions

#### Phase 4: Gateway & Dashboard (Weeks 21-28)

- [ ] **`verdant-gateway`**: Raspberry Pi gateway daemon
  - [ ] `bridge.rs`: Mesh radio ↔ local network bridge
  - [ ] `db.rs`: redb schema for events, node status, governance state
  - [ ] `api.rs`: axum REST + WebSocket endpoints
  - [ ] `sync.rs`: Gateway-to-gateway CRDT-based sync
  - [ ] `governance.rs`: Proposal lifecycle state machine
- [ ] **`ui/`**: TypeScript PWA
  - [ ] `MeshMap.tsx`: Leaflet + OpenTopoMap with node overlay
  - [ ] `EventTimeline.tsx`: Real-time event stream via WebSocket
  - [ ] `AlertManager.tsx`: Configurable alert rules
  - [ ] `GovernancePortal.tsx`: Proposal create/vote/view
  - [ ] `FarmDashboard.tsx`: Per-sugarbush micro-climate charts
  - [ ] `FloodTracker.tsx`: Animated flood propagation overlay
  - [ ] Service worker for offline-first operation
  - [ ] WASM-compiled QuDAG verifier for in-browser validation

#### Phase 5: Integration & Pilot (Weeks 29-36)

- [ ] End-to-end integration: firmware → mesh → gateway → dashboard
- [ ] 50-node field deployment on Vermont sugarbush
- [ ] Vector graph calibration with ground-truth environmental data
- [ ] First maple season monitoring
- [ ] Anomaly threshold tuning based on real-world false positive rates
- [ ] Gateway failover testing
- [ ] Security audit of QuDAG protocol

### 2. Verification Criteria

| Requirement | Verification Method | Pass Criteria |
|-------------|-------------------|---------------|
| G1: RF sensing without cameras | Deploy node near known wildlife corridor; compare CSI detections to trail camera | 80%+ correlation with trail camera events |
| G2: Self-learning in 90 days | Deploy node, wait 90 days, measure anomaly detection accuracy | False positive rate < 5% after 90 days |
| G3: Quantum-resistant comms | Submit protocol to external cryptographic review | No vulnerabilities found in PQ primitives |
| G4: Self-healing < 30s | Kill a relay node in simulation; measure reroute time | 95th percentile reroute < 30 seconds |
| G5: Decentralized governance | Run governance vote across 50 nodes; verify no central authority needed | Vote completes with 100% consistency |
| G6: Flood-resilient comms | Simulate cell tower failure; verify mesh continues operating | All nodes reachable within 60s of tower loss |
| G7: Early pest detection | Inject known pest RF signature; measure detection lead time | Detection 14+ days before visual symptoms |
| G8: Presence monitoring | Test with known occupancy patterns; verify alert accuracy | 99%+ accuracy on 36-hour inactivity detection |
| NFR-1: Memory < 512 KB | Measure peak RAM usage with `defmt` instrumentation | Peak usage < 480 KB |
| NFR-2: Power < 50 mW avg | Measure with INA219 current sensor over 24 hours | Average < 50 mW, peak < 300 mW |
| NFR-5: Boot < 3 seconds | Measure boot-to-first-sense time | < 3 seconds cold boot |

### 3. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| ESP32 CSI API instability across SDK versions | High | Medium | Pin esp-idf version; abstract CSI behind trait for portability |
| Post-quantum crypto too slow for ESP32 | Medium | High | Benchmark early; fall back to hybrid (Ed25519 + Dilithium) if needed |
| Vermont winter destroys pilot nodes | Medium | Medium | Ruggedized enclosures; deploy extras; this validates design |
| Flash wear from vector graph writes | Low | High | Checkpoint hourly, not per-cycle; wear-leveling in `storage.rs` |
| Community adoption resistance | Medium | High | Partner with Vermont Extension Service; lead with maple farmer value |
| RF interference in dense forest | Medium | Medium | Terrain-aware routing; adaptive transmission power; denser node placement |

### 4. File Manifest

```
Total new Rust crates:     10
Total Rust source files:   ~45
Total TypeScript files:    ~20
Estimated lines of Rust:   ~25,000
Estimated lines of TS:     ~8,000
```

### 5. Dependencies (Key Crates)

```toml
# Embedded (no_std)
esp-hal = "0.23"
esp-wifi = "0.12"
embassy-executor = "0.7"
nalgebra = { version = "0.33", default-features = false }
micromath = "2.1"
postcard = "1.1"
lz4_flex = { version = "0.11", default-features = false }
pqcrypto-dilithium = "0.5"
pqcrypto-kyber = "0.8"
heapless = "0.8"
defmt = "0.3"

# Gateway (std)
tokio = { version = "1", features = ["full"] }
axum = "0.8"
redb = "2.4"
serde = { version = "1", features = ["derive"] }
tracing = "0.1"

# Simulation
bevy_ecs = "0.15"
rand = "0.8"
```

### 6. Open Questions for Stakeholder Input

1. **Node density**: What is the target inter-node distance? (Proposed: 200-500 feet for sugarbush, 0.5-1 mile for forest)
2. **Gateway hardware**: Raspberry Pi 4/5 or move to something more ruggedized?
3. **Governance quorum**: What percentage of zones must vote for different action types?
4. **Data sharing defaults**: Opt-in or opt-out for inter-zone environmental data sharing?
5. **Robot platform**: Start with ground rover (simpler) or aerial drone (more capable in Vermont terrain)?
6. **Regulatory pre-clearance**: Engage VT PUC or FCC before pilot, or deploy within existing allocations?
