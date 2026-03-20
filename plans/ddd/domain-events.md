# Verdant Domain Events

## Event-Driven Architecture with London School TDD

Domain events flow between bounded contexts via the mesh protocol. Each event is tested by mocking the publisher/subscriber boundary.

---

## Event Catalog

### Environmental Intelligence Events

```rust
/// Emitted when a node's anomaly score exceeds the threshold.
/// Scope: local node → mesh broadcast
pub struct AnomalyDetected {
    pub source_zone: ZoneId,        // never source_node (privacy)
    pub category: EventCategory,
    pub score: AnomalyScore,
    pub embedding_hash: EmbeddingHash,  // for deduplication
    pub timestamp: Timestamp,
}

/// Emitted when the vector graph learns a significant new pattern.
/// Scope: local node → epidemic broadcast (TTL=16)
pub struct PatternLearned {
    pub source_zone: ZoneId,
    pub delta: CompressedDelta,
    pub delta_size_bytes: u32,
    pub graph_version: Version,
    pub timestamp: Timestamp,
}

/// Emitted when seasonal baseline recalibration completes.
/// Scope: local node only (not transmitted)
pub struct BaselineRecalibrated {
    pub season_slot: SeasonSlot,
    pub observation_count: u32,
    pub centroid_shift: f32,   // how much the centroid moved
    pub timestamp: Timestamp,
}
```

### SAFLA Events

```rust
/// Emitted when cross-node consensus confirms an environmental event.
/// This is the primary event that triggers downstream actions.
/// Scope: mesh-wide broadcast
pub struct EventConfirmed {
    pub event_id: EventId,
    pub category: EventCategory,
    pub confidence: f32,              // 0.0 to 1.0
    pub affected_zone: ZoneId,
    pub corroborating_count: u8,
    pub recommended_action: RecommendedAction,
    pub timestamp: Timestamp,
}

pub enum EventCategory {
    Pest { species_hint: Option<PestSpecies> },
    Flood { severity: FloodSeverity, upstream_origin: Option<ZoneId> },
    Fire { smoke_density: f32 },
    Infrastructure { sub_type: InfrastructureType },
    Wildlife { movement_type: MovementType },
    Climate { sub_type: ClimateType },
}

pub enum RecommendedAction {
    AlertOnly,
    AlertAndMonitor,
    DispatchRobot { mission_type: MissionType },
    EvacuationWarning,
    PreemptiveAlert { downstream_zones: Vec<ZoneId> },
}

/// Emitted when a neighbor node is declared dead.
/// Scope: routing table update, local
pub struct NeighborDeclaredDead {
    pub dead_node: NodeId,
    pub last_seen: Timestamp,
    pub consecutive_misses: u8,
    pub reroute_success: bool,
    pub timestamp: Timestamp,
}

/// Emitted when mesh partition is detected.
/// Scope: local, triggers epidemic mode
pub struct PartitionDetected {
    pub isolated_zones: Vec<ZoneId>,
    pub last_known_gateway: Option<NodeId>,
    pub timestamp: Timestamp,
}

/// Emitted when partition heals.
/// Scope: local, restores normal routing
pub struct PartitionHealed {
    pub reconnected_via: NodeId,
    pub partition_duration_secs: u64,
    pub timestamp: Timestamp,
}
```

### Governance Events

```rust
/// Emitted when a zone submits a new proposal.
/// Scope: mesh-wide broadcast
pub struct ProposalSubmitted {
    pub proposal_id: ProposalHash,
    pub proposer_zone: ZoneId,
    pub title: String,
    pub action: GovernanceAction,
    pub voting_deadline: Timestamp,
    pub quorum: Quorum,
    pub timestamp: Timestamp,
}

/// Emitted when a zone casts a vote.
/// Scope: mesh-wide broadcast
pub struct VoteCast {
    pub proposal_id: ProposalHash,
    pub voter_zone: ZoneId,
    pub vote: Vote,
    pub signature: DilithiumSignature,
    pub timestamp: Timestamp,
}

/// Emitted when vote tallying completes.
/// Scope: mesh-wide broadcast
pub struct ProposalResolved {
    pub proposal_id: ProposalHash,
    pub result: TallyResult,
    pub yes_count: u32,
    pub no_count: u32,
    pub abstain_count: u32,
    pub quorum_met: bool,
    pub timestamp: Timestamp,
}

/// Emitted when credits are allocated or consumed.
/// Scope: DAG ledger entry
pub struct CreditTransferred {
    pub from_zone: ZoneId,
    pub to_zone: ZoneId,
    pub amount: u64,
    pub reason: CreditReason,
    pub timestamp: Timestamp,
}
```

### Robotics Events

