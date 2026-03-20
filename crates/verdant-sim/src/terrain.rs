/// A 2D position in the simulation grid (meters from origin).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Position {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Euclidean distance to another position in meters.
    pub fn distance_to(&self, other: &Position) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// RF propagation model parameters.
#[derive(Clone, Debug)]
pub struct RfModel {
    /// Maximum radio range in meters.
    pub max_range_m: f64,
    /// PDR at zero distance (ideal link).
    pub pdr_at_zero: f32,
}

impl Default for RfModel {
    fn default() -> Self {
        Self {
            max_range_m: 500.0,
            pdr_at_zero: 0.98,
        }
    }
}

impl RfModel {
    /// Compute expected packet delivery ratio between two positions.
    ///
    /// Returns 0.0 if out of range, decays linearly otherwise.
    pub fn compute_pdr(&self, a: &Position, b: &Position) -> f32 {
        let dist = a.distance_to(b);
        if dist >= self.max_range_m {
            return 0.0;
        }
        let ratio = 1.0 - (dist / self.max_range_m) as f32;
        self.pdr_at_zero * ratio
    }

    /// Compute approximate RTT in ms from distance.
    pub fn compute_rtt_ms(&self, a: &Position, b: &Position) -> u32 {
        let dist = a.distance_to(b);
        // ~1ms per 100m + 5ms processing baseline
        5 + (dist / 100.0) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_zero() {
        let p = Position::new(0.0, 0.0);
        assert_eq!(p.distance_to(&p), 0.0);
    }

    #[test]
    fn distance_pythagorean() {
        let a = Position::new(0.0, 0.0);
        let b = Position::new(3.0, 4.0);
        assert!((a.distance_to(&b) - 5.0).abs() < 0.001);
    }

    #[test]
    fn pdr_at_zero_distance() {
        let rf = RfModel::default();
        let p = Position::new(100.0, 100.0);
        let pdr = rf.compute_pdr(&p, &p);
        assert!((pdr - 0.98).abs() < 0.01);
    }

    #[test]
    fn pdr_at_max_range() {
        let rf = RfModel::default();
        let a = Position::new(0.0, 0.0);
        let b = Position::new(rf.max_range_m, 0.0);
        assert_eq!(rf.compute_pdr(&a, &b), 0.0);
    }

    #[test]
    fn pdr_decays_with_distance() {
        let rf = RfModel::default();
        let a = Position::new(0.0, 0.0);
        let near = Position::new(100.0, 0.0);
        let far = Position::new(400.0, 0.0);
        assert!(rf.compute_pdr(&a, &near) > rf.compute_pdr(&a, &far));
    }
}
