# ADR-010: Agentic Robotics Integration

**Status:** Accepted
**Date:** 2026-03-20
**Deciders:** Verdant Core Team
**SPARC Refs:** FR-R1 through FR-R5, G9

## Context

When the mesh confirms physical events (pest infestation, pipe freeze, post-storm damage), it needs a physical response capability. Vermont's GPS-denied mountain valleys, harsh weather, and dispersed terrain require autonomous robots that can navigate without satellite signals and coordinate as a swarm.

Candidate approaches:
1. **Manual human response only** — simplest but too slow for time-critical events (pipe freeze, post-storm supply delivery)
2. **GPS-guided commercial drones** — available off-shelf but fail in Vermont's mountain valleys
3. **Quantum magnetometer navigation + swarm coordination** — novel but enables GPS-denied autonomy
4. **Tethered/fixed response (e.g., automated valves, sirens)** — limited action vocabulary; high installation cost

## Decision

Integrate an **agentic robotics layer** using quantum magnetometer navigation, mesh-coordinated swarm tasking, and a mission state machine with hard safety constraints. Robots double as mobile mesh relay nodes.

## Rationale

- **Quantum magnetometer navigation** exploits Earth's magnetic field anomalies, which are locally unique and stable. By pre-mapping magnetic signatures (via initial survey flights), robots can navigate valleys where GPS is unreliable. This uses the same principle as Ruv's quantum-magnetic-navigation work.
- **Swarm coordination via mesh protocol.** Robots participate in the QuDAG mesh as mobile nodes. Task assignment, conflict resolution, and return-to-base are coordinated via mesh messages, not a central controller.
- **Mission state machine.** Each robot runs a deterministic state machine: `Idle → Tasked → Navigating → Executing → Returning → Idle`. Hard safety constraints (no-fly zones, weather abort, battery minimum) can interrupt any state.
- **Mobile relay nodes.** In transit, robots extend the mesh's coverage. During a partition event (flood, ice storm), a robot flying between disconnected zones can bridge the gap.
- **Phased integration.** Ground rovers (Phase 4) before aerial drones (Phase 5). This manages risk and regulatory complexity (FAA Part 107).

## Consequences

- **Positive:** Physical response to detected events; GPS-denied navigation; mesh extension via mobile relays
- **Negative:** High hardware cost per unit; regulatory complexity (FAA); safety-critical software requires formal verification
- **Mitigated by:** Phase 4/5 timeline (not launch blocker); safety constraints are conservative defaults; formal verification of safety module using `kani` proof harness

## London School TDD Approach

```rust
pub trait MagneticNavigator {
    fn current_position(&self) -> Result<Position, NavError>;
    fn navigate_to(&mut self, target: Position) -> Result<NavCommand, NavError>;
    fn magnetic_fix_quality(&self) -> FixQuality;
}

pub trait MissionAssigner {
    fn accept_mission(&mut self, mission: &Mission) -> Result<MissionId, MissionError>;
    fn report_status(&self, mission_id: MissionId) -> MissionStatus;
    fn abort(&mut self, mission_id: MissionId) -> Result<(), MissionError>;
}

pub trait SafetyConstraintChecker {
    fn check_no_fly_zone(&self, position: &Position) -> bool;
    fn check_weather(&self) -> WeatherStatus;
    fn check_battery(&self) -> BatteryStatus;
    fn check_wildlife_corridor(&self, position: &Position, time: Timestamp) -> bool;
}

pub trait SwarmCoordinator {
    fn request_task(&mut self, event: &ConfirmedEvent) -> Result<TaskAssignment, SwarmError>;
    fn resolve_conflict(&mut self, task_a: TaskId, task_b: TaskId) -> Resolution;
    fn report_completion(&mut self, task_id: TaskId, result: TaskResult) -> Result<(), SwarmError>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn mission_aborts_when_entering_no_fly_zone() {
        let mut mock_nav = MockMagneticNavigator::new();
        let mut mock_safety = MockSafetyConstraintChecker::new();
        let mut mock_assigner = MockMissionAssigner::new();

        // GIVEN: robot is navigating toward a target
        mock_nav.expect_current_position()
            .returning(|| Ok(Position::new(44.26, -72.58, 350.0)));

        // AND: current position is in a no-fly zone
        mock_safety.expect_check_no_fly_zone()
            .returning(|_| true);  // IN no-fly zone

        mock_safety.expect_check_weather()
            .returning(|| WeatherStatus::Safe);
        mock_safety.expect_check_battery()
            .returning(|| BatteryStatus::Sufficient);

        // THEN: mission is aborted
        mock_assigner.expect_abort()
            .with(eq(mission_id_1))
            .times(1)
            .returning(|_| Ok(()));

        let mut robot = RobotController::new(
            mock_nav, mock_assigner, mock_safety, mock_swarm
        );
        robot.set_active_mission(mission_id_1);
        robot.safety_check_cycle();
    }

    #[test]
    fn robot_acts_as_relay_node_while_navigating() {
        let mut mock_nav = MockMagneticNavigator::new();
        let mut mock_relay = MockMeshTransport::new();

        // GIVEN: robot is in transit
        mock_nav.expect_current_position()
            .returning(|| Ok(Position::new(44.30, -72.55, 500.0)));

        // AND: receives a mesh message not addressed to it
        let relay_msg = mesh_message_for(destination_zone_c);

        // THEN: message is forwarded (robot acts as relay)
        mock_relay.expect_send()
            .withf(|msg| msg.destination_zone == destination_zone_c)
            .times(1)
            .returning(|_| Ok(()));

        let mut robot = RobotController::new(
            mock_nav, mock_assigner, mock_safety, mock_swarm
        );
        robot.handle_mesh_message(relay_msg, &mut mock_relay);
    }

    #[test]
    fn swarm_resolves_conflict_by_proximity() {
        let mut mock_swarm = MockSwarmCoordinator::new();

        // GIVEN: two robots assigned overlapping tasks
        // THEN: coordinator resolves conflict
        mock_swarm.expect_resolve_conflict()
            .with(eq(task_a), eq(task_b))
            .times(1)
            .returning(|_, _| Resolution::AssignTo(robot_closer));

        let mut robot = RobotController::new(
            mock_nav, mock_assigner, mock_safety, mock_swarm
        );
        robot.handle_conflict(task_a, task_b);
    }

    #[test]
    fn mission_state_machine_transitions_correctly() {
        let mut mock_nav = MockMagneticNavigator::new();
        let mut mock_assigner = MockMissionAssigner::new();

        mock_assigner.expect_accept_mission()
            .returning(|_| Ok(mission_id_1));

        mock_nav.expect_navigate_to()
            .returning(|_| Ok(NavCommand::Heading(45.0)));

        mock_assigner.expect_report_status()
            .returning(|_| MissionStatus::Navigating);

        let mut robot = RobotController::new(
            mock_nav, mock_assigner, mock_safety, mock_swarm
        );

        assert_eq!(robot.state(), RobotState::Idle);

        robot.assign_mission(seed_dispersal_mission());
        assert_eq!(robot.state(), RobotState::Tasked);

        robot.begin_navigation();
        assert_eq!(robot.state(), RobotState::Navigating);
    }
}
```
