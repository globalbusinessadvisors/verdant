#[allow(unused_imports)]
use micromath::F32Ext; // provides sqrt() in no_std

use crate::hal::RawCsiBuffer;

/// Extracted features from a WiFi CSI capture.
#[derive(Clone, Debug)]
pub struct CsiFeatures {
    /// Variance of subcarrier amplitudes — high = movement detected.
    pub amplitude_variance: f32,
    /// Standard deviation of phase shifts — correlates with object size.
    pub phase_std_dev: f32,
    /// Temporal autocorrelation — high = periodic activity, low = transient.
    pub temporal_autocorrelation: f32,
    /// Dominant frequency component in amplitude spectrum.
    pub dominant_frequency: f32,
    /// Number of subcarriers in the capture.
    pub subcarrier_count: u8,
}

/// Extracts statistical features from raw WiFi CSI data.
pub struct CsiFeatureExtractor;

impl CsiFeatureExtractor {
    /// Extract features from a raw CSI buffer.
    ///
    /// All computations use f32 arithmetic suitable for fixed-point
    /// hardware (no f64).
    pub fn extract(buffer: &RawCsiBuffer) -> CsiFeatures {
        let n = buffer.subcarriers.len();
        if n == 0 {
            return CsiFeatures {
                amplitude_variance: 0.0,
                phase_std_dev: 0.0,
                temporal_autocorrelation: 0.0,
                dominant_frequency: 0.0,
                subcarrier_count: 0,
            };
        }

        let n_f = n as f32;

        // --- Amplitude statistics ---
        let amp_sum: f32 = buffer
            .subcarriers
            .iter()
            .map(|s| s.amplitude as f32)
            .sum();
        let amp_mean = amp_sum / n_f;

        let amp_var: f32 = buffer
            .subcarriers
            .iter()
            .map(|s| {
                let diff = s.amplitude as f32 - amp_mean;
                diff * diff
            })
            .sum::<f32>()
            / n_f;

        // --- Phase statistics ---
        let phase_sum: f32 = buffer
            .subcarriers
            .iter()
            .map(|s| s.phase as f32)
            .sum();
        let phase_mean = phase_sum / n_f;

        let phase_var: f32 = buffer
            .subcarriers
            .iter()
            .map(|s| {
                let diff = s.phase as f32 - phase_mean;
                diff * diff
            })
            .sum::<f32>()
            / n_f;
        let phase_std = phase_var.sqrt();

        // --- Temporal autocorrelation (lag-1 on amplitudes) ---
        let autocorr = if n >= 2 {
            let mut sum_product = 0.0f32;
            for i in 1..n {
                let a = buffer.subcarriers[i - 1].amplitude as f32 - amp_mean;
                let b = buffer.subcarriers[i].amplitude as f32 - amp_mean;
                sum_product += a * b;
            }
            if amp_var > 0.0 {
                sum_product / ((n - 1) as f32 * amp_var)
            } else {
                0.0
            }
        } else {
            0.0
        };

        // --- Dominant frequency (index of max amplitude) ---
        let dominant_idx = buffer
            .subcarriers
            .iter()
            .enumerate()
            .max_by_key(|(_, s)| s.amplitude.unsigned_abs())
            .map(|(i, _)| i)
            .unwrap_or(0);
        let dominant_frequency = dominant_idx as f32 / n_f;

        CsiFeatures {
            amplitude_variance: amp_var,
            phase_std_dev: phase_std,
            temporal_autocorrelation: autocorr.clamp(-1.0, 1.0),
            dominant_frequency,
            subcarrier_count: n as u8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use verdant_core::types::SubcarrierData;

    fn uniform_buffer(amplitude: i16, phase: i16, count: usize) -> RawCsiBuffer {
        let mut subcarriers = heapless::Vec::new();
        for _ in 0..count {
            let _ = subcarriers.push(SubcarrierData { amplitude, phase });
        }
        RawCsiBuffer {
            subcarriers,
            duration_ms: 5000,
        }
    }

    fn varied_buffer() -> RawCsiBuffer {
        let mut subcarriers = heapless::Vec::new();
        // Deliberately varied amplitudes to simulate movement
        for i in 0..32 {
            let amp = if i % 2 == 0 { 1000 } else { -500 };
            let _ = subcarriers.push(SubcarrierData {
                amplitude: amp,
                phase: (i * 10) as i16,
            });
        }
        RawCsiBuffer {
            subcarriers,
            duration_ms: 5000,
        }
    }

    #[test]
    fn extract_low_variance_for_still() {
        let buffer = uniform_buffer(500, 100, 32);
        let features = CsiFeatureExtractor::extract(&buffer);
        assert!(
            features.amplitude_variance < 1.0,
            "Expected near-zero variance, got {}",
            features.amplitude_variance
        );
    }

    #[test]
    fn extract_high_variance_for_movement() {
        let buffer = varied_buffer();
        let features = CsiFeatureExtractor::extract(&buffer);
        assert!(
            features.amplitude_variance > 1000.0,
            "Expected high variance, got {}",
            features.amplitude_variance
        );
    }

    #[test]
    fn extract_phase_std_dev_zero_for_uniform() {
        let buffer = uniform_buffer(500, 100, 32);
        let features = CsiFeatureExtractor::extract(&buffer);
        assert!(features.phase_std_dev < 1.0);
    }

    #[test]
    fn extract_phase_std_dev_nonzero_for_varied() {
        let buffer = varied_buffer();
        let features = CsiFeatureExtractor::extract(&buffer);
        assert!(features.phase_std_dev > 1.0);
    }

    #[test]
    fn extract_empty_buffer() {
        let buffer = RawCsiBuffer {
            subcarriers: heapless::Vec::new(),
            duration_ms: 0,
        };
        let features = CsiFeatureExtractor::extract(&buffer);
        assert_eq!(features.subcarrier_count, 0);
        assert_eq!(features.amplitude_variance, 0.0);
    }

    #[test]
    fn autocorrelation_bounded() {
        let buffer = varied_buffer();
        let features = CsiFeatureExtractor::extract(&buffer);
        assert!(features.temporal_autocorrelation >= -1.0);
        assert!(features.temporal_autocorrelation <= 1.0);
    }

    #[test]
    fn dominant_frequency_bounded() {
        let buffer = varied_buffer();
        let features = CsiFeatureExtractor::extract(&buffer);
        assert!(features.dominant_frequency >= 0.0);
        assert!(features.dominant_frequency <= 1.0);
    }
}
