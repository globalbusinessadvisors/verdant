use verdant_core::config::MAX_SUBCARRIERS;
use verdant_core::types::SubcarrierData;

/// Errors from low-level hardware operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HalError {
    /// Hardware not responding or not initialized.
    HardwareFault,
    /// I2C/SPI bus error.
    BusError,
    /// Capture timed out.
    Timeout,
    /// Invalid configuration parameter.
    InvalidConfig,
}

/// CSI channel bandwidth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CsiBandwidth {
    Mhz20,
    Mhz40,
}

/// Configuration for WiFi CSI capture.
#[derive(Clone, Debug)]
pub struct CsiConfig {
    pub channel: u8,
    pub bandwidth: CsiBandwidth,
}

/// Raw CSI capture buffer from hardware.
#[derive(Clone, Debug)]
pub struct RawCsiBuffer {
    pub subcarriers: heapless::Vec<SubcarrierData, MAX_SUBCARRIERS>,
    pub duration_ms: u32,
}

/// Low-level WiFi CSI hardware abstraction.
///
/// Implemented by ESP32-specific drivers; mocked for testing.
pub trait CsiHardware {
    fn configure_csi(&mut self, config: &CsiConfig) -> Result<(), HalError>;
    fn capture_raw(&mut self, duration_ms: u32) -> Result<RawCsiBuffer, HalError>;
}

/// Low-level environmental sensor hardware abstraction.
///
/// Wraps I2C/SPI/ADC access to BME280, soil moisture probe, and BH1750.
pub trait SensorHardware {
    /// Temperature in centidegrees Celsius (e.g. 2150 = 21.50 °C).
    fn read_temperature(&mut self) -> Result<i16, HalError>;
    /// Relative humidity in hundredths of percent (e.g. 6500 = 65.00%).
    fn read_humidity(&mut self) -> Result<u16, HalError>;
    /// Soil moisture 0–10000 (dry to saturated).
    fn read_soil_moisture(&mut self) -> Result<u16, HalError>;
    /// Barometric pressure in Pascals.
    fn read_pressure(&mut self) -> Result<u32, HalError>;
    /// Ambient light in lux.
    fn read_light(&mut self) -> Result<u16, HalError>;
}
