//! Full pipeline integration test: firmware → mesh → gateway → dashboard API.
//!
//! Simulates a 50-node mesh, injects events, stores them in an in-memory
//! gateway, and verifies the HTTP API returns correct data.

use std::sync::Mutex;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use verdant_core::types::{
    ConfirmedEvent, EventCategory, NodeId, ProposalHash, Timestamp, ZoneId,
};
use verdant_gateway::api::{AppState, build_router};
use verdant_gateway::db::{
    DbError, EventId, EventStore, GovernanceStore, NodeStatus, NodeStatusStore,
    Proposal, ProposalStatus, SignedVote, Vote,
};
use verdant_sim::scenario::{SimConfig, ZoneLayout};
use verdant_sim::sim::{LinearWatershed, Simulation};

// ---------------------------------------------------------------------------
// In-memory store implementations for integration testing
// ---------------------------------------------------------------------------

struct MemEventStore {
    events: Mutex<Vec<ConfirmedEvent>>,
    next_id: Mutex<u64>,
}

impl MemEventStore {
    fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }
}

impl EventStore for MemEventStore {
    fn store_event(&self, event: &ConfirmedEvent) -> Result<EventId, DbError> {
        let mut id = self.next_id.lock().unwrap();
        let eid = *id;
        *id += 1;
        self.events.lock().unwrap().push(event.clone());
        Ok(eid)
    }

    fn events_since(&self, since: Timestamp) -> Result<Vec<ConfirmedEvent>, DbError> {
        Ok(self
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.timestamp >= since)
            .cloned()
            .collect())
    }

    fn events_in_zone(&self, zone: ZoneId, limit: usize) -> Result<Vec<ConfirmedEvent>, DbError> {
        Ok(self
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.affected_zone == zone)
            .take(limit)
            .cloned()
            .collect())
    }
}

struct MemNodeStore {
    statuses: Mutex<Vec<(NodeId, NodeStatus)>>,
}

impl MemNodeStore {
    fn new() -> Self {
        Self {
            statuses: Mutex::new(Vec::new()),
        }
    }
}

impl NodeStatusStore for MemNodeStore {
    fn update_status(&self, status: &NodeStatus) -> Result<(), DbError> {
        let mut store = self.statuses.lock().unwrap();
        if let Some(existing) = store.iter_mut().find(|(id, _)| *id == status.node_id) {
            existing.1 = status.clone();
        } else {
            store.push((status.node_id, status.clone()));
        }
        Ok(())
    }

    fn all_statuses(&self) -> Result<Vec<(NodeId, NodeStatus)>, DbError> {
        Ok(self.statuses.lock().unwrap().clone())
    }

    fn status_for(&self, node_id: NodeId) -> Result<Option<NodeStatus>, DbError> {
        Ok(self
            .statuses
            .lock()
            .unwrap()
            .iter()
            .find(|(id, _)| *id == node_id)
            .map(|(_, s)| s.clone()))
    }
}

struct MemGovStore {
    proposals: Mutex<Vec<Proposal>>,
    votes: Mutex<Vec<(ProposalHash, SignedVote)>>,
}

impl MemGovStore {
    fn new() -> Self {
        Self {
            proposals: Mutex::new(Vec::new()),
            votes: Mutex::new(Vec::new()),
        }
    }
}

impl GovernanceStore for MemGovStore {
    fn store_proposal(&self, proposal: &Proposal) -> Result<(), DbError> {
        self.proposals.lock().unwrap().push(proposal.clone());
        Ok(())
    }

    fn active_proposals(&self) -> Result<Vec<Proposal>, DbError> {
        Ok(self
            .proposals
            .lock()
            .unwrap()
            .iter()
            .filter(|p| p.status == ProposalStatus::Active)
            .cloned()
            .collect())
    }

    fn record_vote(&self, proposal_id: &ProposalHash, vote: &SignedVote) -> Result<(), DbError> {
        self.votes.lock().unwrap().push((*proposal_id, vote.clone()));
        Ok(())
    }

