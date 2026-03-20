use verdant_core::config::{
    ANOMALY_THRESHOLD, CHECKPOINT_INTERVAL_SECS, CSI_SAMPLE_WINDOW_MS, CONSENSUS_QUORUM,
    PATTERN_PROPAGATION_TTL, TEMPORAL_WINDOW_SECS,
};
use verdant_core::error::{EmitError, SenseError};
use verdant_core::traits::{
    AnomalySource, ConfirmedEventSink, CsiCapture, EnvironmentalSensor, FlashStorage,
    MeshTransport, PowerMonitor, SeasonalClock,
};
use verdant_core::types::{
    Anomaly, ConfirmedEvent, MeshFrame, NeighborAnomaly, NodeId,
    Timestamp,
};
use verdant_safla::consensus::ConsensusEngine;
use verdant_safla::health::HealthAssessor;
use verdant_safla::propagation::PatternPropagator;
use verdant_vector::anomaly::classify_anomaly;
use verdant_vector::graph::VectorGraph;

/// Result of one firmware cycle.
pub struct CycleResult {
    /// Recommended sleep duration in milliseconds before next cycle.
    pub sleep_ms: u32,
    /// Whether an anomaly was broadcast this cycle.
    pub anomaly_broadcast: bool,
    /// Whether a checkpoint was written this cycle.
    pub checkpointed: bool,
}

/// The node firmware orchestrator — wires together all subsystems behind
/// trait boundaries.
///
/// Generic over all hardware traits so that tests can inject mocks while
/// production uses concrete ESP32 implementations.
pub struct NodeFirmware<C, S, T, F, P, K>
where
    C: CsiCapture,
    S: EnvironmentalSensor,
    T: MeshTransport,
    F: FlashStorage,
    P: PowerMonitor,
    K: SeasonalClock,
{
    // Hardware ports
    csi: C,
    sensor: S,
    transport: T,
    storage: F,
    power: P,
    clock: K,

    // Domain state
    pub graph: VectorGraph,
    self_id: NodeId,

    // SAFLA components
    consensus_engine: ConsensusEngine,
    _health_assessor: HealthAssessor,
    propagator: PatternPropagator,

    // Checkpoint tracking
    last_checkpoint_ms: u64,

    // Anomaly accumulation for consensus
    local_anomalies: heapless::Vec<Anomaly, 16>,
    neighbor_anomalies: heapless::Vec<NeighborAnomaly, 64>,
    confirmed_events: heapless::Vec<ConfirmedEvent, 8>,
}

