use verdant_core::config::{CONSENSUS_QUORUM, TEMPORAL_WINDOW_SECS};
use verdant_core::error::EmitError;
use verdant_core::types::{
    ConfirmedEvent, EventCategory, NodeId, Timestamp, ZoneId,
};
use verdant_mesh::routing::RoutingUpdate;
use verdant_safla::consensus::ConsensusEngine;
use verdant_safla::events::{AlertEmitter, FloodPreemptiveAlert, FloodPropagationHandler, WatershedGraph};
use verdant_safla::health::HealthAssessor;

use crate::node::{training_embedding, SimClock, SimNode};
use crate::scenario::{Alert, SimConfig, ZoneLayout};
use crate::terrain::{Position, RfModel};

/// The simulation engine — manages nodes, time, and message passing.
pub struct Simulation {
    pub nodes: Vec<SimNode>,
    pub config: SimConfig,
    pub rf_model: RfModel,
    pub clock: SimClock,
    pub tick: u64,
    pub consensus_engine: ConsensusEngine,
    pub health_assessor: HealthAssessor,
}

impl Simulation {
    pub fn new(config: SimConfig) -> Self {
        let rf_model = RfModel {
            max_range_m: config.rf_max_range_m,
            pdr_at_zero: 0.98,
        };
        Self {
            nodes: Vec::new(),
            rf_model,
            clock: SimClock::new(10), // mid-spring
            tick: 0,
            consensus_engine: ConsensusEngine::new(
                CONSENSUS_QUORUM,
                TEMPORAL_WINDOW_SECS,
                500.0,
            ),
            health_assessor: HealthAssessor::new(60, 3),
            config,
        }
    }

    /// Deploy nodes according to the configured layout.
    pub fn deploy_nodes(&mut self) {
        let ZoneLayout::Grid { rows, cols } = &self.config.zone_layout;
        let spacing = self.config.rf_max_range_m * 0.6; // nodes within range of neighbors

        let mut node_idx = 0u8;
        for r in 0..*rows {
            for c in 0..*cols {
                if (r * cols + c) >= self.config.node_count {
                    break;
                }
                let zone_idx = (r * cols + c) % self.config.zone_count;
                let id = NodeId([node_idx, 0, 0, 0, 0, 0, 0, 0]);
                let zone = ZoneId([zone_idx as u8, 0, 0, 0]);
                let position = Position::new(c as f64 * spacing, r as f64 * spacing);

                self.nodes.push(SimNode::new(id, zone, position));
                node_idx += 1;
            }
        }

        // Establish initial routing: each node discovers neighbors within range
        self.update_all_link_qualities();
    }

    /// Update link quality between all pairs of alive nodes.
    fn update_all_link_qualities(&mut self) {
        let positions: Vec<(usize, NodeId, Position, bool)> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (i, n.id, n.position, n.alive))
            .collect();

        for i in 0..positions.len() {
            if !positions[i].3 {
                continue; // dead
            }
            for j in (i + 1)..positions.len() {
                if !positions[j].3 {
                    continue;
                }
                let pdr = self.rf_model.compute_pdr(&positions[i].2, &positions[j].2);
                if pdr > 0.0 {
                    let rtt = self.rf_model.compute_rtt_ms(&positions[i].2, &positions[j].2);
                    self.nodes[i]
                        .routing_table
                        .update_link_quality(positions[j].1, pdr, rtt);
                    self.nodes[j]
                        .routing_table
                        .update_link_quality(positions[i].1, pdr, rtt);
                }
            }
        }

