import { useCallback, useEffect, useRef, useState } from "react";
import type { MeshDataPort, OfflineSyncPort } from "../lib/ports";
import type { ConfirmedEvent, NodeStatus } from "../lib/types";

const NODES_CACHE_KEY = "mesh:nodes";
const EVENTS_CACHE_KEY = "mesh:events";

export interface UseMeshDataResult {
  nodes: NodeStatus[];
  events: ConfirmedEvent[];
  isOffline: boolean;
  isLoading: boolean;
  refresh: () => Promise<void>;
}

export function useMeshData(
  meshPort: MeshDataPort,
  offlinePort: OfflineSyncPort,
): UseMeshDataResult {
  const [nodes, setNodes] = useState<NodeStatus[]>([]);
  const [events, setEvents] = useState<ConfirmedEvent[]>([]);
  const [isOffline, setIsOffline] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const unsubRef = useRef<(() => void) | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const [fetchedNodes, fetchedEvents] = await Promise.all([
        meshPort.fetchNodeStatuses(),
        meshPort.fetchEvents(0),
      ]);
      setNodes(fetchedNodes);
      setEvents(fetchedEvents);
      setIsOffline(false);

      await Promise.all([
        offlinePort.setLocalState(NODES_CACHE_KEY, fetchedNodes),
        offlinePort.setLocalState(EVENTS_CACHE_KEY, fetchedEvents),
      ]);
    } catch {
      setIsOffline(true);
      const [cachedNodes, cachedEvents] = await Promise.all([
        offlinePort.getLocalState<NodeStatus[]>(NODES_CACHE_KEY),
        offlinePort.getLocalState<ConfirmedEvent[]>(EVENTS_CACHE_KEY),
      ]);
      if (cachedNodes) setNodes(cachedNodes);
      if (cachedEvents) setEvents(cachedEvents);
    } finally {
      setIsLoading(false);
    }
  }, [meshPort, offlinePort]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    unsubRef.current = meshPort.subscribeEvents((event) => {
      setEvents((prev) => [...prev, event]);
    });
    return () => {
      unsubRef.current?.();
      unsubRef.current = null;
    };
  }, [meshPort]);

  return { nodes, events, isOffline, isLoading, refresh };
}
