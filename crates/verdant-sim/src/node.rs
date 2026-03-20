use std::collections::VecDeque;

use verdant_core::config::EMBEDDING_DIM;
use verdant_core::error::{EmitError, HealError};
use verdant_core::traits::{AnomalySource, ConfirmedEventSink, NetworkHealer, SeasonalClock};
use verdant_core::types::{
    Anomaly, ConfirmedEvent, Embedding, EventCategory, MeshFrame, NeighborAnomaly, NodeId,
    SeasonSlot, Timestamp, TopologyChange, ZoneId,
};
use verdant_mesh::routing::RoutingTable;
use verdant_safla::consensus::ConsensusEngine;
use verdant_safla::health::HealthAssessor;
use verdant_vector::graph::VectorGraph;

use crate::terrain::Position;

/// A simulated mesh node with full internal state.
pub struct SimNode {
    pub id: NodeId,
    pub zone: ZoneId,
    pub position: Position,
    pub alive: bool,

    pub graph: VectorGraph,
    pub routing_table: RoutingTable,

    // Anomaly / event state
    pub local_anomalies: Vec<Anomaly>,
    pub neighbor_anomalies: Vec<NeighborAnomaly>,
    pub confirmed_events: Vec<ConfirmedEvent>,
    pub alerts_received: Vec<crate::scenario::Alert>,

    // Message queues
    pub outbox: VecDeque<(NodeId, MeshFrame)>, // (target_neighbor, frame)
    pub inbox: VecDeque<MeshFrame>,
}

impl SimNode {
    pub fn new(id: NodeId, zone: ZoneId, position: Position) -> Self {
        Self {
            id,
            zone,
            position,
            alive: true,
            graph: VectorGraph::new(256), // smaller for sim performance
            routing_table: RoutingTable::new(id),
            local_anomalies: Vec::new(),
            neighbor_anomalies: Vec::new(),
            confirmed_events: Vec::new(),
            alerts_received: Vec::new(),
            outbox: VecDeque::new(),
            inbox: VecDeque::new(),
        }
    }

    /// Feed an embedding observation (simulating a sense cycle).
    pub fn observe(&mut self, embedding: Embedding, clock: &impl SeasonalClock) {
        let score = self.graph.anomaly_score(&embedding, clock);
        self.graph.update(embedding.clone(), clock);

        if score > verdant_core::config::ANOMALY_THRESHOLD {
            self.local_anomalies.push(Anomaly {
                category: EventCategory::Pest {
                    species_hint: None,
                },
                score,
                zone: self.zone,
                timestamp: Timestamp(0), // set by caller
            });
        }
    }

    /// Feed a flood-specific observation.
    pub fn observe_flood(
        &mut self,
        embedding: Embedding,
        clock: &impl SeasonalClock,
        now: Timestamp,
    ) {
        let score = self.graph.anomaly_score(&embedding, clock);
        self.graph.update(embedding.clone(), clock);

        if score > verdant_core::config::ANOMALY_THRESHOLD {
            self.local_anomalies.push(Anomaly {
                category: EventCategory::Flood {
                    severity: verdant_core::types::FloodSeverity::Emergency,
                    upstream_origin: None,
                },
                score,
                zone: self.zone,
                timestamp: now,
            });
        }
    }

    /// Run consensus on accumulated anomalies.
    pub fn run_consensus(&mut self, engine: &ConsensusEngine) {
        let source = SimAnomalySource {
            local: &self.local_anomalies,
            neighbor: &self.neighbor_anomalies,
        };
        let mut sink = SimEventSink {
            events: &mut self.confirmed_events,
        };
        let _ = engine.evaluate(&source, &mut sink);
    }

    /// Run health assessment on routing table neighbors.
    pub fn run_health(&mut self, assessor: &HealthAssessor, now: Timestamp) {
        let neighbors: Vec<_> = self
            .routing_table
            .active_neighbors()
            .cloned()
            .collect();
        let mut healer = SimHealer {
            routing_table: &mut self.routing_table,
        };
        let _report = assessor.assess(&neighbors, now, &mut healer);
    }
}

