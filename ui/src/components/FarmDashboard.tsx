import { useMemo } from "react";
import type { SensorReading } from "../lib/types";

export interface FarmDashboardProps {
  readings: SensorReading[];
  zoneName?: string;
}

interface ChartPoint {
  x: number;
  y: number;
}

function LineChart({ data, label }: { data: ChartPoint[]; label: string }) {
  if (data.length === 0) return null;

  const W = 400;
  const H = 150;
  const PAD = 4;
  const minY = Math.min(...data.map((d) => d.y));
  const maxY = Math.max(...data.map((d) => d.y));
  const rangeY = maxY - minY || 1;

  const points = data
    .map((d, i) => {
      const x = PAD + (i / Math.max(data.length - 1, 1)) * (W - 2 * PAD);
      const y = H - PAD - ((d.y - minY) / rangeY) * (H - 2 * PAD);
      return `${x},${y}`;
    })
    .join(" ");

  return (
    <svg
      data-testid={`${label}-chart`}
      width={W}
      height={H}
      role="img"
      aria-label={`${label} chart`}
      style={{ display: "block", marginBottom: 8 }}
    >
      <polyline points={points} fill="none" stroke="currentColor" strokeWidth="2" />
    </svg>
  );
}

function toChartPoints(readings: SensorReading[], accessor: (r: SensorReading) => number): ChartPoint[] {
  return readings.map((r) => ({ x: r.timestamp, y: accessor(r) }));
}

function latestReading(readings: SensorReading[]): SensorReading | undefined {
  return readings.length > 0 ? readings[readings.length - 1] : undefined;
}

export function FarmDashboard({ readings, zoneName }: FarmDashboardProps) {
  const sorted = useMemo(
    () => [...readings].sort((a, b) => a.timestamp - b.timestamp),
    [readings],
  );

  const latest = latestReading(sorted);

  const tempData = useMemo(() => toChartPoints(sorted, (r) => r.temperature), [sorted]);
  const humidityData = useMemo(() => toChartPoints(sorted, (r) => r.humidity), [sorted]);
  const soilData = useMemo(() => toChartPoints(sorted, (r) => r.soil_moisture), [sorted]);
  const pressureData = useMemo(() => toChartPoints(sorted, (r) => r.pressure), [sorted]);

  // Simple sap flow heuristic: temperature rising above 0°C after a freeze
  const sapFlowFavorable =
    latest !== undefined &&
    latest.temperature > 0 &&
    sorted.length >= 2 &&
    sorted[sorted.length - 2]!.temperature <= 0;

  return (
    <section aria-label="Farm dashboard">
      <h2>{zoneName ?? "Farm"} — Sugarbush Monitor</h2>

      {latest && (
        <dl data-testid="latest-readings" style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: "4px 12px" }}>
          <dt>Temperature</dt><dd>{latest.temperature.toFixed(1)} °C</dd>
          <dt>Humidity</dt><dd>{latest.humidity.toFixed(0)}%</dd>
          <dt>Soil Moisture</dt><dd>{latest.soil_moisture.toFixed(0)}%</dd>
          <dt>Pressure</dt><dd>{latest.pressure.toFixed(0)} hPa</dd>
        </dl>
      )}

      <div data-testid="sap-flow-indicator" style={{ margin: "8px 0", fontWeight: "bold" }}>
        Sap Flow: {sapFlowFavorable ? "Favorable" : "Not favorable"}
      </div>

      <LineChart data={tempData} label="temperature" />
      <LineChart data={humidityData} label="humidity" />
      <LineChart data={soilData} label="soil-moisture" />
      <LineChart data={pressureData} label="pressure" />
    </section>
  );
}