        // Exchange routing updates between neighbors
        self.exchange_routing_updates();
    }

    /// One round of routing update exchange between all neighbor pairs.
    fn exchange_routing_updates(&mut self) {
        // Collect all routes to share
        let mut updates: Vec<(NodeId, RoutingUpdate)> = Vec::new();
        for node in &self.nodes {
            if !node.alive {
                continue;
            }
            for neighbor in node.routing_table.active_neighbors() {
                // Share all routes with this neighbor (split-horizon applied by the table)
                for route in node.routing_table.routes_for_advertisement(neighbor.id) {
                    updates.push((
                        neighbor.id,
                        RoutingUpdate {
                            from: node.id,
                            destination: route.destination,
                            cost: route.cost,
                            hops: route.hops,
                        },
                    ));
                }
            }
        }

        for (target, update) in updates {
            if let Some(node) = self.nodes.iter_mut().find(|n| n.id == target && n.alive) {
                let _ = node.routing_table.apply_update(update);
            }
        }
    }

    /// Run routing convergence for multiple rounds.
    pub fn run_until_mesh_converges(&mut self, max_rounds: usize) {
        for _ in 0..max_rounds {
            self.update_all_link_qualities();
        }
    }

    /// Train all nodes with baseline data for the given number of ticks.
    pub fn fast_forward_training(&mut self, ticks: u64) {
        for t in 0..ticks {
            for node in &mut self.nodes {
                if !node.alive {
                    continue;
                }
                let emb = training_embedding(&node.zone, t);
                node.graph.update(emb, &self.clock);
            }
        }
    }

    /// Advance the simulation by one tick.
    pub fn tick(&mut self) {
        self.tick += 1;

        // Propagate neighbor anomalies between nearby nodes
        self.propagate_anomalies();

        // Run consensus on each node
        let engine = ConsensusEngine::new(
            CONSENSUS_QUORUM,
            TEMPORAL_WINDOW_SECS,
            500.0,
        );
        for node in &mut self.nodes {
            if !node.alive {
                continue;
            }
            node.run_consensus(&engine);
        }
    }

    /// Run for N ticks.
    pub fn run_for(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.tick();
        }
    }

    /// Share anomalies between nearby nodes.
    fn propagate_anomalies(&mut self) {
        // Collect all anomalies with positions
        let anomaly_reports: Vec<(Position, Vec<verdant_core::types::Anomaly>)> = self
            .nodes
            .iter()
            .filter(|n| n.alive)
            .map(|n| (n.position, n.local_anomalies.clone()))
            .collect();

        // Distribute to neighbors within range
        for node in &mut self.nodes {
            if !node.alive {
                continue;
            }
            node.neighbor_anomalies.clear();
            for (pos, anomalies) in &anomaly_reports {
                let dist = node.position.distance_to(pos) as f32;
                if dist < 1.0 {
                    continue; // skip self
                }
                if dist > self.rf_model.max_range_m as f32 {
                    continue;
                }
                for anomaly in anomalies {
                    node.neighbor_anomalies.push(verdant_core::types::NeighborAnomaly {
                        source_zone: anomaly.zone,
                        category: anomaly.category.clone(),
                        score: anomaly.score,
                        timestamp: anomaly.timestamp,
                        distance_m: dist,
                    });
                }
            }
        }
    }

    /// Inject a flood event at all nodes in and near the specified zone.
    ///
    /// A flood is a spatial event that affects nearby nodes regardless of
    /// zone boundaries. All nodes within RF range of any zone member are
    /// injected, ensuring enough corroboration for consensus.
    pub fn inject_flood(&mut self, zone: ZoneId, now: Timestamp) {
        // Find positions of all zone-member nodes
        let zone_positions: Vec<Position> = self
            .nodes
            .iter()
            .filter(|n| n.alive && n.zone == zone)
            .map(|n| n.position)
            .collect();

        for node in &mut self.nodes {
            if !node.alive {
                continue;
            }
            // Inject if node is in the zone OR within RF range of a zone member
            let in_range = node.zone == zone
                || zone_positions
                    .iter()
                    .any(|p| node.position.distance_to(p) < self.rf_model.max_range_m);
            if in_range {
                node.local_anomalies.push(verdant_core::types::Anomaly {
                    category: EventCategory::Flood {
                        severity: verdant_core::types::FloodSeverity::Emergency,
                        upstream_origin: None,
                    },
                    score: 0.95,
                    zone: node.zone,
                    timestamp: now,
                });
            }
        }
    }

    /// Inject a pest anomaly at all nodes in and near the specified zone.
    pub fn inject_pest(&mut self, zone: ZoneId, now: Timestamp) {
        let zone_positions: Vec<Position> = self
            .nodes
            .iter()
            .filter(|n| n.alive && n.zone == zone)
            .map(|n| n.position)
            .collect();

        for node in &mut self.nodes {
            if !node.alive {
                continue;
            }
            let in_range = node.zone == zone
                || zone_positions
                    .iter()
                    .any(|p| node.position.distance_to(p) < self.rf_model.max_range_m);
            if in_range {
                node.local_anomalies.push(verdant_core::types::Anomaly {
                    category: EventCategory::Pest {
                        species_hint: None,
                    },
                    score: 0.92,
                    zone: node.zone,
                    timestamp: now,
                });
            }
        }
    }

    /// Kill a specific node.
    pub fn kill_node(&mut self, id: NodeId) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) {
            node.alive = false;
        }
    }

    /// Check if a node can reach any gateway.
    pub fn can_reach_gateway(&self, id: NodeId) -> bool {
        self.nodes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.routing_table.has_gateway_route())
            .unwrap_or(false)
    }

    /// Get all confirmed events for a zone.
    pub fn confirmed_events_in_zone(&self, zone: ZoneId) -> Vec<&ConfirmedEvent> {
        self.nodes
            .iter()
            .filter(|n| n.zone == zone && n.alive)
            .flat_map(|n| &n.confirmed_events)
            .collect()
    }

    /// Check if any node in a zone has a confirmed event of the given category.
    pub fn has_confirmed_event(&self, zone: ZoneId, category_match: fn(&EventCategory) -> bool) -> bool {
        self.nodes
            .iter()
            .filter(|n| n.zone == zone && n.alive)
            .any(|n| n.confirmed_events.iter().any(|e| category_match(&e.category)))
    }

    /// Find a node that has active neighbors (potential relay).
    pub fn find_relay_node(&self) -> Option<NodeId> {
        self.nodes
            .iter()
            .filter(|n| n.alive)
            .find(|n| n.routing_table.active_neighbors().count() >= 2)
            .map(|n| n.id)
    }

    /// Run flood propagation for confirmed flood events using a simple watershed.
    pub fn propagate_floods(&mut self, watershed: &impl WatershedGraph) {
        let flood_events: Vec<ConfirmedEvent> = self
            .nodes
            .iter()
            .filter(|n| n.alive)
            .flat_map(|n| n.confirmed_events.clone())
            .filter(|e| matches!(e.category, EventCategory::Flood { .. }))
            .collect();

        let mut emitter = SimAlertEmitter {
            alerts: Vec::new(),
        };
        for event in &flood_events {
            let _ = FloodPropagationHandler::propagate(event, watershed, &mut emitter);
        }

        // Deliver alerts to target zones
        for alert in &emitter.alerts {
            for node in &mut self.nodes {
                if node.zone == alert.target_zone && node.alive {
                    node.alerts_received.push(Alert::FloodPreemptive(alert.clone()));
                }
            }
        }
    }

    /// Get all alerts received by nodes in a zone.
    pub fn alerts_received_by(&self, zone: ZoneId) -> Vec<&Alert> {
        self.nodes
            .iter()
            .filter(|n| n.zone == zone && n.alive)
            .flat_map(|n| &n.alerts_received)
            .collect()
    }
}

struct SimAlertEmitter {
    alerts: Vec<FloodPreemptiveAlert>,
}

impl AlertEmitter for SimAlertEmitter {
    fn emit_preemptive_alert(&mut self, alert: &FloodPreemptiveAlert) -> Result<(), EmitError> {
        self.alerts.push(alert.clone());
        Ok(())
    }
}

/// Simple watershed graph for testing: zone(0) → zone(1) → zone(2) → zone(3).
pub struct LinearWatershed {
    pub zone_count: usize,
    pub flow_time_secs: u32,
    pub base_saturation: f32,
}

impl WatershedGraph for LinearWatershed {
    fn downstream_neighbors(&self, zone: ZoneId) -> heapless::Vec<(ZoneId, u32), 8> {
        let idx = zone.0[0] as usize;
        let mut v = heapless::Vec::new();
        if idx + 1 < self.zone_count {
            let next = ZoneId([idx as u8 + 1, 0, 0, 0]);
            let _ = v.push((next, self.flow_time_secs));
        }
        v
    }

    fn current_soil_saturation(&self, _zone: ZoneId) -> f32 {
        self.base_saturation
    }
}
