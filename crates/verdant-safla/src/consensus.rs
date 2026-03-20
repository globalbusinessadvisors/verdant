use verdant_core::error::EmitError;
use verdant_core::traits::{AnomalySource, ConfirmedEventSink};
use verdant_core::types::{Anomaly, ConfirmedEvent, EventCategory, NeighborAnomaly, RecommendedAction};

/// Consensus errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsensusError {
    EmitFailed(EmitError),
}

impl From<EmitError> for ConsensusError {
    fn from(e: EmitError) -> Self {
        Self::EmitFailed(e)
    }
}

/// Cross-node anomaly consensus engine.
///
/// Evaluates local anomalies against neighbor reports, applying spatial,
/// temporal, and category filters. When sufficient corroboration is found
/// (quorum met), emits a confirmed event via the sink.
pub struct ConsensusEngine {
    quorum: u8,
    temporal_window_ms: u64,
    spatial_radius_m: f32,
}

impl ConsensusEngine {
    pub fn new(quorum: u8, temporal_window_secs: u64, spatial_radius_m: f32) -> Self {
        Self {
            quorum,
            temporal_window_ms: temporal_window_secs * 1000,
            spatial_radius_m,
        }
    }

    /// Evaluate all local anomalies against neighbor reports and emit
    /// confirmed events for those meeting the quorum.
    pub fn evaluate(
        &self,
        source: &impl AnomalySource,
        sink: &mut impl ConfirmedEventSink,
    ) -> Result<heapless::Vec<ConfirmedEvent, 8>, ConsensusError> {
        let local = source.local_anomalies();
        let neighbors = source.neighbor_anomalies(self.temporal_window_ms / 1000);

        let mut confirmed = heapless::Vec::new();

        for anomaly in &local {
            let corroborating = self.count_corroborating(anomaly, &neighbors);

            if corroborating >= self.quorum {
                let confidence = Self::compute_confidence(anomaly, corroborating);
                let event = ConfirmedEvent {
                    event_id: Self::event_id_from_anomaly(anomaly),
                    category: anomaly.category.clone(),
                    confidence,
                    affected_zone: anomaly.zone,
                    corroborating_count: corroborating,
                    recommended_action: Self::recommend_action(&anomaly.category),
                    timestamp: anomaly.timestamp,
                };

                sink.emit_confirmed(&event)?;
                let _ = confirmed.push(event);
            }
        }

        Ok(confirmed)
    }

    /// Count neighbor anomalies that corroborate a local anomaly.
    ///
    /// A neighbor anomaly corroborates if:
    /// 1. Same event category
    /// 2. Within spatial radius
    /// 3. Within temporal window
    fn count_corroborating(
        &self,
        local: &Anomaly,
        neighbors: &[NeighborAnomaly],
    ) -> u8 {
        let mut count = 0u8;
        for n in neighbors {
            if !Self::categories_match(&local.category, &n.category) {
                continue;
            }
            if n.distance_m > self.spatial_radius_m {
                continue;
            }
            let time_diff = local.timestamp.saturating_diff(&n.timestamp)
                .max(n.timestamp.saturating_diff(&local.timestamp));
            if time_diff > self.temporal_window_ms {
                continue;
            }
            count = count.saturating_add(1);
        }
        count
    }

    /// Check if two event categories match at the top level.
    fn categories_match(a: &EventCategory, b: &EventCategory) -> bool {
        core::mem::discriminant(a) == core::mem::discriminant(b)
    }

    /// Compute confidence from the anomaly score and corroboration count.
    fn compute_confidence(anomaly: &Anomaly, corroborating: u8) -> f32 {
        let base = anomaly.score;
        let boost = (corroborating as f32) * 0.05;
        (base + boost).clamp(0.0, 1.0)
    }

    /// Derive a deterministic event ID from an anomaly.
    fn event_id_from_anomaly(anomaly: &Anomaly) -> u64 {
        let zone_bytes = anomaly.zone.0;
        let ts = anomaly.timestamp.as_millis();
        let mut id = ts;
        for &b in &zone_bytes {
            id = id.wrapping_mul(31).wrapping_add(b as u64);
        }
        id
    }