```rust
/// Emitted when a robot accepts a mission.
/// Scope: swarm coordination (zone-level)
pub struct MissionAccepted {
    pub mission_id: MissionId,
    pub robot_id: RobotId,
    pub mission_type: MissionType,
    pub target_position: Position,
    pub estimated_duration_secs: u32,
    pub timestamp: Timestamp,
}

/// Emitted when a robot completes or aborts a mission.
/// Scope: swarm coordination + gateway
pub struct MissionCompleted {
    pub mission_id: MissionId,
    pub robot_id: RobotId,
    pub result: MissionResult,
    pub actual_duration_secs: u32,
    pub timestamp: Timestamp,
}

pub enum MissionResult {
    Success,
    PartialSuccess { details: String },
    Aborted { reason: AbortReason },
    Failed { error: String },
}

pub enum AbortReason {
    NoFlyZone,
    WeatherAbort,
    LowBattery,
    NavigationFailure,
    SafetyOverride,
}

/// Emitted when a robot enters/exits safety abort.
/// Scope: immediate swarm broadcast
pub struct SafetyEvent {
    pub robot_id: RobotId,
    pub reason: AbortReason,
    pub position: Position,
    pub returning_to_base: bool,
    pub timestamp: Timestamp,
}
```

### Flood-Specific Event Chain

```rust
/// Specialized event chain for flood propagation prediction.
/// Emitted preemptively for downstream zones.
pub struct FloodPreemptiveAlert {
    pub origin_event: EventId,       // links to the upstream EventConfirmed
    pub target_zone: ZoneId,
    pub estimated_arrival_secs: u32,
    pub severity: FloodSeverity,
    pub soil_saturation: f32,        // of target zone at time of alert
    pub recommended_action: FloodAction,
    pub timestamp: Timestamp,
}

pub enum FloodSeverity { Watch, Warning, Emergency }
pub enum FloodAction { Monitor, PrepareEvacuation, EvacuateNow }
```

### Node Lifecycle Events

```rust
/// Emitted when a node completes first boot after provisioning.
/// Scope: zone-local
pub struct NodeProvisioned {
    pub node_id: NodeId,
    pub zone_id: ZoneId,
    pub firmware_version: SemVer,
    pub hardware_revision: HardwareRev,
    pub timestamp: Timestamp,
}

/// Emitted when OTA firmware update is applied.
/// Scope: zone-local
pub struct FirmwareUpdated {
    pub node_id: NodeId,
    pub old_version: SemVer,
    pub new_version: SemVer,
    pub update_hash: FirmwareHash,
    pub timestamp: Timestamp,
}

/// Emitted when power management enters emergency mode.
/// Scope: zone-local
pub struct PowerEmergency {
    pub node_id: NodeId,
    pub battery_level: BatteryLevel,
    pub solar_output_mw: u16,
    pub entering_deep_sleep: bool,
    pub estimated_recovery_hours: Option<u16>,
    pub timestamp: Timestamp,
}
```

---

## Event Flow Diagrams

### Anomaly Detection → Confirmed Event → Response

```
Node A                Node B              Node C              SAFLA           Gateway
  │                     │                    │                  │                │
  │ AnomalyDetected     │                    │                  │                │
  ├────────────────────►│                    │                  │                │
  │ (pest, score=0.91)  │                    │                  │                │
  │                     │ AnomalyDetected    │                  │                │
  │                     ├───────────────────►│                  │                │
  │                     │ (pest, score=0.88) │                  │                │
  │                     │                    │ AnomalyDetected  │                │
  │                     │                    ├─────────────────►│                │
  │                     │                    │ (pest, score=0.90)                │
  │                     │                    │                  │                │
  │                     │                    │  [consensus:     │                │
  │                     │                    │   3 nodes,       │                │
  │                     │                    │   same zone,     │                │
  │                     │                    │   within 300s]   │                │
  │                     │                    │                  │                │
  │                     │                    │   EventConfirmed │                │
  │◄─────────────────────────────────────────┤ (pest, conf=0.95)               │
  │                     │◄───────────────────┤                  ├───────────────►│
  │                     │                    │◄─────────────────┤  (WebSocket)   │
  │                     │                    │                  │                │
```

### Flood Propagation Chain

```
Upstream Zone A         Zone B              Zone C           Zone D (Town)
      │                   │                    │                   │
      │ EventConfirmed    │                    │                   │
      │ (flood/emergency) │                    │                   │
      ├──────────────────►│                    │                   │
      │                   │                    │                   │
      │      FloodPreemptiveAlert             │                   │
      │      (arrival: 45min)                  │                   │
      │                   ├───────────────────►│                   │
      │                   │                    │                   │
      │                   │     FloodPreemptiveAlert              │
      │                   │     (arrival: 2hr)                    │
      │                   │                    ├──────────────────►│
      │                   │                    │                   │
      │                   │                    │  [town alert      │
      │                   │                    │   system activates]│
```

---

## London School TDD for Event Handlers

