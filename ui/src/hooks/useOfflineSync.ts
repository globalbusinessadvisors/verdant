import { useCallback, useEffect, useRef, useState } from "react";
import type { OfflineSyncPort } from "../lib/ports";
import type { SyncOperation, SyncResult } from "../lib/types";

export interface UseOfflineSyncResult {
  isSyncing: boolean;
  lastResult: SyncResult | null;
  queueOperation: (op: SyncOperation) => Promise<void>;
  syncNow: () => Promise<void>;
}

export function useOfflineSync(
  offlinePort: OfflineSyncPort,
): UseOfflineSyncResult {
  const [isSyncing, setIsSyncing] = useState(false);
  const [lastResult, setLastResult] = useState<SyncResult | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const syncNow = useCallback(async () => {
    setIsSyncing(true);
    try {
      const result = await offlinePort.processSyncQueue();
      setLastResult(result);
    } finally {
      setIsSyncing(false);
    }
  }, [offlinePort]);

  const queueOperation = useCallback(
    async (op: SyncOperation) => {
      await offlinePort.queueForSync(op);
    },
    [offlinePort],
  );

  useEffect(() => {
    const onOnline = () => void syncNow();
    window.addEventListener("online", onOnline);

    intervalRef.current = setInterval(() => {
      if (navigator.onLine) void syncNow();
    }, 30_000);

    return () => {
      window.removeEventListener("online", onOnline);
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [syncNow]);

  return { isSyncing, lastResult, queueOperation, syncNow };
}
