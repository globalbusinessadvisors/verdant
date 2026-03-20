import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect } from "vitest";
import { EventTimeline } from "../EventTimeline";
import type { ConfirmedEvent } from "../../lib/types";

const flood: ConfirmedEvent = {
  event_id: 1,
  category: { type: "Flood", severity: "Warning" },
  confidence: 0.9,
  affected_zone: "z1",
  corroborating_count: 3,
  recommended_action: { type: "AlertOnly" },
  timestamp: 1_700_000_000_000,
};

const fire: ConfirmedEvent = {
  event_id: 2,
  category: { type: "Fire", smoke_density: 0.5 },
  confidence: 0.8,
  affected_zone: "z2",
  corroborating_count: 2,
  recommended_action: { type: "AlertAndMonitor" },
  timestamp: 1_700_000_001_000,
};

const pest: ConfirmedEvent = {
  event_id: 3,
  category: { type: "Pest", species_hint: "EmeraldAshBorer" },
  confidence: 0.7,
  affected_zone: "z1",
  corroborating_count: 1,
  recommended_action: { type: "AlertOnly" },
  timestamp: 1_700_000_002_000,
};

describe("EventTimeline", () => {
  it("filters by category", async () => {
    const user = userEvent.setup();
    render(<EventTimeline events={[flood, fire, pest]} />);

    // All three shown initially
    expect(screen.getAllByTestId("event-item")).toHaveLength(3);

    // Click Flood filter
    await user.click(screen.getByRole("button", { name: "Flood" }));

    const items = screen.getAllByTestId("event-item");
    expect(items).toHaveLength(1);
    expect(items[0]).toHaveAttribute("data-category", "Flood");
  });
});
