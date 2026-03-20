import type { FloodPreemptiveAlert, FloodSeverity } from "../lib/types";

export interface FloodTrackerProps {
  alerts: FloodPreemptiveAlert[];
}

const SEVERITY_COLORS: Record<FloodSeverity, string> = {
  Watch: "#eab308",
  Warning: "#f97316",
  Emergency: "#dc2626",
};

const SEVERITY_LABELS: Record<FloodSeverity, string> = {
  Watch: "Monitor conditions",
  Warning: "Prepare for potential flooding",
  Emergency: "Evacuate low-lying areas immediately",
};

function formatArrival(secs: number): string {
  if (secs < 3600) return `${Math.round(secs / 60)} min`;
  return `${(secs / 3600).toFixed(1)} hr`;
}

export function FloodTracker({ alerts }: FloodTrackerProps) {
  const sorted = [...alerts].sort((a, b) => a.estimated_arrival_secs - b.estimated_arrival_secs);

  return (
    <section aria-label="Flood tracker">
      <h2>Flood Propagation Tracker</h2>
      {sorted.length === 0 && <p>No active flood alerts.</p>}
      <ul aria-label="Flood alerts" style={{ listStyle: "none", padding: 0 }}>
        {sorted.map((alert) => (
          <li
            key={`${alert.origin_event_id}-${alert.target_zone}`}
            data-testid="flood-zone"
            data-severity={alert.severity}
            style={{
              padding: 12,
              marginBottom: 8,
              borderRadius: 6,
              backgroundColor: SEVERITY_COLORS[alert.severity],
              color: "#fff",
            }}
          >
            <strong>Zone {alert.target_zone}</strong>
            <span style={{ marginLeft: 12 }}>
              ETA: {formatArrival(alert.estimated_arrival_secs)}
            </span>
            <div>Soil saturation: {(alert.soil_saturation * 100).toFixed(0)}%</div>
            <div data-testid="recommendation">{SEVERITY_LABELS[alert.severity]}</div>
          </li>
        ))}
      </ul>
    </section>
  );
}
