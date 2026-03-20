use verdant_core::config::RAW_FEATURE_DIM;
use verdant_core::error::SenseError;
use verdant_core::types::RawFeatures;

use crate::csi::CsiFeatureExtractor;
use crate::hal::{CsiHardware, HalError, SensorHardware};

/// Fuses WiFi CSI and environmental sensor data into a single
/// [`RawFeatures`] vector suitable for embedding projection.
pub struct SensorFusion<C: CsiHardware, S: SensorHardware> {
    csi_hw: C,
    sensor_hw: S,
}

impl<C: CsiHardware, S: SensorHardware> SensorFusion<C, S> {
    pub fn new(csi_hw: C, sensor_hw: S) -> Self {
        Self { csi_hw, sensor_hw }
    }

    /// Capture CSI and read all environmental sensors, then fuse into
    /// a single feature vector.
    ///
    /// The output vector layout (RAW_FEATURE_DIM = 40 elements):
    /// - \[0\]:  CSI amplitude variance (normalized)
    /// - \[1\]:  CSI phase std deviation (normalized)
    /// - \[2\]:  CSI temporal autocorrelation \[-1, 1\]
    /// - \[3\]:  CSI dominant frequency \[0, 1\]
    /// - \[4\]:  CSI subcarrier count / 64.0
    /// - \[5\]:  Temperature (normalized to \[-1, 1\] over \[-40, 60\] °C range)
    /// - \[6\]:  Humidity (normalized to \[0, 1\])
    /// - \[7\]:  Soil moisture (normalized to \[0, 1\])
    /// - \[8\]:  Pressure (normalized around 101325 Pa)
    /// - \[9\]:  Pressure delta (normalized)
    /// - \[10\]: Light (normalized to \[0, 1\] over \[0, 10000\] lux)
    /// - \[11..40\]: reserved (zero-filled)
    pub fn capture_and_fuse(
        &mut self,
        csi_duration_ms: u32,
    ) -> Result<RawFeatures, SenseError> {
        // Capture CSI
        let csi_buf = self
            .csi_hw
            .capture_raw(csi_duration_ms)
            .map_err(hal_to_sense)?;
        let csi_features = CsiFeatureExtractor::extract(&csi_buf);

        // Read environmental sensors
        let temp = self.sensor_hw.read_temperature().map_err(hal_to_sense)?;
        let humidity = self.sensor_hw.read_humidity().map_err(hal_to_sense)?;
        let soil = self.sensor_hw.read_soil_moisture().map_err(hal_to_sense)?;
        let pressure = self.sensor_hw.read_pressure().map_err(hal_to_sense)?;
        let light = self.sensor_hw.read_light().map_err(hal_to_sense)?;

        // Build feature vector
        let mut data = heapless::Vec::<f32, RAW_FEATURE_DIM>::new();

        // CSI features (indices 0–4)
        let _ = data.push(csi_features.amplitude_variance / 10000.0);  // normalize
        let _ = data.push(csi_features.phase_std_dev / 1000.0);        // normalize
        let _ = data.push(csi_features.temporal_autocorrelation);
        let _ = data.push(csi_features.dominant_frequency);
        let _ = data.push(csi_features.subcarrier_count as f32 / 64.0);

        // Environmental features (indices 5–10)
        // Temperature: map [-4000, 6000] centidegrees to [-1, 1]
        let _ = data.push((temp as f32) / 5000.0);
        // Humidity: map [0, 10000] hundredths to [0, 1]
        let _ = data.push((humidity as f32) / 10000.0);
        // Soil moisture: [0, 10000] to [0, 1]
        let _ = data.push((soil as f32) / 10000.0);
        // Pressure: center around 101325 Pa, normalize ±5000 Pa
        let _ = data.push((pressure as f32 - 101325.0) / 5000.0);
        // Pressure delta: just scale down
        // Light: [0, 10000] lux to [0, 1]
        let _ = data.push((light as f32) / 10000.0);

        // Fill remaining slots with zeros
        while data.len() < RAW_FEATURE_DIM {
            let _ = data.push(0.0);
        }

        Ok(RawFeatures { data })
    }
}

