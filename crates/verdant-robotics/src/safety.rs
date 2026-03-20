use verdant_core::types::Timestamp;

use crate::mission::{AbortReason, MissionStateMachine, Position};

/// Weather conditions for flight safety.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeatherStatus {
    Safe,
    Marginal,
    Unsafe,
}

/// Battery status for flight safety.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatteryStatus {
    Sufficient,
    Low,
    Critical,
}

/// Checks environmental and hardware constraints for safe robot operation.
///
/// Concrete implementations read from weather sensors, geofence databases,
/// and battery monitors. Mocked for testing.
pub trait SafetyConstraintChecker {
    /// Returns `true` if the position is inside a no-fly zone.
    fn check_no_fly_zone(&self, position: &Position) -> bool;

    /// Current weather safety status.
    fn check_weather(&self) -> WeatherStatus;

    /// Current battery safety status.
    fn check_battery(&self) -> BatteryStatus;

    /// Returns `true` if the position is in an active wildlife corridor at the given time.
    fn check_wildlife_corridor(&self, position: &Position, time: Timestamp) -> bool;
}

/// Run a safety check cycle. If any constraint is violated, the mission
/// state machine is immediately aborted.
///
/// Returns `Some(reason)` if an abort was triggered, `None` if all safe.
pub fn run_safety_check(
    position: &Position,
    now: Timestamp,
    checker: &impl SafetyConstraintChecker,
    state_machine: &mut MissionStateMachine,
) -> Option<AbortReason> {
    if checker.check_no_fly_zone(position) {
        let reason = AbortReason::NoFlyZone;
        let _ = state_machine.safety_abort(reason);
        return Some(reason);
    }

    if checker.check_weather() == WeatherStatus::Unsafe {
        let reason = AbortReason::WeatherAbort;
        let _ = state_machine.safety_abort(reason);
        return Some(reason);
    }

    if checker.check_battery() == BatteryStatus::Critical {
        let reason = AbortReason::LowBattery;
        let _ = state_machine.safety_abort(reason);
        return Some(reason);
    }

    if checker.check_wildlife_corridor(position, now) {
        let reason = AbortReason::SafetyOverride;
        let _ = state_machine.safety_abort(reason);
        return Some(reason);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission::{Mission, MissionStateMachine, RobotState};
    use mockall::mock;
    use verdant_core::types::{BatteryLevel, MissionType};

    mock! {
        pub Safety {}
        impl SafetyConstraintChecker for Safety {
            fn check_no_fly_zone(&self, position: &Position) -> bool;
            fn check_weather(&self) -> WeatherStatus;
            fn check_battery(&self) -> BatteryStatus;
            fn check_wildlife_corridor(&self, position: &Position, time: Timestamp) -> bool;
        }
    }

    fn test_position() -> Position {
        Position { lat: 44.26, lon: -72.58, alt_m: 350.0 }
    }

    fn navigating_machine() -> MissionStateMachine {
        let mut sm = MissionStateMachine::new(BatteryLevel(0.80), 0.20);
        sm.assign(Mission {
            id: 1,
            mission_type: MissionType::SeedDispersal,
            target: test_position(),
            assigned_at: Timestamp::from_secs(100),
        }).unwrap();
        sm.begin_navigation().unwrap();
        sm
    }

    fn safe_mock() -> MockSafety {
        let mut mock = MockSafety::new();
        mock.expect_check_no_fly_zone().returning(|_| false);
        mock.expect_check_weather().returning(|| WeatherStatus::Safe);
        mock.expect_check_battery().returning(|| BatteryStatus::Sufficient);
        mock.expect_check_wildlife_corridor().returning(|_, _| false);
        mock
    }

    #[test]
    fn abort_on_no_fly_zone() {
        let mut sm = navigating_machine();
        let mut mock = MockSafety::new();
        mock.expect_check_no_fly_zone().returning(|_| true);
        // Other checks not reached due to early return

        let result = run_safety_check(
            &test_position(), Timestamp::from_secs(200), &mock, &mut sm,
        );
        assert_eq!(result, Some(AbortReason::NoFlyZone));
        assert_eq!(sm.state(), RobotState::SafetyAbort);
    }

    #[test]
    fn abort_on_unsafe_weather() {
        let mut sm = navigating_machine();
        let mut mock = MockSafety::new();
        mock.expect_check_no_fly_zone().returning(|_| false);
        mock.expect_check_weather().returning(|| WeatherStatus::Unsafe);

        let result = run_safety_check(
            &test_position(), Timestamp::from_secs(200), &mock, &mut sm,
        );
        assert_eq!(result, Some(AbortReason::WeatherAbort));
        assert_eq!(sm.state(), RobotState::SafetyAbort);
    }

    #[test]
    fn abort_on_critical_battery() {
        let mut sm = navigating_machine();
        let mut mock = MockSafety::new();
        mock.expect_check_no_fly_zone().returning(|_| false);
        mock.expect_check_weather().returning(|| WeatherStatus::Safe);
        mock.expect_check_battery().returning(|| BatteryStatus::Critical);

        let result = run_safety_check(
            &test_position(), Timestamp::from_secs(200), &mock, &mut sm,
        );
        assert_eq!(result, Some(AbortReason::LowBattery));
        assert_eq!(sm.state(), RobotState::SafetyAbort);
    }

    #[test]
    fn abort_on_wildlife_corridor() {
        let mut sm = navigating_machine();
        let mut mock = MockSafety::new();
        mock.expect_check_no_fly_zone().returning(|_| false);
        mock.expect_check_weather().returning(|| WeatherStatus::Safe);
        mock.expect_check_battery().returning(|| BatteryStatus::Sufficient);
        mock.expect_check_wildlife_corridor().returning(|_, _| true);

        let result = run_safety_check(
            &test_position(), Timestamp::from_secs(200), &mock, &mut sm,
        );
        assert_eq!(result, Some(AbortReason::SafetyOverride));
    }

    #[test]
    fn no_abort_when_all_safe() {
        let mut sm = navigating_machine();
        let mock = safe_mock();

        let result = run_safety_check(
            &test_position(), Timestamp::from_secs(200), &mock, &mut sm,
        );
        assert_eq!(result, None);
        assert_eq!(sm.state(), RobotState::Navigating);
    }

    #[test]
    fn abort_from_executing_state() {
        let mut sm = navigating_machine();
        sm.arrive_at_target().unwrap();
        assert_eq!(sm.state(), RobotState::Executing);

        let mut mock = MockSafety::new();
        mock.expect_check_no_fly_zone().returning(|_| true);

        let result = run_safety_check(
            &test_position(), Timestamp::from_secs(200), &mock, &mut sm,
        );
        assert_eq!(result, Some(AbortReason::NoFlyZone));
        assert_eq!(sm.state(), RobotState::SafetyAbort);
    }
}
