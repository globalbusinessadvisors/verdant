use serde::{Deserialize, Serialize};
use verdant_core::error::EmitError;
use verdant_core::types::{ConfirmedEvent, EventCategory, FloodSeverity, Timestamp, ZoneId};

/// Preemptive flood alert sent to downstream zones.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FloodPreemptiveAlert {
    pub origin_event_id: u64,
    pub target_zone: ZoneId,
    pub estimated_arrival_secs: u32,
    pub severity: FloodSeverity,
    pub soil_saturation: f32,
    pub timestamp: Timestamp,
}

/// Provides watershed topology for flood propagation.
pub trait WatershedGraph {
    /// Return downstream zones with estimated flow time in seconds.
    fn downstream_neighbors(&self, zone: ZoneId) -> heapless::Vec<(ZoneId, u32), 8>;
    /// Current soil saturation [0.0–1.0] for a zone.
    fn current_soil_saturation(&self, zone: ZoneId) -> f32;
}

/// Emits preemptive flood alerts to downstream zones.
pub trait AlertEmitter {
    fn emit_preemptive_alert(&mut self, alert: &FloodPreemptiveAlert) -> Result<(), EmitError>;
}

/// Propagation errors from the flood handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FloodPropagationError {
    EmitFailed(EmitError),
}

impl From<EmitError> for FloodPropagationError {
    fn from(e: EmitError) -> Self {
        Self::EmitFailed(e)
    }
}

/// Propagates flood alerts downstream through a watershed graph.
///
/// Implements a BFS/Dijkstra-style traversal: starting from the confirmed
/// flood event's zone, visit each downstream zone, adjusting estimated
/// arrival time for soil saturation, and emit preemptive alerts.
pub struct FloodPropagationHandler;