    /// Determine recommended action based on event category.
    fn recommend_action(category: &EventCategory) -> RecommendedAction {
        match category {
            EventCategory::Flood { .. } => RecommendedAction::PreemptiveAlert,
            EventCategory::Fire { .. } => RecommendedAction::EvacuationWarning,
            EventCategory::Pest { .. } => RecommendedAction::AlertAndMonitor,
            EventCategory::Infrastructure { .. } => RecommendedAction::AlertAndMonitor,
            EventCategory::Wildlife { .. } => RecommendedAction::AlertOnly,
            EventCategory::Climate { .. } => RecommendedAction::AlertOnly,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use verdant_core::types::{FloodSeverity, PestSpecies, Timestamp, ZoneId};

    mock! {
        pub Source {}
        impl AnomalySource for Source {
            fn local_anomalies(&self) -> heapless::Vec<Anomaly, 16>;
            fn neighbor_anomalies(&self, window_secs: u64) -> heapless::Vec<NeighborAnomaly, 64>;
        }
    }

    mock! {
        pub Sink {}
        impl ConfirmedEventSink for Sink {
            fn emit_confirmed(&mut self, event: &ConfirmedEvent) -> Result<(), EmitError>;
            fn request_robot_task(&mut self, event: &ConfirmedEvent) -> Result<(), EmitError>;
        }
    }

    fn zone_a() -> ZoneId {
        ZoneId([1, 0, 0, 0])
    }

    fn pest_anomaly() -> Anomaly {
        Anomaly {
            category: EventCategory::Pest { species_hint: Some(PestSpecies::Unknown) },
            score: 0.90,
            zone: zone_a(),
            timestamp: Timestamp::from_secs(1000),
        }
    }

    fn nearby_pest(distance: f32, time_offset_secs: u64) -> NeighborAnomaly {
        NeighborAnomaly {
            source_zone: zone_a(),
            category: EventCategory::Pest { species_hint: Some(PestSpecies::Unknown) },
            score: 0.88,
            timestamp: Timestamp::from_secs(1000 - time_offset_secs),
            distance_m: distance,
        }
    }

    fn nearby_flood(distance: f32, time_offset_secs: u64) -> NeighborAnomaly {
        NeighborAnomaly {
            source_zone: zone_a(),
            category: EventCategory::Flood { severity: FloodSeverity::Warning, upstream_origin: None },
            score: 0.85,
            timestamp: Timestamp::from_secs(1000 - time_offset_secs),
            distance_m: distance,
        }
    }

    #[test]
    fn confirms_when_quorum_met() {
        let engine = ConsensusEngine::new(3, 300, 500.0);
        let mut source = MockSource::new();
        let mut sink = MockSink::new();

        source.expect_local_anomalies()
            .returning(|| {
                let mut v = heapless::Vec::new();
                let _ = v.push(pest_anomaly());
                v
            });

        source.expect_neighbor_anomalies()
            .returning(|_| {
                let mut v = heapless::Vec::new();
                let _ = v.push(nearby_pest(100.0, 30));
                let _ = v.push(nearby_pest(200.0, 60));
                let _ = v.push(nearby_pest(150.0, 90));
                v
            });

        sink.expect_emit_confirmed()
            .withf(|e| matches!(e.category, EventCategory::Pest { .. }) && e.confidence > 0.8)
            .times(1)
            .returning(|_| Ok(()));

        let confirmed = engine.evaluate(&source, &mut sink).unwrap();
        assert_eq!(confirmed.len(), 1);
    }

    #[test]
    fn does_not_confirm_below_quorum() {
        let engine = ConsensusEngine::new(3, 300, 500.0);
        let mut source = MockSource::new();
        let mut sink = MockSink::new();

        source.expect_local_anomalies()
            .returning(|| {
                let mut v = heapless::Vec::new();
                let _ = v.push(pest_anomaly());
                v
            });

        source.expect_neighbor_anomalies()
            .returning(|_| {
                let mut v = heapless::Vec::new();
                let _ = v.push(nearby_pest(100.0, 30));
                // Only 1 corroboration, need 3
                v
            });

        sink.expect_emit_confirmed().times(0);

        let confirmed = engine.evaluate(&source, &mut sink).unwrap();
        assert!(confirmed.is_empty());
    }

    #[test]
    fn filters_temporally_distant() {
        let engine = ConsensusEngine::new(3, 300, 500.0);
        let mut source = MockSource::new();
        let mut sink = MockSink::new();

        source.expect_local_anomalies()
            .returning(|| {
                let mut v = heapless::Vec::new();
                let _ = v.push(pest_anomaly());
                v
            });

        source.expect_neighbor_anomalies()
            .returning(|_| {
                let mut v = heapless::Vec::new();
                let _ = v.push(nearby_pest(100.0, 30));   // within window
                let _ = v.push(nearby_pest(100.0, 400));  // outside 300s window
                let _ = v.push(nearby_pest(100.0, 500));  // outside 300s window
                v
            });

        sink.expect_emit_confirmed().times(0);

        let confirmed = engine.evaluate(&source, &mut sink).unwrap();
        assert!(confirmed.is_empty());
    }

    #[test]
    fn filters_spatially_distant() {
        let engine = ConsensusEngine::new(3, 300, 500.0);
        let mut source = MockSource::new();
        let mut sink = MockSink::new();

        source.expect_local_anomalies()
            .returning(|| {
                let mut v = heapless::Vec::new();
                let _ = v.push(pest_anomaly());
                v
            });

        source.expect_neighbor_anomalies()
            .returning(|_| {
                let mut v = heapless::Vec::new();
                let _ = v.push(nearby_pest(100.0, 30));    // within radius
                let _ = v.push(nearby_pest(1000.0, 30));   // too far
                let _ = v.push(nearby_pest(5000.0, 30));   // too far
                v
            });

        sink.expect_emit_confirmed().times(0);

        let confirmed = engine.evaluate(&source, &mut sink).unwrap();
        assert!(confirmed.is_empty());
    }

    #[test]
    fn filters_different_category() {
        let engine = ConsensusEngine::new(3, 300, 500.0);
        let mut source = MockSource::new();
        let mut sink = MockSink::new();

        source.expect_local_anomalies()
            .returning(|| {
                let mut v = heapless::Vec::new();
                let _ = v.push(pest_anomaly()); // Pest
                v
            });

        source.expect_neighbor_anomalies()
            .returning(|_| {
                let mut v = heapless::Vec::new();
                let _ = v.push(nearby_pest(100.0, 30));    // pest ✓
                let _ = v.push(nearby_flood(100.0, 30));   // flood ✗
                let _ = v.push(nearby_flood(100.0, 60));   // flood ✗
                v
            });

        sink.expect_emit_confirmed().times(0);

        let confirmed = engine.evaluate(&source, &mut sink).unwrap();
        assert!(confirmed.is_empty());
    }

    #[test]
    fn flood_gets_preemptive_alert_action() {
        let engine = ConsensusEngine::new(2, 300, 500.0); // lower quorum for test
        let mut source = MockSource::new();
        let mut sink = MockSink::new();

        source.expect_local_anomalies()
            .returning(|| {
                let mut v = heapless::Vec::new();
                let _ = v.push(Anomaly {
                    category: EventCategory::Flood {
                        severity: FloodSeverity::Emergency,
                        upstream_origin: None,
                    },
                    score: 0.95,
                    zone: zone_a(),
                    timestamp: Timestamp::from_secs(1000),
                });
                v
            });

        source.expect_neighbor_anomalies()
            .returning(|_| {
                let mut v = heapless::Vec::new();
                let _ = v.push(nearby_flood(100.0, 30));
                let _ = v.push(nearby_flood(200.0, 60));
                v
            });

        sink.expect_emit_confirmed()
            .withf(|e| matches!(e.recommended_action, RecommendedAction::PreemptiveAlert))
            .times(1)
            .returning(|_| Ok(()));

        engine.evaluate(&source, &mut sink).unwrap();
    }
}
