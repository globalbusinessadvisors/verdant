use core::cmp::Ordering;
use core::fmt;

use heapless::String;
use serde::{Deserialize, Serialize};

use crate::config::{EMBEDDING_DIM, MAX_FRAME_PAYLOAD, MAX_SUBCARRIERS, RAW_FEATURE_DIM};

// ---------------------------------------------------------------------------
// Identity value objects
// ---------------------------------------------------------------------------

/// Hardware-derived node identifier (from ESP32 eFuse).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct NodeId(pub [u8; 8]);

/// Zone identifier — the atomic unit of governance.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct ZoneId(pub [u8; 4]);

/// SHA-256 hash of a QuDAG message.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct MessageHash(pub [u8; 32]);

/// Content-addressed governance proposal identifier.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct ProposalHash(pub [u8; 32]);

/// Firmware image hash for OTA verification.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct FirmwareHash(pub [u8; 32]);

// ---------------------------------------------------------------------------
// Temporal / versioning value objects
// ---------------------------------------------------------------------------

/// Monotonic timestamp in milliseconds.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Timestamp(pub u64);

impl PartialOrd for Timestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Timestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Timestamp {
    pub const fn from_secs(secs: u64) -> Self {
        Self(secs * 1000)
    }

    pub const fn as_millis(&self) -> u64 {
        self.0
    }

    pub fn saturating_diff(&self, other: &Self) -> u64 {
        self.0.saturating_sub(other.0)
    }
}

/// Monotonically increasing vector graph version.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Version(pub u64);

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Version {
    pub const fn new(v: u64) -> Self {
        Self(v)
    }

    pub fn increment(&mut self) {
        self.0 = self.0.saturating_add(1);
    }
}

/// Semantic version for firmware releases.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

// ---------------------------------------------------------------------------
// Bounded string (no_std compatible)
// ---------------------------------------------------------------------------

/// Fixed-capacity string for governance titles and similar text.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct BoundedString<const N: usize>(String<N>);