// --- Trait implementations for simulation ---

struct SimAnomalySource<'a> {
    local: &'a [Anomaly],
    neighbor: &'a [NeighborAnomaly],
}

impl AnomalySource for SimAnomalySource<'_> {
    fn local_anomalies(&self) -> heapless::Vec<Anomaly, 16> {
        let mut v = heapless::Vec::new();
        for a in self.local.iter().take(16) {
            let _ = v.push(a.clone());
        }
        v
    }

    fn neighbor_anomalies(&self, _window_secs: u64) -> heapless::Vec<NeighborAnomaly, 64> {
        let mut v = heapless::Vec::new();
        for a in self.neighbor.iter().take(64) {
            let _ = v.push(a.clone());
        }
        v
    }
}

struct SimEventSink<'a> {
    events: &'a mut Vec<ConfirmedEvent>,
}

impl ConfirmedEventSink for SimEventSink<'_> {
    fn emit_confirmed(&mut self, event: &ConfirmedEvent) -> Result<(), EmitError> {
        self.events.push(event.clone());
        Ok(())
    }

    fn request_robot_task(&mut self, _event: &ConfirmedEvent) -> Result<(), EmitError> {
        Ok(()) // no robots in simulation
    }
}

struct SimHealer<'a> {
    routing_table: &'a mut RoutingTable,
}

impl NetworkHealer for SimHealer<'_> {
    fn reroute_around(&mut self, dead: NodeId) -> Result<(), HealError> {
        self.routing_table.mark_dead(dead);
        Ok(())
    }

    fn propose_topology_change(&mut self, _change: &TopologyChange) -> Result<(), HealError> {
        Ok(())
    }
}

/// A seasonal clock for the simulation that returns a fixed slot.
pub struct SimClock {
    pub slot: SeasonSlot,
}

impl SimClock {
    pub fn new(week: u8) -> Self {
        Self {
            slot: SeasonSlot::new(week),
        }
    }
}

impl SeasonalClock for SimClock {
    fn current_slot(&self) -> SeasonSlot {
        self.slot
    }
}

/// Generate a deterministic embedding for training from a seed + zone.
///
/// Training embeddings are concentrated in the first half of dimensions
/// with positive values, creating a stable baseline direction.
pub fn training_embedding(zone: &ZoneId, tick: u64) -> Embedding {
    let mut data = [0i16; EMBEDDING_DIM];
    let variation = ((tick % 20) as i16) - 10;
    // First half: strong positive signal based on zone
    for (i, d) in data[..EMBEDDING_DIM / 2].iter_mut().enumerate() {
        *d = (zone.0[i % 4] as i16 + 1) * 200 + (i as i16) * 50 + variation;
    }
    // Second half: near zero
    for d in &mut data[EMBEDDING_DIM / 2..] {
        *d = variation;
    }
    Embedding { data }
}

/// Generate an anomalous embedding that is nearly orthogonal to training data.
///
/// Anomalous embeddings are concentrated in the second half of dimensions
/// with strong values, while the first half is near zero. This creates
/// a large cosine distance from the training baseline.
pub fn anomalous_embedding(zone: &ZoneId) -> Embedding {
    let mut data = [0i16; EMBEDDING_DIM];
    // First half: near zero (opposite of training)
    for d in &mut data[..EMBEDDING_DIM / 2] {
        *d = 5;
    }
    // Second half: strong positive signal
    for (i, d) in data[EMBEDDING_DIM / 2..].iter_mut().enumerate() {
        let idx = i + EMBEDDING_DIM / 2;
        *d = (zone.0[idx % 4] as i16 + 1) * 200 + (idx as i16) * 50;
    }
    Embedding { data }
}
