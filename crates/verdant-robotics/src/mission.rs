use serde::{Deserialize, Serialize};
use verdant_core::types::{BatteryLevel, MissionType, Timestamp};

/// Mission state machine errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissionError {
    /// Cannot accept a new mission while one is active.
    AlreadyBusy,
    /// Battery below minimum threshold for mission acceptance.
    InsufficientBattery,
    /// Invalid state transition attempted.
    InvalidTransition {
        from: RobotState,
        action: &'static str,
    },
}

/// Robot state in the mission lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RobotState {
    Idle,
    Tasked,
    Navigating,
    Executing,
    Returning,
    SafetyAbort,
}

/// A mission assigned to a robot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mission {
    pub id: u64,
    pub mission_type: MissionType,
    pub target: Position,
    pub assigned_at: Timestamp,
}

/// Simple 2D + altitude position.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub lat: f64,
    pub lon: f64,
    pub alt_m: f32,
}

/// Result of a completed mission execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionResult {
    Success,
    PartialSuccess,
    Failed,
}

/// Reason for a safety abort.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbortReason {
    NoFlyZone,
    WeatherAbort,
    LowBattery,
    NavigationFailure,
    SafetyOverride,
}

/// Deterministic state machine governing the robot mission lifecycle.
///
/// Invariants:
/// - At most one active mission at a time.
/// - Battery must exceed `min_battery_for_mission` to accept a mission.
/// - `safety_abort()` can be called from **any** state (overrides everything).
pub struct MissionStateMachine {
    state: RobotState,
    active_mission: Option<Mission>,
    battery: BatteryLevel,
    min_battery_for_mission: f32,
    abort_reason: Option<AbortReason>,
}

impl MissionStateMachine {
    pub fn new(battery: BatteryLevel, min_battery_for_mission: f32) -> Self {
        Self {
            state: RobotState::Idle,
            active_mission: None,
            battery,
            min_battery_for_mission,
            abort_reason: None,
        }
    }

    pub fn state(&self) -> RobotState {
        self.state
    }

    pub fn active_mission(&self) -> Option<&Mission> {
        self.active_mission.as_ref()
    }

    pub fn abort_reason(&self) -> Option<AbortReason> {
        self.abort_reason
    }

    /// Update battery level (called each cycle).
    pub fn update_battery(&mut self, level: BatteryLevel) {
        self.battery = level;
    }

    /// Accept a new mission. Only valid from `Idle`.
    pub fn assign(&mut self, mission: Mission) -> Result<(), MissionError> {
        if self.state != RobotState::Idle {
            return Err(MissionError::AlreadyBusy);
        }
        if self.battery.0 < self.min_battery_for_mission {
            return Err(MissionError::InsufficientBattery);
        }
        self.active_mission = Some(mission);
        self.state = RobotState::Tasked;
        self.abort_reason = None;
        Ok(())
    }

    /// Begin navigating to the mission target. Only valid from `Tasked`.
    pub fn begin_navigation(&mut self) -> Result<(), MissionError> {
        self.require_state(RobotState::Tasked, "begin_navigation")?;
        self.state = RobotState::Navigating;
        Ok(())
    }

    /// Arrived at the mission target. Only valid from `Navigating`.
    pub fn arrive_at_target(&mut self) -> Result<(), MissionError> {
        self.require_state(RobotState::Navigating, "arrive_at_target")?;
        self.state = RobotState::Executing;
        Ok(())
    }

    /// Mission execution complete. Only valid from `Executing`.
    pub fn complete_execution(&mut self, _result: MissionResult) -> Result<(), MissionError> {
        self.require_state(RobotState::Executing, "complete_execution")?;
        self.state = RobotState::Returning;
        Ok(())
    }

    /// Begin returning to base. Only valid from `Executing` (early return) or already `Returning`.
    pub fn begin_return(&mut self) -> Result<(), MissionError> {
        match self.state {
            RobotState::Executing | RobotState::Returning => {
                self.state = RobotState::Returning;
                Ok(())
            }
            other => Err(MissionError::InvalidTransition {
                from: other,
                action: "begin_return",
            }),
        }
    }

    /// Arrived back at base. Only valid from `Returning`.
    pub fn arrive_at_base(&mut self) -> Result<(), MissionError> {
        self.require_state(RobotState::Returning, "arrive_at_base")?;
        self.active_mission = None;
        self.state = RobotState::Idle;
        Ok(())
    }

    /// Immediate safety abort from **any** state.
    pub fn safety_abort(&mut self, reason: AbortReason) -> Result<(), MissionError> {
        self.abort_reason = Some(reason);
        self.state = RobotState::SafetyAbort;
        Ok(())
    }

    /// Reset from `SafetyAbort` back to `Idle` after the situation is resolved.
    pub fn clear_abort(&mut self) -> Result<(), MissionError> {
        self.require_state(RobotState::SafetyAbort, "clear_abort")?;
        self.active_mission = None;
        self.state = RobotState::Idle;
        self.abort_reason = None;
        Ok(())
    }