impl<const N: usize> BoundedString<N> {
    /// Create from a `&str`. Returns `None` if the string exceeds capacity.
    pub fn new(s: &str) -> Option<Self> {
        String::try_from(s).ok().map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn capacity(&self) -> usize {
        N
    }
}

impl<const N: usize> fmt::Display for BoundedString<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Event classification
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EventCategory {
    Pest {
        species_hint: Option<PestSpecies>,
    },
    Flood {
        severity: FloodSeverity,
        upstream_origin: Option<ZoneId>,
    },
    Fire {
        smoke_density: f32,
    },
    Infrastructure {
        sub_type: InfrastructureType,
    },
    Wildlife {
        movement_type: MovementType,
    },
    Climate {
        sub_type: ClimateType,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FloodSeverity {
    Watch,
    Warning,
    Emergency,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PestSpecies {
    EmeraldAshBorer,
    HemlockWoollyAdelgid,
    BeechLeafDisease,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InfrastructureType {
    PipeFreeze,
    RoadWashout,
    PowerLine,
    BridgeDamage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MovementType {
    Migration,
    Corridor,
    Unusual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClimateType {
    FreezeThaw,
    Drought,
    ExtremeHeat,
    IceStorm,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RecommendedAction {
    AlertOnly,
    AlertAndMonitor,
    DispatchRobot { mission_type: MissionType },
    EvacuationWarning,
    PreemptiveAlert,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MissionType {
    SeedDispersal,
    WaterSensorDeploy,
    SupplyDelivery,
    VisualInspection,
    EmergencyRelay,
}

// ---------------------------------------------------------------------------
// Sensor data types
// ---------------------------------------------------------------------------

/// Raw WiFi CSI capture frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CsiFrame {
    pub subcarriers: heapless::Vec<SubcarrierData, MAX_SUBCARRIERS>,
    pub duration_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubcarrierData {
    pub amplitude: i16,
    pub phase: i16,
}

/// Fused environmental sensor reading.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SensorReading {
    /// Temperature in centidegrees Celsius (e.g. 2150 = 21.50 °C).
    pub temperature: i16,
    /// Relative humidity in hundredths of percent (e.g. 6500 = 65.00%).
    pub humidity: u16,
    /// Soil moisture 0–10000 (0 = dry, 10000 = saturated).
    pub soil_moisture: u16,
    /// Barometric pressure in Pascals.
    pub pressure: u32,
    /// Pressure rate of change in Pa/s (signed).
    pub pressure_delta: i16,
    /// Light level in lux.
    pub light: u16,
}

/// Concatenated raw feature vector consumed by `EmbeddingProjector`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawFeatures {
    pub data: heapless::Vec<f32, RAW_FEATURE_DIM>,
}

// ---------------------------------------------------------------------------
// Embedding types
// ---------------------------------------------------------------------------

/// Fixed-dimension environment embedding using i16 quantized values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Embedding {
    pub data: [i16; EMBEDDING_DIM],
}

impl Embedding {
    pub const fn zero() -> Self {
        Self {
            data: [0i16; EMBEDDING_DIM],
        }
    }
}

/// Week-of-year slot for seasonal baseline bucketing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SeasonSlot {
    /// Week number 0–51.
    pub week: u8,
}

impl SeasonSlot {
    pub const fn new(week: u8) -> Self {
        Self { week: if week > 51 { 51 } else { week } }
    }
}

// ---------------------------------------------------------------------------
// Anomaly / confirmed event types
// ---------------------------------------------------------------------------

/// A locally detected anomaly (not yet confirmed by consensus).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Anomaly {
    pub category: EventCategory,
    pub score: f32,
    pub zone: ZoneId,
    pub timestamp: Timestamp,
}

/// An anomaly report received from a neighboring node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NeighborAnomaly {
    pub source_zone: ZoneId,
    pub category: EventCategory,
    pub score: f32,
    pub timestamp: Timestamp,
    /// Approximate distance to reporting node in meters.
    pub distance_m: f32,
}

/// An event confirmed by cross-node spatial-temporal consensus.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfirmedEvent {
    pub event_id: u64,
    pub category: EventCategory,
    pub confidence: f32,
    pub affected_zone: ZoneId,
    pub corroborating_count: u8,
    pub recommended_action: RecommendedAction,
    pub timestamp: Timestamp,
}

// ---------------------------------------------------------------------------
// Mesh transport types
// ---------------------------------------------------------------------------

/// A single mesh protocol frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeshFrame {
    pub source: NodeId,
    pub payload: heapless::Vec<u8, MAX_FRAME_PAYLOAD>,
    pub ttl: u8,
}

/// A BLE beacon received during neighbor discovery.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BleBeacon {
    pub node_id: NodeId,
    pub zone_id: ZoneId,
    pub rssi: i8,
}

// ---------------------------------------------------------------------------
// Crypto types
// ---------------------------------------------------------------------------

/// CRYSTALS-Dilithium digital signature.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DilithiumSignature {
    pub bytes: heapless::Vec<u8, 3309>,
}

/// Public key (Dilithium or Kyber, depending on context).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PublicKey {
    pub bytes: heapless::Vec<u8, 1952>,
}

/// Shared secret produced by Kyber KEM.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SharedSecret {
    pub bytes: [u8; 32],
}

/// Kyber ciphertext.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ciphertext {
    pub bytes: heapless::Vec<u8, 1568>,
}

// ---------------------------------------------------------------------------
// Power types
// ---------------------------------------------------------------------------

/// Battery charge level as a fraction 0.0–1.0.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BatteryLevel(pub f32);

impl BatteryLevel {
    pub fn is_critical(&self) -> bool {
        self.0 < 0.1
    }

    pub fn is_low(&self) -> bool {
        self.0 < 0.25
    }
}

