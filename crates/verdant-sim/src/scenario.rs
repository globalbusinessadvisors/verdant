use verdant_safla::events::FloodPreemptiveAlert;

/// An alert received by a zone during simulation.
#[derive(Clone, Debug)]
pub enum Alert {
    FloodPreemptive(FloodPreemptiveAlert),
    Confirmed(verdant_core::types::ConfirmedEvent),
}

/// Configuration for the simulation.
#[derive(Clone, Debug)]
pub struct SimConfig {
    pub node_count: usize,
    pub zone_count: usize,
    pub zone_layout: ZoneLayout,
    pub rf_max_range_m: f64,
    pub training_ticks: u64,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            node_count: 20,
            zone_count: 4,
            zone_layout: ZoneLayout::Grid { rows: 4, cols: 5 },
            rf_max_range_m: 500.0,
            training_ticks: 200,
        }
    }
}

/// How nodes are laid out spatially.
#[derive(Clone, Debug)]
pub enum ZoneLayout {
    /// Regular grid with given spacing.
    Grid { rows: usize, cols: usize },
}
