import { renderHook, waitFor, act } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { useMeshData } from "./useMeshData";
import type { MeshDataPort, OfflineSyncPort } from "../lib/ports";
import type { ConfirmedEvent, NodeStatus } from "../lib/types";

function mockMeshPort(overrides: Partial<MeshDataPort> = {}): MeshDataPort {
  return {
    fetchNodeStatuses: vi.fn().mockResolvedValue([]),
    fetchEvents: vi.fn().mockResolvedValue([]),
    subscribeEvents: vi.fn().mockReturnValue(() => {}),
    ...overrides,
  };
}

function mockOfflinePort(
  overrides: Partial<OfflineSyncPort> = {},
): OfflineSyncPort {
  return {
    getLocalState: vi.fn().mockResolvedValue(null),
    setLocalState: vi.fn().mockResolvedValue(undefined),
    queueForSync: vi.fn().mockResolvedValue(undefined),
    processSyncQueue: vi
      .fn()
      .mockResolvedValue({ synced: 0, failed: 0, remaining: 0 }),
    ...overrides,
  };
}

const sampleNode: NodeStatus = {
  node_id: "aabbccdd00112233",
  zone_id: "aabb0011",
  last_seen: 1000,
  battery_level: 0.85,
  graph_version: 42,
  neighbor_count: 3,
  uptime_secs: 3600,
};

const sampleEvent: ConfirmedEvent = {
  event_id: 1,
  category: { type: "Fire", smoke_density: 0.7 },
  confidence: 0.95,
  affected_zone: "aabb0011",
  corroborating_count: 3,
  recommended_action: { type: "AlertAndMonitor" },
  timestamp: 1000,
};

describe("useMeshData", () => {
  it("fetches_from_gateway_and_caches", async () => {
    const mesh = mockMeshPort({
      fetchNodeStatuses: vi.fn().mockResolvedValue([sampleNode]),
      fetchEvents: vi.fn().mockResolvedValue([sampleEvent]),
    });
    const offline = mockOfflinePort();

    const { result } = renderHook(() => useMeshData(mesh, offline));

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.nodes).toEqual([sampleNode]);
    expect(result.current.events).toEqual([sampleEvent]);
    expect(result.current.isOffline).toBe(false);
    expect(offline.setLocalState).toHaveBeenCalledWith(
      "mesh:nodes",
      [sampleNode],
    );
    expect(offline.setLocalState).toHaveBeenCalledWith(
      "mesh:events",
      [sampleEvent],
    );
  });

  it("falls_back_to_cache_when_offline", async () => {
    const mesh = mockMeshPort({
      fetchNodeStatuses: vi.fn().mockRejectedValue(new Error("offline")),
      fetchEvents: vi.fn().mockRejectedValue(new Error("offline")),
    });
    const offline = mockOfflinePort({
      getLocalState: vi.fn().mockImplementation((key: string) => {
        if (key === "mesh:nodes") return Promise.resolve([sampleNode]);
        if (key === "mesh:events") return Promise.resolve([sampleEvent]);
        return Promise.resolve(null);
      }),
    });

    const { result } = renderHook(() => useMeshData(mesh, offline));

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.isOffline).toBe(true);
    expect(result.current.nodes).toEqual([sampleNode]);
    expect(result.current.events).toEqual([sampleEvent]);
  });

  it("subscribes_on_mount_unsubscribes_on_unmount", async () => {
    const unsubscribe = vi.fn();
    const mesh = mockMeshPort({
      subscribeEvents: vi.fn().mockReturnValue(unsubscribe),
    });
    const offline = mockOfflinePort();

    const { unmount } = renderHook(() => useMeshData(mesh, offline));

    await waitFor(() => {
      expect(mesh.subscribeEvents).toHaveBeenCalledTimes(1);
    });

    act(() => unmount());

    expect(unsubscribe).toHaveBeenCalledTimes(1);
  });
});
