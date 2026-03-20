use crate::mission::Position;

/// Navigation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavError {
    /// Magnetometer cannot establish a fix.
    NoFix,
    /// Navigation hardware fault.
    HardwareFault,
}

/// Command issued by the navigator to the flight controller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NavCommand {
    /// Fly toward the given heading in degrees (0–360).
    Heading(f32),
    /// Hold current position.
    Hover,
    /// Initiate landing sequence.
    Land,
}

/// Quality of the magnetic position fix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixQuality {
    Excellent,
    Good,
    Degraded,
    Lost,
}

/// Quantum magnetometer-based navigation for GPS-denied terrain.
///
/// Concrete implementations use pre-mapped magnetic signatures to determine
/// position. Mocked for testing.
pub trait MagneticNavigator {
    /// Get current estimated position from magnetic field matching.
    fn current_position(&self) -> Result<Position, NavError>;

    /// Compute a navigation command to reach the target position.
    fn navigate_to(&mut self, target: Position) -> Result<NavCommand, NavError>;

    /// Current quality of the magnetic position fix.
    fn magnetic_fix_quality(&self) -> FixQuality;
}
