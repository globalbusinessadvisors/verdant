/// Wake cycle interval in milliseconds.
pub const DUTY_CYCLE_MS: u32 = 30_000;

/// Duration of WiFi CSI capture per cycle in milliseconds.
pub const CSI_SAMPLE_WINDOW_MS: u32 = 5_000;

/// Cosine distance threshold above which an observation is considered anomalous.
pub const ANOMALY_THRESHOLD: f32 = 0.85;

/// Maximum number of embedding nodes in the vector graph.
pub const MAX_GRAPH_NODES: usize = 2048;

/// Dimensionality of environment embedding vectors.
pub const EMBEDDING_DIM: usize = 32;

/// Maximum hop count for epidemic pattern delta broadcasts.
pub const PATTERN_PROPAGATION_TTL: u8 = 16;

/// Minimum number of corroborating neighbor anomalies for consensus.
pub const CONSENSUS_QUORUM: u8 = 3;

/// Temporal window (seconds) for anomaly consensus correlation.
pub const TEMPORAL_WINDOW_SECS: u64 = 300;

/// Maximum number of mesh neighbors tracked per node.
pub const MAX_NEIGHBORS: usize = 256;

/// Maximum number of DAG tips maintained.
pub const MAX_DAG_TIPS: usize = 8;

/// Interval (seconds) between vector graph flash checkpoints.
pub const CHECKPOINT_INTERVAL_SECS: u64 = 3600;

/// Number of WiFi CSI subcarriers captured.
pub const MAX_SUBCARRIERS: usize = 64;

/// Maximum size of a single mesh frame payload in bytes.
pub const MAX_FRAME_PAYLOAD: usize = 4096;

/// Number of raw feature dimensions (CSI features + sensor features).
pub const RAW_FEATURE_DIM: usize = 40;