```rust
// Every event handler is tested by mocking its event source and event sink.
// This is the canonical London School pattern for event-driven systems.

pub trait EventHandler<E> {
    type Output;
    fn handle(&mut self, event: &E) -> Result<Self::Output, HandleError>;
}

// Example: the handler that converts EventConfirmed into FloodPreemptiveAlerts

pub trait WatershedGraph {
    fn downstream_neighbors(&self, zone: ZoneId) -> Vec<(ZoneId, FlowTime)>;
    fn current_soil_saturation(&self, zone: ZoneId) -> f32;
}

pub trait AlertEmitter {
    fn emit_preemptive_alert(&mut self, alert: &FloodPreemptiveAlert) -> Result<(), EmitError>;
}

pub struct FloodPropagationHandler<W: WatershedGraph, A: AlertEmitter> {
    watershed: W,
    emitter: A,
}

impl<W: WatershedGraph, A: AlertEmitter> EventHandler<EventConfirmed> for FloodPropagationHandler<W, A> {
    type Output = u32; // number of zones alerted

    fn handle(&mut self, event: &EventConfirmed) -> Result<u32, HandleError> {
        if !matches!(event.category, EventCategory::Flood { .. }) {
            return Ok(0);
        }
        // ... propagation logic from SPARC pseudocode
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn flood_handler_alerts_all_downstream_zones() {
        let mut mock_watershed = MockWatershedGraph::new();
        let mut mock_emitter = MockAlertEmitter::new();

        // GIVEN: zone A has two downstream neighbors
        mock_watershed.expect_downstream_neighbors()
            .with(eq(zone_a))
            .returning(|_| vec![
                (zone_b, FlowTime::minutes(45)),
                (zone_c, FlowTime::minutes(120)),
            ]);

        // AND: moderate soil saturation (increases urgency)
        mock_watershed.expect_current_soil_saturation()
            .returning(|_| 0.75);

        // Additional downstream from zone B
        mock_watershed.expect_downstream_neighbors()
            .with(eq(zone_b))
            .returning(|_| vec![(zone_d, FlowTime::minutes(90))]);

        mock_watershed.expect_downstream_neighbors()
            .with(eq(zone_c))
            .returning(|_| vec![]); // end of watershed

        mock_watershed.expect_downstream_neighbors()
            .with(eq(zone_d))
            .returning(|_| vec![]); // end of watershed

        // THEN: 3 preemptive alerts are emitted (B, C, D)
        mock_emitter.expect_emit_preemptive_alert()
            .times(3)
            .returning(|_| Ok(()));

        let mut handler = FloodPropagationHandler::new(mock_watershed, mock_emitter);
        let confirmed = EventConfirmed {
            category: EventCategory::Flood {
                severity: FloodSeverity::Emergency,
                upstream_origin: None,
            },
            affected_zone: zone_a,
            confidence: 0.95,
            ..test_confirmed()
        };

        let count = handler.handle(&confirmed).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn flood_handler_ignores_non_flood_events() {
        let mut mock_emitter = MockAlertEmitter::new();

        // THEN: no alerts emitted for pest events
        mock_emitter.expect_emit_preemptive_alert().times(0);

        let mut handler = FloodPropagationHandler::new(mock_watershed, mock_emitter);
        let pest_event = EventConfirmed {
            category: EventCategory::Pest { species_hint: None },
            ..test_confirmed()
        };

        let count = handler.handle(&pest_event).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn flood_handler_adjusts_arrival_time_for_saturated_soil() {
        let mut mock_watershed = MockWatershedGraph::new();
        let mut mock_emitter = MockAlertEmitter::new();

        mock_watershed.expect_downstream_neighbors()
            .with(eq(zone_a))
            .returning(|_| vec![(zone_b, FlowTime::minutes(60))]);

        mock_watershed.expect_downstream_neighbors()
            .with(eq(zone_b))
            .returning(|_| vec![]);

        // Soil is 90% saturated — water moves faster
        mock_watershed.expect_current_soil_saturation()
            .with(eq(zone_b))
            .returning(|_| 0.90);

        // THEN: arrival time is reduced (saturation factor < 1.0)
        mock_emitter.expect_emit_preemptive_alert()
            .withf(|alert| alert.estimated_arrival_secs < 3600) // less than 60 min
            .times(1)
            .returning(|_| Ok(()));

        let mut handler = FloodPropagationHandler::new(mock_watershed, mock_emitter);
        handler.handle(&flood_confirmed_at(zone_a)).unwrap();
    }
}
```

---

## Event Serialization

All domain events are serialized with `postcard` (binary, `no_std` compatible) for mesh transport and with `serde_json` for the gateway API.

```rust
// Events implement both serialization paths
#[derive(Serialize, Deserialize)]
pub struct EventConfirmed { /* ... */ }

// Mesh serialization (compact binary)
impl MeshSerialize for EventConfirmed {
    fn to_mesh_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        postcard::to_vec(self).map_err(Into::into)
    }
}

// API serialization (JSON for dashboard)
// Automatically derived from serde::Serialize
```
