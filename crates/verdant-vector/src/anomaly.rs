use verdant_core::types::{
    ClimateType, EventCategory, FloodSeverity, InfrastructureType, PestSpecies, SensorReading,
    Embedding,
};

use crate::embedding::cosine_distance;

/// Compute the anomaly score between a current observation and a baseline.
///
/// Returns `[0.0, 1.0]` where `0.0` = identical direction (normal) and
/// `1.0` = maximally anomalous.
pub fn score_anomaly(current: &Embedding, baseline: &Embedding) -> f32 {
    cosine_distance(current, baseline)
}

/// Classify an anomaly based on the embedding score and sensor context.
///
/// Uses simple threshold rules on the raw sensor reading to determine
/// the most likely event category when an anomaly has been detected.
pub fn classify_anomaly(
    _embedding: &Embedding,
    _score: f32,
    sensor: &SensorReading,
) -> EventCategory {
    // Flood: high soil moisture AND pressure dropping
    if sensor.soil_moisture > 8000 && sensor.pressure_delta < -50 {
        return EventCategory::Flood {
            severity: if sensor.soil_moisture > 9500 {
                FloodSeverity::Emergency
            } else if sensor.soil_moisture > 9000 {
                FloodSeverity::Warning
            } else {
                FloodSeverity::Watch
            },
            upstream_origin: None,
        };
    }

    // Fire: high temperature AND low humidity AND light anomaly
    if sensor.temperature > 4000 && sensor.humidity < 2000 {
        return EventCategory::Fire {
            smoke_density: (sensor.temperature as f32 - 4000.0) / 1000.0,
        };
    }

    // Infrastructure: pipe freeze — very low temp AND pressure drop
    if sensor.temperature < -1000 && sensor.pressure_delta < -100 {
        return EventCategory::Infrastructure {
            sub_type: InfrastructureType::PipeFreeze,
        };
    }

    // Climate: freeze-thaw cycle — temperature near zero with oscillation
    if sensor.temperature > -200 && sensor.temperature < 200 && sensor.pressure_delta.abs() > 30 {
        return EventCategory::Climate {
            sub_type: ClimateType::FreezeThaw,
        };
    }

    // Default: pest / biological (CSI-dominated anomaly without clear sensor cause)
    EventCategory::Pest {
        species_hint: Some(PestSpecies::Unknown),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use verdant_core::config::EMBEDDING_DIM;

    fn emb(vals: &[i16]) -> Embedding {
        let mut data = [0i16; EMBEDDING_DIM];
        for (i, &v) in vals.iter().enumerate().take(EMBEDDING_DIM) {
            data[i] = v;
        }
        Embedding { data }
    }

    #[test]
    fn high_score_when_far_from_baseline() {
        let baseline = emb(&[1000, 0, 0, 0]);
        let mut current_data = [0i16; EMBEDDING_DIM];
        current_data[1] = 1000; // orthogonal
        let current = Embedding { data: current_data };
        let score = score_anomaly(&current, &baseline);
        assert!(score > 0.4, "Expected score > 0.4, got {score}");
    }

    #[test]
    fn low_score_when_near_baseline() {
        let baseline = emb(&[1000, 500, 250]);
        let current = emb(&[990, 505, 248]);
        let score = score_anomaly(&current, &baseline);
        assert!(score < 0.1, "Expected score < 0.1, got {score}");
    }

    #[test]
    fn classify_returns_flood_when_moisture_spike() {
        let sensor = SensorReading {
            temperature: 1500,
            humidity: 9000,
            soil_moisture: 9200,
            pressure: 100500,
            pressure_delta: -80,
            light: 200,
        };
        let cat = classify_anomaly(&Embedding::zero(), 0.9, &sensor);
        assert!(matches!(cat, EventCategory::Flood { severity: FloodSeverity::Warning, .. }));
    }

    #[test]
    fn classify_returns_flood_emergency_extreme_moisture() {
        let sensor = SensorReading {
            temperature: 1500,
            humidity: 9000,
            soil_moisture: 9800,
            pressure: 100500,
            pressure_delta: -80,
            light: 200,
        };
        let cat = classify_anomaly(&Embedding::zero(), 0.9, &sensor);
        assert!(matches!(cat, EventCategory::Flood { severity: FloodSeverity::Emergency, .. }));
    }

    #[test]
    fn classify_returns_fire_on_high_temp_low_humidity() {
        let sensor = SensorReading {
            temperature: 4500,
            humidity: 1500,
            soil_moisture: 1000,
            pressure: 101000,
            pressure_delta: 0,
            light: 5000,
        };
        let cat = classify_anomaly(&Embedding::zero(), 0.9, &sensor);
        assert!(matches!(cat, EventCategory::Fire { .. }));
    }

    #[test]
    fn classify_returns_pipe_freeze() {
        let sensor = SensorReading {
            temperature: -2000,
            humidity: 8000,
            soil_moisture: 3000,
            pressure: 101000,
            pressure_delta: -150,
            light: 0,
        };
        let cat = classify_anomaly(&Embedding::zero(), 0.9, &sensor);
        assert!(matches!(
            cat,
            EventCategory::Infrastructure { sub_type: InfrastructureType::PipeFreeze }
        ));
    }

    #[test]
    fn classify_defaults_to_pest() {
        let sensor = SensorReading {
            temperature: 2000,
            humidity: 6000,
            soil_moisture: 4000,
            pressure: 101325,
            pressure_delta: 0,
            light: 500,
        };
        let cat = classify_anomaly(&Embedding::zero(), 0.9, &sensor);
        assert!(matches!(cat, EventCategory::Pest { .. }));
    }
}
