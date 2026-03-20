import { useMemo, useState } from "react";
import type { ConfirmedEvent, EventCategory } from "../lib/types";

export interface EventTimelineProps {
  events: ConfirmedEvent[];
}

const CATEGORY_TYPES: EventCategory["type"][] = [
  "Pest",
  "Flood",
  "Fire",
  "Infrastructure",
  "Wildlife",
  "Climate",
];

function formatTimestamp(ms: number): string {
  return new Date(ms).toLocaleString();
}

function eventSummary(event: ConfirmedEvent): string {
  const cat = event.category;
  switch (cat.type) {
    case "Flood":
      return `Flood (${cat.severity})`;
    case "Fire":
      return `Fire (smoke: ${(cat.smoke_density * 100).toFixed(0)}%)`;
    case "Pest":
      return `Pest${cat.species_hint ? ` — ${cat.species_hint}` : ""}`;
    case "Infrastructure":
      return `Infrastructure — ${cat.sub_type}`;
    case "Wildlife":
      return `Wildlife — ${cat.movement_type}`;
    case "Climate":
      return `Climate — ${cat.sub_type}`;
  }
}

export function EventTimeline({ events }: EventTimelineProps) {
  const [filter, setFilter] = useState<EventCategory["type"] | null>(null);

  const filtered = useMemo(
    () =>
      filter ? events.filter((e) => e.category.type === filter) : events,
    [events, filter],
  );

  const sorted = useMemo(
    () => [...filtered].sort((a, b) => b.timestamp - a.timestamp),
    [filtered],
  );

  return (
    <section aria-label="Event timeline">
      <nav aria-label="Category filters" style={{ display: "flex", gap: 8, marginBottom: 12, flexWrap: "wrap" }}>
        <button
          onClick={() => setFilter(null)}
          aria-pressed={filter === null}
          style={{ fontWeight: filter === null ? "bold" : "normal" }}
        >
          All
        </button>
        {CATEGORY_TYPES.map((cat) => (
          <button
            key={cat}
            onClick={() => setFilter(cat)}
            aria-pressed={filter === cat}
            style={{ fontWeight: filter === cat ? "bold" : "normal" }}
          >
            {cat}
          </button>
        ))}
      </nav>
      <ul aria-label="Events" style={{ listStyle: "none", padding: 0 }}>
        {sorted.map((event) => (
          <li key={event.event_id} data-testid="event-item" data-category={event.category.type} style={{ padding: "8px 0", borderBottom: "1px solid #e2e8f0" }}>
            <strong>{eventSummary(event)}</strong>
            <span style={{ marginLeft: 12, color: "#64748b" }}>
              Zone {event.affected_zone} — {formatTimestamp(event.timestamp)}
            </span>
            <span style={{ marginLeft: 8 }}>
              Confidence: {(event.confidence * 100).toFixed(0)}%
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}
