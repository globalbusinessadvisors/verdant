use verdant_core::error::SenseError;
use verdant_core::traits::EnvironmentalSensor;
use verdant_core::types::SensorReading;

use crate::hal::{HalError, SensorHardware};

/// Ring buffer capacity for pressure history (for delta computation).
const PRESSURE_HISTORY_LEN: usize = 8;

/// Wraps low-level sensor hardware into the [`EnvironmentalSensor`] trait,
/// adding pressure-delta computation from a rolling history.
pub struct EnvironmentalReader<S: SensorHardware> {
    hardware: S,
    pressure_history: [u32; PRESSURE_HISTORY_LEN],
    history_count: usize,
    history_idx: usize,
}

impl<S: SensorHardware> EnvironmentalReader<S> {
    pub fn new(hardware: S) -> Self {
        Self {
            hardware,
            pressure_history: [0; PRESSURE_HISTORY_LEN],
            history_count: 0,
            history_idx: 0,
        }
    }

    /// Compute the pressure rate of change (Pa/reading interval) from the
    /// rolling history. Returns 0 if fewer than 2 readings are available.
    fn compute_pressure_delta(&self, current: u32) -> i16 {
        if self.history_count == 0 {
            return 0;
        }

        // Compare with the oldest value in the buffer
        let oldest_idx = if self.history_count >= PRESSURE_HISTORY_LEN {
            self.history_idx // ring buffer: oldest is at current write position
        } else {
            0
        };
        let oldest = self.pressure_history[oldest_idx];
        let diff = current as i32 - oldest as i32;
        let divisor = self.history_count.min(PRESSURE_HISTORY_LEN) as i32;
        let delta = diff / divisor;
        delta.clamp(-32768, 32767) as i16
    }

    /// Push a pressure reading into the rolling history.
    fn push_pressure(&mut self, pressure: u32) {
        self.pressure_history[self.history_idx] = pressure;
        self.history_idx = (self.history_idx + 1) % PRESSURE_HISTORY_LEN;
        if self.history_count < PRESSURE_HISTORY_LEN {
            self.history_count += 1;
        }
    }
}

fn hal_to_sense(e: HalError) -> SenseError {
    match e {
        HalError::Timeout => SenseError::CsiCaptureTimeout,
        _ => SenseError::SensorReadFailed,
    }
}

impl<S: SensorHardware> EnvironmentalSensor for EnvironmentalReader<S> {
    fn read(&mut self) -> Result<SensorReading, SenseError> {
        let temperature = self.hardware.read_temperature().map_err(hal_to_sense)?;
        let humidity = self.hardware.read_humidity().map_err(hal_to_sense)?;
        let soil_moisture = self.hardware.read_soil_moisture().map_err(hal_to_sense)?;
        let pressure = self.hardware.read_pressure().map_err(hal_to_sense)?;
        let light = self.hardware.read_light().map_err(hal_to_sense)?;

        let pressure_delta = self.compute_pressure_delta(pressure);
        self.push_pressure(pressure);

        Ok(SensorReading {
            temperature,
            humidity,
            soil_moisture,
            pressure,
            pressure_delta,
            light,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;

    mock! {
        pub Sensors {}
        impl SensorHardware for Sensors {
            fn read_temperature(&mut self) -> Result<i16, HalError>;
            fn read_humidity(&mut self) -> Result<u16, HalError>;
            fn read_soil_moisture(&mut self) -> Result<u16, HalError>;
            fn read_pressure(&mut self) -> Result<u32, HalError>;
            fn read_light(&mut self) -> Result<u16, HalError>;
        }
    }

    fn setup_mock_returning(
        temp: i16,
        hum: u16,
        soil: u16,
        press: u32,
        light: u16,
    ) -> MockSensors {
        let mut mock = MockSensors::new();
        mock.expect_read_temperature().returning(move || Ok(temp));
        mock.expect_read_humidity().returning(move || Ok(hum));
        mock.expect_read_soil_moisture().returning(move || Ok(soil));
        mock.expect_read_pressure().returning(move || Ok(press));
        mock.expect_read_light().returning(move || Ok(light));
        mock
    }

    #[test]
    fn read_returns_sensor_values() {
        let mock = setup_mock_returning(2150, 6500, 4200, 101325, 850);
        let mut reader = EnvironmentalReader::new(mock);
        let reading = reader.read().unwrap();
        assert_eq!(reading.temperature, 2150);
        assert_eq!(reading.humidity, 6500);
        assert_eq!(reading.soil_moisture, 4200);
        assert_eq!(reading.pressure, 101325);
        assert_eq!(reading.light, 850);
    }

    #[test]
    fn read_computes_pressure_delta() {
        let mut mock = MockSensors::new();
        mock.expect_read_temperature().returning(|| Ok(2000));
        mock.expect_read_humidity().returning(|| Ok(5000));
        mock.expect_read_soil_moisture().returning(|| Ok(3000));
        mock.expect_read_light().returning(|| Ok(500));

        // First read at 101000, second at 101200
        let mut call_count = 0u32;
        mock.expect_read_pressure().returning(move || {
            call_count += 1;
            if call_count == 1 {
                Ok(101000)
            } else {
                Ok(101200)
            }
        });

        let mut reader = EnvironmentalReader::new(mock);

        let r1 = reader.read().unwrap();
        assert_eq!(r1.pressure_delta, 0); // first reading, no history

        let r2 = reader.read().unwrap();
        assert_eq!(r2.pressure_delta, 200); // (101200 - 101000) / 1
    }

    #[test]
    fn read_propagates_sensor_error() {
        let mut mock = MockSensors::new();
        mock.expect_read_temperature()
            .returning(|| Err(HalError::BusError));
        // Other methods should not be called
        mock.expect_read_humidity().times(0);
        mock.expect_read_soil_moisture().times(0);
        mock.expect_read_pressure().times(0);
        mock.expect_read_light().times(0);

        let mut reader = EnvironmentalReader::new(mock);
        let result = reader.read();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SenseError::SensorReadFailed);
    }

    #[test]
    fn pressure_delta_averages_over_history() {
        let mut mock = MockSensors::new();
        mock.expect_read_temperature().returning(|| Ok(2000));
        mock.expect_read_humidity().returning(|| Ok(5000));
        mock.expect_read_soil_moisture().returning(|| Ok(3000));
        mock.expect_read_light().returning(|| Ok(500));

        let mut call = 0u32;
        mock.expect_read_pressure().returning(move || {
            call += 1;
            // Pressure increases by 100 each reading
            Ok(100000 + call * 100)
        });

        let mut reader = EnvironmentalReader::new(mock);

        // Fill up several readings
        for _ in 0..5 {
            reader.read().unwrap();
        }

        let r = reader.read().unwrap();
        // Delta should be positive (pressure is increasing)
        assert!(r.pressure_delta > 0, "Expected positive delta, got {}", r.pressure_delta);
    }
}
