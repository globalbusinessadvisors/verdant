import type { OfflineSyncPort } from "../ports";
import type { SyncOperation, SyncResult } from "../types";

const DB_NAME = "verdant-offline";
const DB_VERSION = 1;
const STATE_STORE = "state";
const SYNC_QUEUE_STORE = "sync-queue";

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(STATE_STORE)) {
        db.createObjectStore(STATE_STORE);
      }
      if (!db.objectStoreNames.contains(SYNC_QUEUE_STORE)) {
        db.createObjectStore(SYNC_QUEUE_STORE, { keyPath: "id" });
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

function tx<T>(
  db: IDBDatabase,
  store: string,
  mode: IDBTransactionMode,
  op: (s: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  return new Promise((resolve, reject) => {
    const txn = db.transaction(store, mode);
    const req = op(txn.objectStore(store));
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

export class IndexedDbOfflineAdapter implements OfflineSyncPort {
  private dbPromise: Promise<IDBDatabase> | null = null;

  private getDb(): Promise<IDBDatabase> {
    if (!this.dbPromise) {
      this.dbPromise = openDb();
    }
    return this.dbPromise;
  }

  async getLocalState<T>(key: string): Promise<T | null> {
    const db = await this.getDb();
    const result = await tx(db, STATE_STORE, "readonly", (s) => s.get(key));
    return (result as T) ?? null;
  }

  async setLocalState<T>(key: string, value: T): Promise<void> {
    const db = await this.getDb();
    await tx(db, STATE_STORE, "readwrite", (s) => s.put(value, key));
  }

  async queueForSync(operation: SyncOperation): Promise<void> {
    const db = await this.getDb();
    await tx(db, SYNC_QUEUE_STORE, "readwrite", (s) => s.put(operation));
  }

  async processSyncQueue(): Promise<SyncResult> {
    const db = await this.getDb();
    const all = await tx<SyncOperation[]>(
      db,
      SYNC_QUEUE_STORE,
      "readonly",
      (s) => s.getAll() as IDBRequest<SyncOperation[]>,
    );

    let synced = 0;
    let failed = 0;

    for (const op of all) {
      try {
        const res = await fetch(op.endpoint, {
          method: op.method,
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(op.body),
        });
        if (res.ok) {
          await tx(db, SYNC_QUEUE_STORE, "readwrite", (s) => s.delete(op.id));
          synced++;
        } else {
          failed++;
        }
      } catch {
        failed++;
      }
    }

    const remaining = all.length - synced;
    return { synced, failed, remaining };
  }
}