// ---------------------------------------------------------------------------
// Topology types
// ---------------------------------------------------------------------------

/// A proposed change to the mesh topology.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopologyChange {
    pub action: TopologyAction,
    pub affected_node: NodeId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TopologyAction {
    SuggestRelay { via: NodeId },
    DropLink { neighbor: NodeId },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_string_accepts_within_capacity() {
        let s = BoundedString::<16>::new("hello");
        assert!(s.is_some());
        assert_eq!(s.unwrap().as_str(), "hello");
    }

    #[test]
    fn bounded_string_rejects_over_capacity() {
        let long = "a]".repeat(20); // 40 bytes
        let s = BoundedString::<16>::new(&long);
        assert!(s.is_none());
    }

    #[test]
    fn bounded_string_empty() {
        let s = BoundedString::<16>::new("").unwrap();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn timestamp_ordering() {
        let t1 = Timestamp(100);
        let t2 = Timestamp(200);
        assert!(t1 < t2);
        assert!(t2 > t1);
        assert_eq!(t1, Timestamp(100));
    }

    #[test]
    fn timestamp_from_secs() {
        let t = Timestamp::from_secs(5);
        assert_eq!(t.as_millis(), 5000);
    }

    #[test]
    fn timestamp_saturating_diff() {
        let t1 = Timestamp(100);
        let t2 = Timestamp(300);
        assert_eq!(t2.saturating_diff(&t1), 200);
        assert_eq!(t1.saturating_diff(&t2), 0);
    }

    #[test]
    fn version_monotonicity() {
        let mut v = Version::new(0);
        let v0 = v;
        v.increment();
        assert!(v > v0);
        v.increment();
        assert_eq!(v, Version::new(2));
    }

    #[test]
    fn version_saturating_increment() {
        let mut v = Version::new(u64::MAX);
        v.increment();
        assert_eq!(v, Version::new(u64::MAX));
    }

    #[test]
    fn battery_level_thresholds() {
        assert!(BatteryLevel(0.05).is_critical());
        assert!(BatteryLevel(0.05).is_low());
        assert!(BatteryLevel(0.20).is_low());
        assert!(!BatteryLevel(0.20).is_critical());
        assert!(!BatteryLevel(0.50).is_low());
    }

    #[test]
    fn season_slot_clamped() {
        let slot = SeasonSlot::new(99);
        assert_eq!(slot.week, 51);
    }

    #[test]
    fn node_id_round_trip_postcard() {
        let id = NodeId([1, 2, 3, 4, 5, 6, 7, 8]);
        let bytes = postcard::to_allocvec(&id).unwrap();
        let decoded: NodeId = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(id, decoded);
    }

    #[test]
    fn zone_id_round_trip_postcard() {
        let id = ZoneId([0xDE, 0xAD, 0xBE, 0xEF]);
        let bytes = postcard::to_allocvec(&id).unwrap();
        let decoded: ZoneId = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(id, decoded);
    }

    #[test]
    fn sensor_reading_round_trip_postcard() {
        let reading = SensorReading {
            temperature: 2150,
            humidity: 6500,
            soil_moisture: 4200,
            pressure: 101325,
            pressure_delta: -50,
            light: 850,
        };
        let bytes = postcard::to_allocvec(&reading).unwrap();
        let decoded: SensorReading = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(reading, decoded);
    }

    #[test]
    fn event_category_round_trip_postcard() {
        let cat = EventCategory::Flood {
            severity: FloodSeverity::Emergency,
            upstream_origin: Some(ZoneId([1, 2, 3, 4])),
        };
        let bytes = postcard::to_allocvec(&cat).unwrap();
        let decoded: EventCategory = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(cat, decoded);
    }

    #[test]
    fn embedding_zero() {
        let e = Embedding::zero();
        assert!(e.data.iter().all(|&x| x == 0));
    }
}
