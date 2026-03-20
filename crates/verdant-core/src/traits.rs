use crate::error::{
    CryptoError, EmitError, HealError, RadioError, SenseError, StorageError, TransportError,
};
use crate::types::{
    Anomaly, BatteryLevel, BleBeacon, Ciphertext, ConfirmedEvent, CsiFrame, DilithiumSignature,
    Embedding, MeshFrame, NeighborAnomaly, NodeId, PublicKey, RawFeatures, SeasonSlot,
    SensorReading, SharedSecret, TopologyChange,
};

// ---------------------------------------------------------------------------
// Sensor abstraction ports
// ---------------------------------------------------------------------------

/// Captures WiFi Channel State Information for RF-based activity sensing.
pub trait CsiCapture {
    fn capture(&mut self, duration_ms: u32) -> Result<CsiFrame, SenseError>;
}

/// Reads fused environmental sensor data.
pub trait EnvironmentalSensor {
    fn read(&mut self) -> Result<SensorReading, SenseError>;
}

// ---------------------------------------------------------------------------
// Mesh communication ports
// ---------------------------------------------------------------------------

/// Transport layer for sending and receiving mesh protocol frames.
pub trait MeshTransport {
    fn send(&mut self, frame: &MeshFrame) -> Result<(), TransportError>;
    fn receive(&mut self) -> Result<Option<MeshFrame>, TransportError>;
    fn broadcast(&mut self, frame: &MeshFrame, ttl: u8) -> Result<(), TransportError>;
}

/// Low-level radio hardware abstraction.
pub trait RadioHardware {
    fn transmit_frame(&mut self, raw: &[u8]) -> Result<(), RadioError>;
    fn receive_frame(&mut self, buf: &mut [u8]) -> Result<usize, RadioError>;
    fn scan_ble_beacons(&mut self) -> Result<heapless::Vec<BleBeacon, 32>, RadioError>;
}

// ---------------------------------------------------------------------------
// Post-quantum crypto ports
// ---------------------------------------------------------------------------

/// Post-quantum cryptographic operations (CRYSTALS-Dilithium / Kyber).
pub trait PostQuantumCrypto {
    fn sign(&self, data: &[u8]) -> Result<DilithiumSignature, CryptoError>;
    fn verify(
        &self,
        data: &[u8],
        sig: &DilithiumSignature,
        pk: &PublicKey,
    ) -> Result<bool, CryptoError>;
    fn encapsulate(&self, pk: &PublicKey) -> Result<(SharedSecret, Ciphertext), CryptoError>;
    fn decapsulate(&self, ct: &Ciphertext) -> Result<SharedSecret, CryptoError>;
}

// ---------------------------------------------------------------------------
// Storage ports
// ---------------------------------------------------------------------------

/// Block-level flash storage abstraction.
pub trait FlashStorage {
    fn read_block(&self, addr: u32, buf: &mut [u8]) -> Result<(), StorageError>;
    fn write_block(&mut self, addr: u32, data: &[u8]) -> Result<(), StorageError>;
}

// ---------------------------------------------------------------------------
// Power monitoring ports
// ---------------------------------------------------------------------------

/// Monitors battery and solar power state.
pub trait PowerMonitor {
    fn battery_level(&self) -> BatteryLevel;
    fn solar_output_mw(&self) -> u16;
    /// Returns the recommended sleep duration in milliseconds.
    fn sleep_duration(&self) -> u32;
}

// ---------------------------------------------------------------------------
// Intelligence ports
// ---------------------------------------------------------------------------

/// Projects raw sensor features into a fixed-dimension embedding.
pub trait EmbeddingProjector {
    fn project(&self, raw_features: &RawFeatures) -> Embedding;
}

/// Provides the current seasonal time slot for baseline bucketing.
pub trait SeasonalClock {
    fn current_slot(&self) -> SeasonSlot;
}

// ---------------------------------------------------------------------------
// SAFLA (self-aware feedback loop) ports
// ---------------------------------------------------------------------------

/// Source of local and neighbor anomaly reports.
pub trait AnomalySource {
    fn local_anomalies(&self) -> heapless::Vec<Anomaly, 16>;
    fn neighbor_anomalies(&self, window_secs: u64) -> heapless::Vec<NeighborAnomaly, 64>;
}

/// Sink for confirmed events and robot task requests.
pub trait ConfirmedEventSink {
    fn emit_confirmed(&mut self, event: &ConfirmedEvent) -> Result<(), EmitError>;
    fn request_robot_task(&mut self, event: &ConfirmedEvent) -> Result<(), EmitError>;
}

/// Network self-healing operations.
pub trait NetworkHealer {
    fn reroute_around(&mut self, dead: NodeId) -> Result<(), HealError>;
    fn propose_topology_change(&mut self, change: &TopologyChange) -> Result<(), HealError>;
}
