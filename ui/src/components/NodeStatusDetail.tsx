import type { NodeStatus } from "../lib/types";

export interface NodeStatusDetailProps {
  node: NodeStatus;
}

function formatUptime(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return `${h}h ${m}m`;
}

export function NodeStatusDetail({ node }: NodeStatusDetailProps) {
  return (
    <section aria-label={`Node ${node.node_id}`} data-testid="node-detail">
      <h2>Node {node.node_id.slice(0, 8)}</h2>
      <dl style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: "4px 12px" }}>
        <dt>Node ID</dt><dd>{node.node_id}</dd>
        <dt>Zone</dt><dd>{node.zone_id}</dd>
        <dt>Last Seen</dt><dd>{new Date(node.last_seen).toLocaleString()}</dd>
        <dt>Battery</dt><dd>{(node.battery_level * 100).toFixed(0)}%</dd>
        <dt>Graph Version</dt><dd>{node.graph_version}</dd>
        <dt>Neighbors</dt><dd>{node.neighbor_count}</dd>
        <dt>Uptime</dt><dd>{formatUptime(node.uptime_secs)}</dd>
      </dl>
    </section>
  );
}
