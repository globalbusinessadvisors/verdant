use verdant_core::types::SensorReading;

/// Generates synthetic weather data for simulation.
///
/// Provides seasonally-varying sensor readings with configurable
/// anomaly injection.
pub struct WeatherGenerator {
    base_temperature: i16,   // centidegrees
    base_humidity: u16,      // hundredths of percent
    base_soil_moisture: u16, // 0-10000
    base_pressure: u32,      // Pascals
    base_light: u16,         // lux
}

impl WeatherGenerator {
    /// Create a generator with typical Vermont summer baseline values.
    pub fn vermont_summer() -> Self {
        Self {
            base_temperature: 2200, // 22.0°C
            base_humidity: 6500,    // 65%
            base_soil_moisture: 4000,
            base_pressure: 101325,
            base_light: 3000,
        }
    }

    /// Create a generator with typical Vermont winter baseline values.
    pub fn vermont_winter() -> Self {
        Self {
            base_temperature: -500, // -5.0°C
            base_humidity: 7500,
            base_soil_moisture: 6000,
            base_pressure: 101800,
            base_light: 500,
        }
    }

    /// Generate a normal sensor reading with small random variation.
    pub fn normal_reading(&self, tick: u64) -> SensorReading {
        // Simple deterministic variation based on tick
        let variation = ((tick % 100) as i16) - 50;
        SensorReading {
            temperature: self.base_temperature + variation,
            humidity: self.base_humidity.saturating_add_signed(variation),
            soil_moisture: self.base_soil_moisture.saturating_add_signed(variation),
            pressure: (self.base_pressure as i64 + variation as i64).max(90000) as u32,
            pressure_delta: variation / 10,
            light: self.base_light.saturating_add_signed(variation * 2),
        }
    }

    /// Generate a flood-indicative sensor reading.
    pub fn flood_reading(&self, tick: u64) -> SensorReading {
        let mut reading = self.normal_reading(tick);
        reading.soil_moisture = 9500;    // near saturation
        reading.pressure_delta = -80;     // rapid pressure drop
        reading.humidity = 9500;
        reading
    }

    /// Generate a pest-indicative sensor reading (normal sensors, anomaly is in CSI).
    pub fn pest_reading(&self, tick: u64) -> SensorReading {
        self.normal_reading(tick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summer_baseline() {
        let wg = WeatherGenerator::vermont_summer();
        let r = wg.normal_reading(50);
        assert!(r.temperature > 2000 && r.temperature < 2500);
        assert!(r.humidity > 6000 && r.humidity < 7000);
    }

    #[test]
    fn flood_has_high_moisture() {
        let wg = WeatherGenerator::vermont_summer();
        let r = wg.flood_reading(0);
        assert!(r.soil_moisture > 9000);
        assert!(r.pressure_delta < -50);
    }

    #[test]
    fn readings_vary_by_tick() {
        let wg = WeatherGenerator::vermont_summer();
        let r1 = wg.normal_reading(10);
        let r2 = wg.normal_reading(60);
        assert_ne!(r1.temperature, r2.temperature);
    }
}
