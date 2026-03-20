import type { RobotStatus } from "../lib/types";

export interface RobotOverviewProps {
  robots: RobotStatus[];
}

const STATE_ICONS: Record<RobotStatus["state"], string> = {
  Idle: "Idle",
  Navigating: "Navigating",
  Executing: "Executing",
};

export function RobotOverview({ robots }: RobotOverviewProps) {
  return (
    <section aria-label="Robot overview">
      <h2>Robots</h2>
      {robots.length === 0 && <p>No robots registered.</p>}
      <ul aria-label="Robot list" style={{ listStyle: "none", padding: 0 }}>
        {robots.map((robot) => (
          <li
            key={robot.robot_id}
            data-testid="robot-item"
            style={{ padding: "12px 0", borderBottom: "1px solid #e2e8f0" }}
          >
            <strong>{robot.robot_id.slice(0, 8)}</strong>
            <span style={{ marginLeft: 8 }} data-testid="robot-state">
              {STATE_ICONS[robot.state]}
            </span>
            <span style={{ marginLeft: 8 }}>
              Zone: {robot.zone_id}
            </span>
            <div>
              Battery: {(robot.battery_level * 100).toFixed(0)}%
              <span style={{ marginLeft: 12 }}>
                Safety: {robot.safety_ok ? "OK" : "ALERT"}
              </span>
            </div>
            {robot.current_mission && (
              <div data-testid="robot-mission">
                Mission: {robot.current_mission.mission_type} → Zone {robot.current_mission.target_zone}
                ({(robot.current_mission.progress * 100).toFixed(0)}%)
              </div>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}
