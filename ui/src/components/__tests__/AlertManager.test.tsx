import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import { AlertManager } from "../AlertManager";

describe("AlertManager", () => {
  it("creates rule", async () => {
    const onConfigureRule = vi.fn();
    const user = userEvent.setup();

    render(
      <AlertManager
        alerts={[]}
        zones={["z1", "z2"]}
        onConfigureRule={onConfigureRule}
        onDismiss={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Create Rule" }));

    expect(onConfigureRule).toHaveBeenCalledTimes(1);
    const rule = onConfigureRule.mock.calls[0]![0];
    expect(rule).toMatchObject({
      zone_id: "z1",
      category_filter: "Flood",
      min_severity: "Warning",
      enabled: true,
    });
  });
});
