# ADR-005: redb for Gateway Storage

**Status:** Accepted
**Date:** 2026-03-20
**Deciders:** Verdant Core Team
**SPARC Refs:** FR-G1, FR-G2, FR-G4, G10

## Context

Each Verdant gateway (Raspberry Pi) aggregates data from the mesh and serves it to the TypeScript PWA dashboard. It must persist node status, confirmed events, governance state, and alert configurations. The database must be ACID-compliant, handle power loss gracefully (Vermont ice storms cut power), and run efficiently on ARM with limited resources.

Candidate approaches:
1. **SQLite** — ubiquitous, well-tested, but C dependency complicates Rust builds
2. **sled** — pure Rust but maintenance status uncertain (alpha/beta for years)
3. **redb** — pure Rust, ACID, single-file, purpose-built for embedded use
4. **RocksDB** — overkill; large C++ dependency, designed for server workloads
5. **Plain files + fsync** — too fragile for ACID requirements

## Decision

Use **redb** as the gateway's embedded database.

## Rationale

- **Pure Rust, zero C dependencies.** Cross-compilation to ARM (Raspberry Pi) is trivial with `cargo build --target aarch64-unknown-linux-gnu`.
- **ACID with crash safety.** redb uses a copy-on-write B-tree with write-ahead logging. Power loss mid-transaction does not corrupt the database.
- **Single file.** The entire database is one file, simplifiable backup/restore for rural operators.
- **Typed tables.** redb's API is `Table<K, V>` with type-safe keys and values, which maps cleanly to our domain types via `serde` + `postcard`.
- **Low resource usage.** Memory-mapped I/O with configurable cache size; suitable for Raspberry Pi's 1-8 GB RAM.

## Consequences

- **Positive:** Crash-safe, pure Rust, simple deployment, type-safe API
- **Negative:** Smaller ecosystem than SQLite; no SQL query language (all queries are key-range scans); single-writer
- **Mitigated by:** Gateway workload is single-writer by nature (one mesh bridge thread writes, dashboard reads); range queries cover all dashboard use cases

## London School TDD Approach

```rust
// Gateway DB access is behind a trait so the API layer doesn't depend on redb directly.

pub trait EventStore {
    fn store_event(&mut self, event: &ConfirmedEvent) -> Result<EventId, DbError>;
    fn events_since(&self, since: Timestamp) -> Result<Vec<ConfirmedEvent>, DbError>;
    fn events_in_zone(&self, zone: ZoneId, limit: usize) -> Result<Vec<ConfirmedEvent>, DbError>;
}

pub trait NodeStatusStore {
    fn update_status(&mut self, node_id: NodeId, status: &NodeStatus) -> Result<(), DbError>;
    fn all_statuses(&self) -> Result<Vec<(NodeId, NodeStatus)>, DbError>;
    fn status_for(&self, node_id: NodeId) -> Result<Option<NodeStatus>, DbError>;
}

pub trait GovernanceStore {
    fn store_proposal(&mut self, proposal: &Proposal) -> Result<(), DbError>;
    fn active_proposals(&self) -> Result<Vec<Proposal>, DbError>;
    fn record_vote(&mut self, proposal_id: &ProposalHash, vote: &SignedVote) -> Result<(), DbError>;
}

// API handlers are tested with mock stores (London School):

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn get_events_endpoint_queries_store_with_since_param() {
        let mut mock_store = MockEventStore::new();

        // THEN: store is queried with the since timestamp from the request
        mock_store.expect_events_since()
            .with(eq(Timestamp::from_secs(1000)))
            .times(1)
            .returning(|_| Ok(vec![test_event()]));

        let app = build_router(mock_store, mock_node_store, mock_gov_store);
        let resp = app.oneshot(
            Request::get("/api/events?since=1000").body(Body::empty()).unwrap()
        ).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn post_proposal_stores_and_returns_201() {
        let mut mock_gov = MockGovernanceStore::new();

        mock_gov.expect_store_proposal()
            .withf(|p| p.title == "Upgrade firmware to v2.1")
            .times(1)
            .returning(|_| Ok(()));

        let app = build_router(mock_event_store, mock_node_store, mock_gov);
        let resp = app.oneshot(
            Request::post("/api/governance/proposals")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&test_proposal()).unwrap()))
                .unwrap()
        ).await.unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
    }
}
```
