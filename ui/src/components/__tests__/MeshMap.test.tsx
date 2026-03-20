import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { MeshMap } from "../MeshMap";
import type { NodeStatus } from "../../lib/types";

const NOW = 1_700_000_000_000;

function makeNode(id: string, zone: string, lastSeen: number): NodeStatus {
  return {
    node_id: id,
    zone_id: zone,
    last_seen: lastSeen,
    battery_level: 0.9,
    graph_version: 1,
    neighbor_count: 2,
    uptime_secs: 3600,
  };
}

describe("MeshMap", () => {
  it("renders nodes from useMeshData", () => {
    const nodes: NodeStatus[] = [
      makeNode("aaaa000000000001", "zone01", NOW - 1000),
      makeNode("aaaa000000000002", "zone02", NOW - 1000),
      makeNode("aaaa000000000003", "zone03", NOW - 1000),
    ];

    render(<MeshMap nodes={nodes} events={[]} now={NOW} />);

    const markers = screen.getAllByTestId("node-marker");
    expect(markers).toHaveLength(3);
  });

  it("colors nodes by health status", () => {
    const nodes: NodeStatus[] = [
      makeNode("active01active01", "z1", NOW - 1000),
      makeNode("suspect1suspect1", "z2", NOW - 6 * 60_000),
      makeNode("dead0001dead0001", "z3", NOW - 20 * 60_000),
    ];

    render(<MeshMap nodes={nodes} events={[]} now={NOW} />);

    const markers = screen.getAllByTestId("node-marker");
    expect(markers[0]).toHaveAttribute("data-health", "active");
    expect(markers[1]).toHaveAttribute("data-health", "suspect");
    expect(markers[2]).toHaveAttribute("data-health", "dead");
  });
});
