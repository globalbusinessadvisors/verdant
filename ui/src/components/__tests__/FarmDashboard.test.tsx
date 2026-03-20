import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { FarmDashboard } from "../FarmDashboard";
import type { SensorReading } from "../../lib/types";

const readings: SensorReading[] = [
  { zone_id: "z1", temperature: -2, humidity: 80, soil_moisture: 60, pressure: 1013, timestamp: 1_000 },
  { zone_id: "z1", temperature: 3, humidity: 75, soil_moisture: 55, pressure: 1012, timestamp: 2_000 },
  { zone_id: "z1", temperature: 5, humidity: 70, soil_moisture: 50, pressure: 1011, timestamp: 3_000 },
];

describe("FarmDashboard", () => {
  it("shows temperature chart", () => {
    render(<FarmDashboard readings={readings} />);

    expect(screen.getByTestId("temperature-chart")).toBeInTheDocument();
    expect(screen.getByTestId("humidity-chart")).toBeInTheDocument();
    expect(screen.getByTestId("soil-moisture-chart")).toBeInTheDocument();
    expect(screen.getByTestId("pressure-chart")).toBeInTheDocument();
  });

  it("displays latest readings", () => {
    render(<FarmDashboard readings={readings} />);

    const dl = screen.getByTestId("latest-readings");
    expect(dl.textContent).toContain("5.0");
    expect(dl.textContent).toContain("70");
  });
});
