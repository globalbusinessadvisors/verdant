import { useEffect, useRef } from "react";
import type { ConfirmedEvent, NodeId, NodeStatus, RobotStatus, Timestamp } from "../lib/types";

export interface MeshMapProps {
  nodes: NodeStatus[];
  events: ConfirmedEvent[];
  robots?: RobotStatus[];
  now?: Timestamp;
  onNodeClick?: (nodeId: NodeId) => void;
}

type NodeHealth = "active" | "suspect" | "dead";

const SUSPECT_THRESHOLD_MS = 5 * 60_000;
const DEAD_THRESHOLD_MS = 15 * 60_000;

const HEALTH_COLORS: Record<NodeHealth, string> = {
  active: "#16a34a",
  suspect: "#eab308",
  dead: "#dc2626",
};
const ROBOT_COLOR = "#2563eb";

function nodeHealth(node: NodeStatus, now: number): NodeHealth {
  const age = now - node.last_seen;
  if (age < SUSPECT_THRESHOLD_MS) return "active";
  if (age < DEAD_THRESHOLD_MS) return "suspect";
  return "dead";
}

export function MeshMap({ nodes, events, robots, now, onNodeClick }: MeshMapProps) {
  const mapRef = useRef<HTMLDivElement>(null);
  const currentNow = now ?? Date.now();
  const robotIds = new Set(robots?.map((r) => r.robot_id));

  useEffect(() => {
    const el = mapRef.current;
    if (!el) return;

    let map: import("leaflet").Map | undefined;

    void import("leaflet").then((L) => {
      if (!el.children.length) {
        map = L.map(el).setView([44.5, -72.5], 10);
        L.tileLayer("https://{s}.tile.opentopomap.org/{z}/{x}/{y}.png", {
          attribution: "OpenTopoMap",
          maxZoom: 17,
        }).addTo(map);
      }
    }).catch(() => {
      // Leaflet unavailable (e.g. tests) — silent fallback
    });

    return () => {
      map?.remove();
    };
  }, []);

  return (
    <section aria-label="Mesh map">
      <div ref={mapRef} data-testid="map-container" style={{ width: "100%", height: "400px" }} />
      <ul aria-label="Mesh nodes" style={{ listStyle: "none", padding: 0 }}>
        {nodes.map((node) => {
          const isRobot = robotIds.has(node.node_id);
          const health = nodeHealth(node, currentNow);
          const color = isRobot ? ROBOT_COLOR : HEALTH_COLORS[health];
          return (
            <li
              key={node.node_id}
              data-testid="node-marker"
              data-health={isRobot ? "robot" : health}
              style={{ display: "inline-block", margin: 4, padding: "4px 8px", borderRadius: 4, backgroundColor: color, color: "#fff", cursor: "pointer" }}
              onClick={() => onNodeClick?.(node.node_id)}
            >
              {node.node_id.slice(0, 8)} — {node.zone_id}
            </li>
          );
        })}
      </ul>
      {events.length > 0 && (
        <p data-testid="event-heatmap">{events.length} event(s) on map</p>
      )}
    </section>
  );
}
