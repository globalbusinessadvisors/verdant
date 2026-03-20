# ADR-006: Offline-First PWA Dashboard

**Status:** Accepted
**Date:** 2026-03-20
**Deciders:** Verdant Core Team
**SPARC Refs:** FR-G1, FR-G3, FR-G6, FR-G7, G6

## Context

The Verdant dashboard must be usable by Vermont farmers and rural residents, many of whom have unreliable or no internet connectivity. The dashboard connects to a local gateway (Raspberry Pi on the same LAN or WiFi), not to the internet. It must function fully when the gateway is temporarily unreachable (e.g., power outage at the farmhouse) and sync when connectivity returns.

Candidate approaches:
1. **Native mobile app** — best offline support but requires App Store distribution and platform-specific code
2. **Electron desktop app** — heavy; requires installation; not suited for Raspberry Pi
3. **Server-rendered HTML (HTMX)** — simple but zero offline capability
4. **Progressive Web App (PWA)** — installable from browser, offline-first via Service Worker, single codebase

## Decision

Build the dashboard as an **offline-first PWA** using React + TypeScript, with IndexedDB for local state and a Service Worker for background sync.

## Rationale

- **No App Store dependency.** Users navigate to the gateway's local IP, tap "Add to Home Screen," and the PWA installs itself. No Apple/Google gatekeeping.
- **Offline-first with IndexedDB.** The PWA caches mesh state, events, and alert configurations locally. When the gateway is unreachable, the user sees the last-known state — not a blank screen.
- **Service Worker for sync.** When the gateway comes back online, a background sync reconciles local state with the gateway. CRDTs (Conflict-free Replicated Data Types) handle merge conflicts from concurrent offline edits.
- **WASM bridge for crypto.** QuDAG message verification runs in-browser via a WASM-compiled Rust module (`wasm-bindgen`). Users can verify governance vote integrity without trusting the gateway.
- **Works on any device.** Farmers can use whatever device they have — phone, tablet, old laptop. PWAs run in any modern browser.

## Consequences

- **Positive:** Zero installation; works offline; single codebase; runs on any device; WASM crypto verification
- **Negative:** Limited access to device hardware (no Bluetooth/USB for direct node interaction); Safari PWA support has historical gaps
- **Mitigated by:** Direct node interaction goes through the gateway, not the dashboard; Safari PWA support has improved significantly since iOS 16.4

## London School TDD Approach

```typescript
// TypeScript tests use jest mocks (London School) for gateway API and IndexedDB.

// Ports (interfaces the UI depends on):
interface MeshDataPort {
  fetchNodeStatuses(): Promise<NodeStatus[]>;
  fetchEvents(since: number): Promise<ConfirmedEvent[]>;
  subscribeEvents(callback: (event: ConfirmedEvent) => void): Unsubscribe;
}

interface OfflineSyncPort {
  getLocalState<T>(key: string): Promise<T | null>;
  setLocalState<T>(key: string, value: T): Promise<void>;
  queueForSync(operation: SyncOperation): Promise<void>;
  processSyncQueue(): Promise<SyncResult>;
}

interface GovernancePort {
  submitProposal(proposal: ProposalDraft): Promise<ProposalHash>;
  castVote(proposalId: string, vote: Vote): Promise<void>;
  verifyVoteIntegrity(proposalId: string): Promise<VerificationResult>;
}

// Tests:
describe("useMeshData hook", () => {
  it("fetches from gateway and caches to offline store", async () => {
    const mockMeshData = mock<MeshDataPort>();
    const mockOfflineSync = mock<OfflineSyncPort>();

    // GIVEN: gateway returns node statuses
    when(mockMeshData.fetchNodeStatuses())
      .thenResolve([testNodeStatus("node-1"), testNodeStatus("node-2")]);

    // THEN: results are cached to offline store
    verify(mockOfflineSync.setLocalState("nodeStatuses", anything()))
      .once();

    const { result } = renderHook(() =>
      useMeshData(instance(mockMeshData), instance(mockOfflineSync))
    );

    await waitFor(() => {
      expect(result.current.nodes).toHaveLength(2);
    });
  });

  it("falls back to offline cache when gateway unreachable", async () => {
    const mockMeshData = mock<MeshDataPort>();
    const mockOfflineSync = mock<OfflineSyncPort>();

    // GIVEN: gateway is unreachable
    when(mockMeshData.fetchNodeStatuses())
      .thenReject(new Error("Network error"));

    // AND: offline cache has stale data
    when(mockOfflineSync.getLocalState("nodeStatuses"))
      .thenResolve([testNodeStatus("node-1")]);

    // THEN: gateway is NOT retried (no retry storm)
    verify(mockMeshData.fetchNodeStatuses()).once();

    const { result } = renderHook(() =>
      useMeshData(instance(mockMeshData), instance(mockOfflineSync))
    );

    await waitFor(() => {
      expect(result.current.nodes).toHaveLength(1);
      expect(result.current.isOffline).toBe(true);
    });
  });
});

describe("GovernancePortal", () => {
  it("calls verifyVoteIntegrity via WASM when user clicks verify", async () => {
    const mockGovernance = mock<GovernancePort>();

    when(mockGovernance.verifyVoteIntegrity("proposal-123"))
      .thenResolve({ valid: true, voteCount: 8, quorumMet: true });

    render(
      <GovernancePortal
        governance={instance(mockGovernance)}
        proposalId="proposal-123"
      />
    );

    await userEvent.click(screen.getByRole("button", { name: /verify/i }));

    // THEN: WASM verification was invoked
    verify(mockGovernance.verifyVoteIntegrity("proposal-123")).once();

    await waitFor(() => {
      expect(screen.getByText(/verified.*8 votes/i)).toBeInTheDocument();
    });
  });
});
```
