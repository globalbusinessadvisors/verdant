import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { FloodTracker } from "../FloodTracker";
import type { FloodPreemptiveAlert } from "../../lib/types";

describe("FloodTracker", () => {
  it("shows severity colors", () => {
    const alerts: FloodPreemptiveAlert[] = [
      {
        origin_event_id: 1,
        target_zone: "z1",
        estimated_arrival_secs: 1800,
        severity: "Emergency",
        soil_saturation: 0.95,
        timestamp: 1_700_000_000_000,
      },
    ];

    render(<FloodTracker alerts={alerts} />);

    const zone = screen.getByTestId("flood-zone");
    expect(zone).toHaveAttribute("data-severity", "Emergency");
    // Emergency renders with red background
    expect(zone).toHaveStyle({ backgroundColor: "#dc2626" });
  });

  it("shows evacuation recommendation for Emergency", () => {
    const alerts: FloodPreemptiveAlert[] = [
      {
        origin_event_id: 1,
        target_zone: "z1",
        estimated_arrival_secs: 600,
        severity: "Emergency",
        soil_saturation: 0.99,
        timestamp: 1_700_000_000_000,
      },
    ];

    render(<FloodTracker alerts={alerts} />);

    expect(screen.getByTestId("recommendation").textContent).toContain("Evacuate");
  });
});
