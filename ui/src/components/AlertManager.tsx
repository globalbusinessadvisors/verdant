import { useState } from "react";
import type { Alert, AlertRule, EventCategory, FloodSeverity, ZoneId } from "../lib/types";

export interface AlertManagerProps {
  alerts: Alert[];
  zones: ZoneId[];
  onConfigureRule: (rule: AlertRule) => void;
  onDismiss: (alertId: string) => void;
}

const CATEGORIES: EventCategory["type"][] = [
  "Pest", "Flood", "Fire", "Infrastructure", "Wildlife", "Climate",
];

const SEVERITIES: FloodSeverity[] = ["Watch", "Warning", "Emergency"];

function alertSummary(alert: Alert): string {
  if (alert.type === "FloodPreemptive") {
    return `Flood alert — ${alert.alert.severity} — Zone ${alert.alert.target_zone}`;
  }
  return `${alert.event.category.type} — Zone ${alert.event.affected_zone}`;
}

function alertId(alert: Alert): string {
  if (alert.type === "FloodPreemptive") {
    return `flood-${alert.alert.origin_event_id}`;
  }
  return `event-${alert.event.event_id}`;
}

export function AlertManager({ alerts, zones, onConfigureRule, onDismiss }: AlertManagerProps) {
  const [zoneId, setZoneId] = useState(zones[0] ?? "");
  const [category, setCategory] = useState<EventCategory["type"]>("Flood");
  const [severity, setSeverity] = useState<FloodSeverity>("Warning");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const rule: AlertRule = {
      id: `rule-${Date.now()}`,
      zone_id: zoneId,
      category_filter: category,
      min_severity: severity,
      enabled: true,
    };
    onConfigureRule(rule);
  };

  return (
    <section aria-label="Alert manager">
      <form onSubmit={handleSubmit} aria-label="Configure alert rule">
        <label>
          Zone
          <select value={zoneId} onChange={(e) => setZoneId(e.target.value)}>
            {zones.map((z) => (
              <option key={z} value={z}>{z}</option>
            ))}
          </select>
        </label>
        <label>
          Category
          <select value={category} onChange={(e) => setCategory(e.target.value as EventCategory["type"])}>
            {CATEGORIES.map((c) => (
              <option key={c} value={c}>{c}</option>
            ))}
          </select>
        </label>
        <label>
          Min Severity
          <select value={severity} onChange={(e) => setSeverity(e.target.value as FloodSeverity)}>
            {SEVERITIES.map((s) => (
              <option key={s} value={s}>{s}</option>
            ))}
          </select>
        </label>
        <button type="submit">Create Rule</button>
      </form>

      <ul aria-label="Active alerts" style={{ listStyle: "none", padding: 0 }}>
        {alerts.map((alert) => {
          const id = alertId(alert);
          return (
            <li key={id} data-testid="alert-item" style={{ padding: "8px 0", borderBottom: "1px solid #e2e8f0" }}>
              {alertSummary(alert)}
              <button onClick={() => onDismiss(id)} style={{ marginLeft: 8 }}>
                Dismiss
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