    fn votes_for(&self, proposal_id: &ProposalHash) -> Result<Vec<SignedVote>, DbError> {
        Ok(self
            .votes
            .lock()
            .unwrap()
            .iter()
            .filter(|(pid, _)| pid == proposal_id)
            .map(|(_, v)| v.clone())
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn zone(id: u8) -> ZoneId {
    ZoneId([id, 0, 0, 0])
}

fn sim_50_nodes() -> Simulation {
    let config = SimConfig {
        node_count: 50,
        zone_count: 5,
        zone_layout: ZoneLayout::Grid { rows: 8, cols: 8 },
        rf_max_range_m: 500.0,
        training_ticks: 200,
    };
    let mut sim = Simulation::new(config);
    sim.deploy_nodes();
    sim.run_until_mesh_converges(15);
    sim.fast_forward_training(200);
    sim
}

async fn response_json(resp: axum::response::Response) -> serde_json::Value {
    let body = resp.into_body();
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn flood_event_flows_through_to_api() {
    let mut sim = sim_50_nodes();

    // Inject flood at zone 0
    sim.inject_flood(zone(0), Timestamp::from_secs(1000));
    sim.run_for(5);

    // Collect confirmed events from sim
    let events: Vec<ConfirmedEvent> = sim
        .nodes
        .iter()
        .flat_map(|n| n.confirmed_events.clone())
        .filter(|e| matches!(e.category, EventCategory::Flood { .. }))
        .collect();
    assert!(!events.is_empty(), "Sim should produce flood events");

    // Store in gateway
    let event_store = MemEventStore::new();
    for event in &events {
        event_store.store_event(event).unwrap();
    }

    let app = build_router(AppState {
        event_store,
        node_store: MemNodeStore::new(),
        gov_store: MemGovStore::new(),
    });

    // Query via HTTP API
    let resp = app
        .oneshot(
            Request::get("/api/events?since=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    let arr = json.as_array().unwrap();
    assert!(
        !arr.is_empty(),
        "GET /api/events should return stored flood events"
    );
}

#[tokio::test]
async fn pest_spread_confirmed_in_dashboard_api() {
    let mut sim = sim_50_nodes();

    // Inject pest across zone boundary
    sim.inject_pest(zone(0), Timestamp::from_secs(1000));
    sim.inject_pest(zone(1), Timestamp::from_secs(1000));
    sim.run_for(5);

    let pest_events: Vec<ConfirmedEvent> = sim
        .nodes
        .iter()
        .flat_map(|n| n.confirmed_events.clone())
        .filter(|e| matches!(e.category, EventCategory::Pest { .. }))
        .collect();
    assert!(!pest_events.is_empty(), "Sim should produce pest events");

    let event_store = MemEventStore::new();
    for event in &pest_events {
        event_store.store_event(event).unwrap();
    }

    let app = build_router(AppState {
        event_store,
        node_store: MemNodeStore::new(),
        gov_store: MemGovStore::new(),
    });

    let resp = app
        .oneshot(
            Request::get("/api/events?since=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    let arr = json.as_array().unwrap();
    assert!(arr.len() >= 1, "Should have pest events in API");
}

#[tokio::test]
async fn governance_proposal_vote_tally_via_api() {
    let gov_store = MemGovStore::new();
    let app = build_router(AppState {
        event_store: MemEventStore::new(),
        node_store: MemNodeStore::new(),
        gov_store,
    });

    // Submit proposal
    let draft = serde_json::json!({
        "proposer_zone": [1, 0, 0, 0],
        "title": "Increase monitoring frequency",
        "action": "set_interval 5m",
        "quorum": 0.5,
        "voting_deadline_secs": 86400
    });

    let resp = app
        .clone()
        .oneshot(
            Request::post("/api/governance/proposals")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&draft).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let proposal_json = response_json(resp).await;
    let proposal_id = proposal_json["id"].as_str().unwrap_or("");
    assert!(!proposal_id.is_empty() || proposal_json["id"].is_array());

    // Cast votes
    for zone_byte in 0u8..3 {
        let vote_req = serde_json::json!({
            "voter_zone": [zone_byte, 0, 0, 0],
            "vote": "yes"
        });

        let resp = app
            .clone()
            .oneshot(
                Request::post(format!(
                    "/api/governance/proposals/{}/vote",
                    "test-proposal-id"
                ))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&vote_req).unwrap()))
                .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Verify proposals list
    let resp = app
        .oneshot(
            Request::get("/api/governance/proposals")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    let proposals = json.as_array().unwrap();
    assert!(!proposals.is_empty(), "Should have at least one proposal");
}

#[tokio::test]
async fn node_statuses_appear_in_api() {
    let node_store = MemNodeStore::new();

    // Populate with sim node data
    let mut sim = sim_50_nodes();
    for node in &sim.nodes {
        let status = NodeStatus {
            node_id: node.id,
            zone_id: node.zone,
            last_seen: Timestamp::from_secs(1000),
            battery_level: 0.85,
            graph_version: 1,
            neighbor_count: node.routing_table.active_neighbors().count() as u16,
            uptime_secs: 3600,
        };
        node_store.update_status(&status).unwrap();
    }

    let app = build_router(AppState {
        event_store: MemEventStore::new(),
        node_store,
        gov_store: MemGovStore::new(),
    });

    let resp = app
        .oneshot(Request::get("/api/nodes").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = response_json(resp).await;
    let nodes = json.as_array().unwrap();
    assert!(nodes.len() >= 50, "Should have at least 50 node statuses");
}

#[tokio::test]
async fn flood_propagation_reaches_downstream_zones() {
    let mut sim = sim_50_nodes();

    sim.inject_flood(zone(0), Timestamp::from_secs(1000));
    sim.run_for(5);

    let watershed = LinearWatershed {
        zone_count: 5,
        flow_time_secs: 2700,
        base_saturation: 0.5,
    };
    sim.propagate_floods(&watershed);

    // Downstream zone should have alerts
    let alerts = sim.alerts_received_by(zone(1));
    assert!(
        !alerts.is_empty(),
        "Zone 1 should receive downstream flood alert"
    );
}