impl FloodPropagationHandler {
    /// Propagate flood alerts for a confirmed event.
    ///
    /// Returns the number of zones alerted. Non-flood events return 0.
    pub fn propagate(
        event: &ConfirmedEvent,
        watershed: &impl WatershedGraph,
        emitter: &mut impl AlertEmitter,
    ) -> Result<u32, FloodPropagationError> {
        let severity = match &event.category {
            EventCategory::Flood { severity, .. } => *severity,
            _ => return Ok(0), // not a flood event
        };

        let mut alerted = 0u32;
        // BFS queue: (zone, accumulated_arrival_secs)
        let mut queue: heapless::Vec<(ZoneId, u32), 64> = heapless::Vec::new();
        let _ = queue.push((event.affected_zone, 0));

        // Visited set (simple linear scan — watershed graphs are small)
        let mut visited: heapless::Vec<ZoneId, 64> = heapless::Vec::new();
        let _ = visited.push(event.affected_zone);

        let mut head = 0usize;
        while head < queue.len() {
            let (current_zone, arrival_secs) = queue[head];
            head += 1;

            let downstream = watershed.downstream_neighbors(current_zone);
            for (next_zone, flow_time_secs) in &downstream {
                if visited.contains(next_zone) {
                    continue;
                }
                let _ = visited.push(*next_zone);

                // Adjust for soil saturation: saturated soil → faster propagation
                let saturation = watershed.current_soil_saturation(*next_zone);
                let saturation_factor = 1.0 - (saturation * 0.5); // 1.0 @ dry, 0.5 @ saturated
                let adjusted_time = (*flow_time_secs as f32 * saturation_factor) as u32;
                let total_arrival = arrival_secs + adjusted_time;

                let alert = FloodPreemptiveAlert {
                    origin_event_id: event.event_id,
                    target_zone: *next_zone,
                    estimated_arrival_secs: total_arrival,
                    severity,
                    soil_saturation: saturation,
                    timestamp: event.timestamp,
                };

                emitter.emit_preemptive_alert(&alert)?;
                alerted += 1;

                let _ = queue.push((*next_zone, total_arrival));
            }
        }

        Ok(alerted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use verdant_core::types::{PestSpecies, RecommendedAction};

    mock! {
        pub Watershed {}
        impl WatershedGraph for Watershed {
            fn downstream_neighbors(&self, zone: ZoneId) -> heapless::Vec<(ZoneId, u32), 8>;
            fn current_soil_saturation(&self, zone: ZoneId) -> f32;
        }
    }

    mock! {
        pub Emitter {}
        impl AlertEmitter for Emitter {
            fn emit_preemptive_alert(&mut self, alert: &FloodPreemptiveAlert) -> Result<(), EmitError>;
        }
    }

    fn zone(id: u8) -> ZoneId {
        ZoneId([id, 0, 0, 0])
    }

    fn flood_event(z: ZoneId) -> ConfirmedEvent {
        ConfirmedEvent {
            event_id: 42,
            category: EventCategory::Flood {
                severity: FloodSeverity::Emergency,
                upstream_origin: None,
            },
            confidence: 0.95,
            affected_zone: z,
            corroborating_count: 4,
            recommended_action: RecommendedAction::PreemptiveAlert,
            timestamp: Timestamp::from_secs(1000),
        }
    }

    fn pest_event(z: ZoneId) -> ConfirmedEvent {
        ConfirmedEvent {
            event_id: 99,
            category: EventCategory::Pest {
                species_hint: Some(PestSpecies::EmeraldAshBorer),
            },
            confidence: 0.90,
            affected_zone: z,
            corroborating_count: 3,
            recommended_action: RecommendedAction::AlertAndMonitor,
            timestamp: Timestamp::from_secs(1000),
        }
    }

    #[test]
    fn flood_alerts_all_downstream_zones() {
        let mut watershed = MockWatershed::new();
        let mut emitter = MockEmitter::new();

        // zone(1) → zone(2) and zone(3)
        watershed
            .expect_downstream_neighbors()
            .withf(|z| *z == zone(1))
            .returning(|_| {
                let mut v = heapless::Vec::new();
                let _ = v.push((zone(2), 2700));  // 45 min
                let _ = v.push((zone(3), 7200));  // 120 min
                v
            });

        // zone(2) → zone(4)
        watershed
            .expect_downstream_neighbors()
            .withf(|z| *z == zone(2))
            .returning(|_| {
                let mut v = heapless::Vec::new();
                let _ = v.push((zone(4), 5400)); // 90 min
                v
            });

        // zone(3), zone(4) → no further downstream
        watershed
            .expect_downstream_neighbors()
            .withf(|z| *z == zone(3) || *z == zone(4))
            .returning(|_| heapless::Vec::new());

        // All zones have moderate saturation
        watershed
            .expect_current_soil_saturation()
            .returning(|_| 0.5);

        // Expect 3 alerts: zone(2), zone(3), zone(4)
        emitter
            .expect_emit_preemptive_alert()
            .times(3)
            .returning(|_| Ok(()));

        let event = flood_event(zone(1));
        let count = FloodPropagationHandler::propagate(&event, &watershed, &mut emitter).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn ignores_non_flood_events() {
        let watershed = MockWatershed::new();
        let mut emitter = MockEmitter::new();

        emitter.expect_emit_preemptive_alert().times(0);

        let event = pest_event(zone(1));
        let count = FloodPropagationHandler::propagate(&event, &watershed, &mut emitter).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn adjusts_arrival_time_for_soil_saturation() {
        let mut watershed = MockWatershed::new();
        let mut emitter = MockEmitter::new();

        // zone(1) → zone(2) with 60 min flow time
        watershed
            .expect_downstream_neighbors()
            .withf(|z| *z == zone(1))
            .returning(|_| {
                let mut v = heapless::Vec::new();
                let _ = v.push((zone(2), 3600)); // 60 min
                v
            });

        watershed
            .expect_downstream_neighbors()
            .withf(|z| *z == zone(2))
            .returning(|_| heapless::Vec::new());

        // Zone 2 is 90% saturated → factor = 1.0 - 0.9*0.5 = 0.55
        // adjusted_time = 3600 * 0.55 = 1980
        watershed
            .expect_current_soil_saturation()
            .withf(|z| *z == zone(2))
            .returning(|_| 0.90);

        emitter
            .expect_emit_preemptive_alert()
            .withf(|alert| alert.estimated_arrival_secs < 3600) // less than nominal
            .times(1)
            .returning(|_| Ok(()));

        let event = flood_event(zone(1));
        FloodPropagationHandler::propagate(&event, &watershed, &mut emitter).unwrap();
    }

    #[test]
    fn no_duplicate_alerts_for_diamond_watershed() {
        let mut watershed = MockWatershed::new();
        let mut emitter = MockEmitter::new();

        // Diamond: zone(1) → zone(2), zone(3) → zone(4) (both converge)
        watershed
            .expect_downstream_neighbors()
            .withf(|z| *z == zone(1))
            .returning(|_| {
                let mut v = heapless::Vec::new();
                let _ = v.push((zone(2), 1000));
                let _ = v.push((zone(3), 1000));
                v
            });

        watershed
            .expect_downstream_neighbors()
            .withf(|z| *z == zone(2))
            .returning(|_| {
                let mut v = heapless::Vec::new();
                let _ = v.push((zone(4), 1000));
                v
            });

        watershed
            .expect_downstream_neighbors()
            .withf(|z| *z == zone(3))
            .returning(|_| {
                let mut v = heapless::Vec::new();
                let _ = v.push((zone(4), 1000)); // same zone(4)
                v
            });

        watershed
            .expect_downstream_neighbors()
            .withf(|z| *z == zone(4))
            .returning(|_| heapless::Vec::new());

        watershed
            .expect_current_soil_saturation()
            .returning(|_| 0.3);

        // zone(4) should only be alerted once despite two paths
        emitter
            .expect_emit_preemptive_alert()
            .times(3) // zone(2), zone(3), zone(4) — not 4
            .returning(|_| Ok(()));

        let event = flood_event(zone(1));
        let count = FloodPropagationHandler::propagate(&event, &watershed, &mut emitter).unwrap();
        assert_eq!(count, 3);
    }
}