impl<C, S, T, F, P, K> NodeFirmware<C, S, T, F, P, K>
where
    C: CsiCapture,
    S: EnvironmentalSensor,
    T: MeshTransport,
    F: FlashStorage,
    P: PowerMonitor,
    K: SeasonalClock,
{
    pub fn new(
        csi: C,
        sensor: S,
        transport: T,
        storage: F,
        power: P,
        clock: K,
        self_id: NodeId,
    ) -> Self {
        Self {
            csi,
            sensor,
            transport,
            storage,
            power,
            clock,
            graph: VectorGraph::new(2048),
            self_id,
            consensus_engine: ConsensusEngine::new(CONSENSUS_QUORUM, TEMPORAL_WINDOW_SECS, 500.0),
            _health_assessor: HealthAssessor::new(60, 3),
            propagator: PatternPropagator::new(self_id),
            last_checkpoint_ms: 0,
            local_anomalies: heapless::Vec::new(),
            neighbor_anomalies: heapless::Vec::new(),
            confirmed_events: heapless::Vec::new(),
        }
    }

    /// Execute one complete firmware cycle:
    /// Sense → Learn → Detect → Communicate → Heal → Power/Checkpoint
    pub fn run_cycle(&mut self, now_ms: u64) -> Result<CycleResult, SenseError> {
        let mut anomaly_broadcast = false;

        // --- Phase 1: Sense ---
        let csi_frame = self.csi.capture(CSI_SAMPLE_WINDOW_MS)?;
        let sensor_reading = self.sensor.read()?;

        // --- Phase 2: Learn ---
        // Fuse CSI + sensor into features, project to embedding, update graph
        let features =
            verdant_sense::csi::CsiFeatureExtractor::extract(&verdant_sense::hal::RawCsiBuffer {
                subcarriers: csi_frame.subcarriers.clone(),
                duration_ms: csi_frame.duration_ms,
            });

        // Build raw feature vector matching the fusion layout
        let raw = Self::build_raw_features(&features, &sensor_reading);
        let projector = SimpleProjector;
        let embedding = self.graph.encode_and_update(raw, &projector, &self.clock);

        // --- Phase 3: Detect ---
        let anomaly_score = self.graph.anomaly_score(&embedding, &self.clock);
        if anomaly_score > ANOMALY_THRESHOLD {
            let category = classify_anomaly(&embedding, anomaly_score, &sensor_reading);

            // Record locally
            let anomaly = Anomaly {
                category: category.clone(),
                score: anomaly_score,
                zone: verdant_core::types::ZoneId([0; 4]), // set during provisioning
                timestamp: Timestamp(now_ms),
            };
            let _ = self.local_anomalies.push(anomaly);

            // Broadcast to mesh
            let mut payload = heapless::Vec::new();
            let _ = payload.extend_from_slice(b"ANOM");
            let frame = MeshFrame {
                source: self.self_id,
                payload,
                ttl: PATTERN_PROPAGATION_TTL,
            };
            let _ = self.transport.broadcast(&frame, PATTERN_PROPAGATION_TTL);
            anomaly_broadcast = true;
        }

        // --- Phase 4: Communicate ---
        // Drain incoming messages
        while let Ok(Some(_frame)) = self.transport.receive() {
            // In production: parse frame, dispatch to routing/anomaly/governance
        }

        // --- Phase 5: Self-heal (SAFLA) ---
        // Consensus
        {
            let source = FirmwareAnomalySource {
                local: &self.local_anomalies,
                neighbor: &self.neighbor_anomalies,
            };
            let mut sink = FirmwareEventSink {
                events: &mut self.confirmed_events,
            };
            let _ = self.consensus_engine.evaluate(&source, &mut sink);
        }

        // Pattern propagation
        let _ = self.propagator.check_and_propagate(&self.graph, &mut self.transport);

        // --- Phase 6: Power management + Checkpoint ---
        let checkpointed = self.maybe_checkpoint(now_ms);
        let sleep_ms = self.power.sleep_duration();

        Ok(CycleResult {
            sleep_ms,
            anomaly_broadcast,
            checkpointed,
        })
    }

    /// Write vector graph to flash if the checkpoint interval has elapsed.
    fn maybe_checkpoint(&mut self, now_ms: u64) -> bool {
        let interval_ms = CHECKPOINT_INTERVAL_SECS * 1000;
        if now_ms.saturating_sub(self.last_checkpoint_ms) < interval_ms {
            return false;
        }

        let mut buf = [0u8; 4096];
        let version_bytes = self.graph.version().0.to_le_bytes();
        buf[..8].copy_from_slice(&version_bytes);
        // In production: serialize full graph to flash across multiple blocks
        let _ = self.storage.write_block(0x10_0000, &buf[..64]);
        self.last_checkpoint_ms = now_ms;
        true
    }

    /// Build a RawFeatures vector from CSI features and sensor reading.
    fn build_raw_features(
        csi: &verdant_sense::csi::CsiFeatures,
        sensor: &verdant_core::types::SensorReading,
    ) -> verdant_core::types::RawFeatures {
        let mut data = heapless::Vec::new();
        let _ = data.push(csi.amplitude_variance / 10000.0);
        let _ = data.push(csi.phase_std_dev / 1000.0);
        let _ = data.push(csi.temporal_autocorrelation);
        let _ = data.push(csi.dominant_frequency);
        let _ = data.push(csi.subcarrier_count as f32 / 64.0);
        let _ = data.push(sensor.temperature as f32 / 5000.0);
        let _ = data.push(sensor.humidity as f32 / 10000.0);
        let _ = data.push(sensor.soil_moisture as f32 / 10000.0);
        let _ = data.push((sensor.pressure as f32 - 101325.0) / 5000.0);
        let _ = data.push(sensor.light as f32 / 10000.0);
        while data.len() < verdant_core::config::RAW_FEATURE_DIM {
            let _ = data.push(0.0);
        }
        verdant_core::types::RawFeatures { data }
    }
}

// --- Simple embedding projector (identity-like transform for firmware) ---

struct SimpleProjector;

impl verdant_core::traits::EmbeddingProjector for SimpleProjector {
    fn project(
        &self,
        raw: &verdant_core::types::RawFeatures,
    ) -> verdant_core::types::Embedding {
        let mut data = [0i16; verdant_core::config::EMBEDDING_DIM];
        for (i, d) in data.iter_mut().enumerate() {
            let val = if i < raw.data.len() {
                raw.data[i] * 1000.0
            } else {
                0.0
            };
            *d = val.clamp(-32768.0, 32767.0) as i16;
        }
        verdant_core::types::Embedding { data }
    }
}

// --- Trait adapters for SAFLA within the firmware ---

struct FirmwareAnomalySource<'a> {
    local: &'a [Anomaly],
    neighbor: &'a [NeighborAnomaly],
}

impl AnomalySource for FirmwareAnomalySource<'_> {
    fn local_anomalies(&self) -> heapless::Vec<Anomaly, 16> {
        let mut v = heapless::Vec::new();
        for a in self.local.iter().take(16) {
            let _ = v.push(a.clone());
        }
        v
    }

    fn neighbor_anomalies(&self, _window_secs: u64) -> heapless::Vec<NeighborAnomaly, 64> {
        let mut v = heapless::Vec::new();
        for a in self.neighbor.iter().take(64) {
            let _ = v.push(a.clone());
        }
        v
    }
}

struct FirmwareEventSink<'a> {
    events: &'a mut heapless::Vec<ConfirmedEvent, 8>,
}

impl ConfirmedEventSink for FirmwareEventSink<'_> {
    fn emit_confirmed(&mut self, event: &ConfirmedEvent) -> Result<(), EmitError> {
        let _ = self.events.push(event.clone());
        Ok(())
    }

    fn request_robot_task(&mut self, _event: &ConfirmedEvent) -> Result<(), EmitError> {
        Ok(())
    }
}
