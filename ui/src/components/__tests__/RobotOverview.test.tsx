import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { RobotOverview } from "../RobotOverview";
import type { RobotStatus } from "../../lib/types";

describe("RobotOverview", () => {
  it("shows mission status", () => {
    const robots: RobotStatus[] = [
      {
        robot_id: "robot001robot001",
        zone_id: "z1",
        state: "Executing",
        battery_level: 0.72,
        current_mission: { mission_type: "SeedDispersal", target_zone: "z2", progress: 0.5 },
        last_seen: Date.now(),
        safety_ok: true,
      },
      {
        robot_id: "robot002robot002",
        zone_id: "z3",
        state: "Idle",
        battery_level: 0.95,
        last_seen: Date.now(),
        safety_ok: true,
      },
    ];

    render(<RobotOverview robots={robots} />);

    const items = screen.getAllByTestId("robot-item");
    expect(items).toHaveLength(2);

    const states = screen.getAllByTestId("robot-state");
    expect(states[0]!.textContent).toBe("Executing");
    expect(states[1]!.textContent).toBe("Idle");
  });
});
