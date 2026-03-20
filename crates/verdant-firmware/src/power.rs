use verdant_core::config::DUTY_CYCLE_MS;
use verdant_core::traits::PowerMonitor;
use verdant_core::types::BatteryLevel;

/// Default duty cycle when battery is healthy.
const NORMAL_SLEEP_MS: u32 = DUTY_CYCLE_MS;

/// Extended sleep when battery is low (5 minutes).
const LOW_BATTERY_SLEEP_MS: u32 = 300_000;

/// Emergency sleep when battery is critical (30 minutes).
const CRITICAL_BATTERY_SLEEP_MS: u32 = 1_800_000;

/// Power manager that adapts duty cycling to battery/solar state.
///
/// On ESP32, reads ADC channels for battery voltage and solar panel
/// output. On x86 test builds, the values are injected.
pub struct PowerManager {
    battery: BatteryLevel,
    solar_mw: u16,
}

impl PowerManager {
    pub fn new(battery: BatteryLevel, solar_mw: u16) -> Self {
        Self { battery, solar_mw }
    }

    /// Update readings (called before each cycle in production).
    pub fn update(&mut self, battery: BatteryLevel, solar_mw: u16) {
        self.battery = battery;
        self.solar_mw = solar_mw;
    }
}

impl PowerMonitor for PowerManager {
    fn battery_level(&self) -> BatteryLevel {
        self.battery
    }

    fn solar_output_mw(&self) -> u16 {
        self.solar_mw
    }

    fn sleep_duration(&self) -> u32 {
        if self.battery.is_critical() {
            CRITICAL_BATTERY_SLEEP_MS
        } else if self.battery.is_low() {
            LOW_BATTERY_SLEEP_MS
        } else {
            NORMAL_SLEEP_MS
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_battery_normal_sleep() {
        let pm = PowerManager::new(BatteryLevel(0.80), 100);
        assert_eq!(pm.sleep_duration(), NORMAL_SLEEP_MS);
    }

    #[test]
    fn low_battery_extended_sleep() {
        let pm = PowerManager::new(BatteryLevel(0.15), 50);
        assert_eq!(pm.sleep_duration(), LOW_BATTERY_SLEEP_MS);
    }

    #[test]
    fn critical_battery_emergency_sleep() {
        let pm = PowerManager::new(BatteryLevel(0.05), 0);
        assert_eq!(pm.sleep_duration(), CRITICAL_BATTERY_SLEEP_MS);
    }

    #[test]
    fn update_changes_readings() {
        let mut pm = PowerManager::new(BatteryLevel(0.80), 100);
        assert_eq!(pm.sleep_duration(), NORMAL_SLEEP_MS);

        pm.update(BatteryLevel(0.05), 0);
        assert_eq!(pm.sleep_duration(), CRITICAL_BATTERY_SLEEP_MS);
    }
}