fn hal_to_sense(e: HalError) -> SenseError {
    match e {
        HalError::Timeout => SenseError::CsiCaptureTimeout,
        HalError::HardwareFault => SenseError::CsiHardwareFault,
        _ => SenseError::SensorReadFailed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hal::{CsiConfig, RawCsiBuffer};
    use mockall::mock;
    use verdant_core::types::SubcarrierData;

    mock! {
        pub CsiHw {}
        impl CsiHardware for CsiHw {
            fn configure_csi(&mut self, config: &CsiConfig) -> Result<(), HalError>;
            fn capture_raw(&mut self, duration_ms: u32) -> Result<RawCsiBuffer, HalError>;
        }
    }

    mock! {
        pub SensorHw {}
        impl SensorHardware for SensorHw {
            fn read_temperature(&mut self) -> Result<i16, HalError>;
            fn read_humidity(&mut self) -> Result<u16, HalError>;
            fn read_soil_moisture(&mut self) -> Result<u16, HalError>;
            fn read_pressure(&mut self) -> Result<u32, HalError>;
            fn read_light(&mut self) -> Result<u16, HalError>;
        }
    }

    fn normal_csi_buffer() -> RawCsiBuffer {
        let mut subcarriers = heapless::Vec::new();
        for i in 0..32 {
            let _ = subcarriers.push(SubcarrierData {
                amplitude: 500 + (i * 10) as i16,
                phase: 100 + (i * 5) as i16,
            });
        }
        RawCsiBuffer {
            subcarriers,
            duration_ms: 5000,
        }
    }

    fn setup_mocks() -> (MockCsiHw, MockSensorHw) {
        let mut csi = MockCsiHw::new();
        csi.expect_capture_raw()
            .returning(|_| Ok(normal_csi_buffer()));

        let mut sensors = MockSensorHw::new();
        sensors.expect_read_temperature().returning(|| Ok(2150));
        sensors.expect_read_humidity().returning(|| Ok(6500));
        sensors.expect_read_soil_moisture().returning(|| Ok(4200));
        sensors.expect_read_pressure().returning(|| Ok(101325));
        sensors.expect_read_light().returning(|| Ok(850));

        (csi, sensors)
    }

    #[test]
    fn capture_and_fuse_reads_all_sensors() {
        let mut csi = MockCsiHw::new();
        csi.expect_capture_raw()
            .times(1)
            .returning(|_| Ok(normal_csi_buffer()));

        let mut sensors = MockSensorHw::new();
        sensors.expect_read_temperature().times(1).returning(|| Ok(2150));
        sensors.expect_read_humidity().times(1).returning(|| Ok(6500));
        sensors.expect_read_soil_moisture().times(1).returning(|| Ok(4200));
        sensors.expect_read_pressure().times(1).returning(|| Ok(101325));
        sensors.expect_read_light().times(1).returning(|| Ok(850));

        let mut fusion = SensorFusion::new(csi, sensors);
        let _result = fusion.capture_and_fuse(5000).unwrap();
        // Mock expectations verified on drop
    }

    #[test]
    fn capture_and_fuse_captures_csi_with_correct_duration() {
        let mut csi = MockCsiHw::new();
        csi.expect_capture_raw()
            .withf(|dur| *dur == 5000)
            .times(1)
            .returning(|_| Ok(normal_csi_buffer()));

        let mut sensors = MockSensorHw::new();
        sensors.expect_read_temperature().returning(|| Ok(2000));
        sensors.expect_read_humidity().returning(|| Ok(5000));
        sensors.expect_read_soil_moisture().returning(|| Ok(3000));
        sensors.expect_read_pressure().returning(|| Ok(101000));
        sensors.expect_read_light().returning(|| Ok(500));

        let mut fusion = SensorFusion::new(csi, sensors);
        fusion.capture_and_fuse(5000).unwrap();
    }

    #[test]
    fn capture_and_fuse_returns_correct_dimension() {
        let (csi, sensors) = setup_mocks();
        let mut fusion = SensorFusion::new(csi, sensors);
        let result = fusion.capture_and_fuse(5000).unwrap();
        assert_eq!(result.data.len(), RAW_FEATURE_DIM);
    }

    #[test]
    fn capture_and_fuse_propagates_sensor_error() {
        let mut csi = MockCsiHw::new();
        csi.expect_capture_raw()
            .returning(|_| Ok(normal_csi_buffer()));

        let mut sensors = MockSensorHw::new();
        sensors
            .expect_read_temperature()
            .returning(|| Err(HalError::BusError));
        // Subsequent sensor reads should not be called
        sensors.expect_read_humidity().times(0);
        sensors.expect_read_soil_moisture().times(0);
        sensors.expect_read_pressure().times(0);
        sensors.expect_read_light().times(0);

        let mut fusion = SensorFusion::new(csi, sensors);
        let result = fusion.capture_and_fuse(5000);
        assert_eq!(result.unwrap_err(), SenseError::SensorReadFailed);
    }

    #[test]
    fn capture_and_fuse_propagates_csi_error() {
        let mut csi = MockCsiHw::new();
        csi.expect_capture_raw()
            .returning(|_| Err(HalError::Timeout));

        let sensors = MockSensorHw::new();
        // No sensor reads if CSI fails first

        let mut fusion = SensorFusion::new(csi, sensors);
        let result = fusion.capture_and_fuse(5000);
        assert_eq!(result.unwrap_err(), SenseError::CsiCaptureTimeout);
    }

    #[test]
    fn feature_values_are_reasonable() {
        let (csi, sensors) = setup_mocks();
        let mut fusion = SensorFusion::new(csi, sensors);
        let result = fusion.capture_and_fuse(5000).unwrap();

        // Temperature: 2150 centideg / 5000 = 0.43
        let temp_feat = result.data[5];
        assert!((temp_feat - 0.43).abs() < 0.01, "temp={temp_feat}");

        // Humidity: 6500 / 10000 = 0.65
        let hum_feat = result.data[6];
        assert!((hum_feat - 0.65).abs() < 0.01, "hum={hum_feat}");

        // Soil: 4200 / 10000 = 0.42
        let soil_feat = result.data[7];
        assert!((soil_feat - 0.42).abs() < 0.01, "soil={soil_feat}");

        // Pressure: (101325 - 101325) / 5000 = 0.0
        let press_feat = result.data[8];
        assert!(press_feat.abs() < 0.01, "press={press_feat}");

        // Light: 850 / 10000 = 0.085
        let light_feat = result.data[9];
        assert!((light_feat - 0.085).abs() < 0.01, "light={light_feat}");
    }
}