    fn require_state(&self, expected: RobotState, action: &'static str) -> Result<(), MissionError> {
        if self.state != expected {
            return Err(MissionError::InvalidTransition {
                from: self.state,
                action,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use verdant_core::types::MissionType;

    fn test_mission() -> Mission {
        Mission {
            id: 1,
            mission_type: MissionType::SeedDispersal,
            target: Position { lat: 44.26, lon: -72.58, alt_m: 350.0 },
            assigned_at: Timestamp::from_secs(1000),
        }
    }

    fn idle_machine() -> MissionStateMachine {
        MissionStateMachine::new(BatteryLevel(0.80), 0.20)
    }

    #[test]
    fn state_machine_idle_to_tasked_to_navigating() {
        let mut sm = idle_machine();
        assert_eq!(sm.state(), RobotState::Idle);

        sm.assign(test_mission()).unwrap();
        assert_eq!(sm.state(), RobotState::Tasked);

        sm.begin_navigation().unwrap();
        assert_eq!(sm.state(), RobotState::Navigating);
    }

    #[test]
    fn full_lifecycle() {
        let mut sm = idle_machine();
        sm.assign(test_mission()).unwrap();
        sm.begin_navigation().unwrap();
        sm.arrive_at_target().unwrap();
        assert_eq!(sm.state(), RobotState::Executing);

        sm.complete_execution(MissionResult::Success).unwrap();
        assert_eq!(sm.state(), RobotState::Returning);

        sm.arrive_at_base().unwrap();
        assert_eq!(sm.state(), RobotState::Idle);
        assert!(sm.active_mission().is_none());
    }

    #[test]
    fn assign_rejects_when_already_tasked() {
        let mut sm = idle_machine();
        sm.assign(test_mission()).unwrap();
        let result = sm.assign(test_mission());
        assert_eq!(result, Err(MissionError::AlreadyBusy));
    }

    #[test]
    fn assign_rejects_low_battery() {
        let mut sm = MissionStateMachine::new(BatteryLevel(0.05), 0.20);
        let result = sm.assign(test_mission());
        assert_eq!(result, Err(MissionError::InsufficientBattery));
    }

    #[test]
    fn safety_abort_from_idle() {
        let mut sm = idle_machine();
        sm.safety_abort(AbortReason::WeatherAbort).unwrap();
        assert_eq!(sm.state(), RobotState::SafetyAbort);
        assert_eq!(sm.abort_reason(), Some(AbortReason::WeatherAbort));
    }

    #[test]
    fn safety_abort_from_navigating() {
        let mut sm = idle_machine();
        sm.assign(test_mission()).unwrap();
        sm.begin_navigation().unwrap();
        sm.safety_abort(AbortReason::NoFlyZone).unwrap();
        assert_eq!(sm.state(), RobotState::SafetyAbort);
    }

    #[test]
    fn safety_abort_from_executing() {
        let mut sm = idle_machine();
        sm.assign(test_mission()).unwrap();
        sm.begin_navigation().unwrap();
        sm.arrive_at_target().unwrap();
        sm.safety_abort(AbortReason::LowBattery).unwrap();
        assert_eq!(sm.state(), RobotState::SafetyAbort);
    }

    #[test]
    fn safety_abort_from_returning() {
        let mut sm = idle_machine();
        sm.assign(test_mission()).unwrap();
        sm.begin_navigation().unwrap();
        sm.arrive_at_target().unwrap();
        sm.complete_execution(MissionResult::Success).unwrap();
        sm.safety_abort(AbortReason::NavigationFailure).unwrap();
        assert_eq!(sm.state(), RobotState::SafetyAbort);
    }

    #[test]
    fn clear_abort_returns_to_idle() {
        let mut sm = idle_machine();
        sm.safety_abort(AbortReason::WeatherAbort).unwrap();
        sm.clear_abort().unwrap();
        assert_eq!(sm.state(), RobotState::Idle);
        assert!(sm.abort_reason().is_none());
    }

    #[test]
    fn invalid_transition_begin_navigation_from_idle() {
        let mut sm = idle_machine();
        let result = sm.begin_navigation();
        assert!(matches!(result, Err(MissionError::InvalidTransition { .. })));
    }

    #[test]
    fn invalid_transition_arrive_at_target_from_tasked() {
        let mut sm = idle_machine();
        sm.assign(test_mission()).unwrap();
        let result = sm.arrive_at_target();
        assert!(matches!(result, Err(MissionError::InvalidTransition { .. })));
    }

    #[test]
    fn update_battery_affects_assignment() {
        let mut sm = idle_machine();
        sm.update_battery(BatteryLevel(0.05));
        let result = sm.assign(test_mission());
        assert_eq!(result, Err(MissionError::InsufficientBattery));
    }
}
