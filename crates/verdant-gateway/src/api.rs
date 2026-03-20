use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use verdant_core::types::{ProposalHash, Timestamp, ZoneId};

use crate::db::{
    EventStore, GovernanceStore, NodeStatusStore, Proposal, ProposalStatus, SignedVote, Vote,
};

/// Shared application state holding the store trait objects.
pub struct AppState<E: EventStore, N: NodeStatusStore, G: GovernanceStore> {
    pub event_store: E,
    pub node_store: N,
    pub gov_store: G,
}

/// Build the axum router with all API endpoints.
pub fn build_router<E, N, G>(state: AppState<E, N, G>) -> Router
where
    E: EventStore + 'static,
    N: NodeStatusStore + 'static,
    G: GovernanceStore + 'static,
{
    let shared = Arc::new(state);
    Router::new()
        .route("/api/events", get(get_events::<E, N, G>))
        .route("/api/events/zone/{zone_id}", get(get_events_by_zone::<E, N, G>))
        .route("/api/nodes", get(get_nodes::<E, N, G>))
        .route("/api/nodes/{node_id}", get(get_node::<E, N, G>))
        .route("/api/governance/proposals", get(get_proposals::<E, N, G>).post(post_proposal::<E, N, G>))
        .route("/api/governance/proposals/{id}/vote", post(post_vote::<E, N, G>))
        .with_state(shared)
}

// ---------------------------------------------------------------------------
// Query/request types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct EventsQuery {
    pub since: Option<u64>,
}

#[derive(Deserialize)]
pub struct ProposalDraft {
    pub proposer_zone: [u8; 4],
    pub title: String,
    pub action: String,
    pub quorum: f32,
    pub voting_deadline_secs: u64,
}

#[derive(Deserialize)]
pub struct VoteRequest {
    pub voter_zone: [u8; 4],
    pub vote: String, // "yes", "no", "abstain"
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn get_events<E: EventStore, N: NodeStatusStore, G: GovernanceStore>(
    State(state): State<Arc<AppState<E, N, G>>>,
    Query(query): Query<EventsQuery>,
) -> impl IntoResponse {
    let since = Timestamp(query.since.unwrap_or(0));
    match state.event_store.events_since(since) {
        Ok(events) => (StatusCode::OK, Json(serde_json::to_value(&events).unwrap())),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to read events"})),
        ),
    }
}

async fn get_events_by_zone<E: EventStore, N: NodeStatusStore, G: GovernanceStore>(
    State(state): State<Arc<AppState<E, N, G>>>,
    Path(zone_hex): Path<String>,
) -> impl IntoResponse {
    let zone_byte = u8::from_str_radix(&zone_hex, 16).unwrap_or(0);
    let zone = ZoneId([zone_byte, 0, 0, 0]);
    match state.event_store.events_in_zone(zone, 100) {
        Ok(events) => (StatusCode::OK, Json(serde_json::to_value(&events).unwrap())),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to read events"})),
        ),
    }
}

async fn get_nodes<E: EventStore, N: NodeStatusStore, G: GovernanceStore>(
    State(state): State<Arc<AppState<E, N, G>>>,
) -> impl IntoResponse {
    match state.node_store.all_statuses() {
        Ok(statuses) => (StatusCode::OK, Json(serde_json::to_value(&statuses).unwrap())),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to read node statuses"})),
        ),
    }
}

async fn get_node<E: EventStore, N: NodeStatusStore, G: GovernanceStore>(
    State(state): State<Arc<AppState<E, N, G>>>,
    Path(node_hex): Path<String>,
) -> impl IntoResponse {
    let id_byte = u8::from_str_radix(&node_hex, 16).unwrap_or(0);
    let node_id = verdant_core::types::NodeId([id_byte, 0, 0, 0, 0, 0, 0, 0]);
    match state.node_store.status_for(node_id) {
        Ok(Some(status)) => (StatusCode::OK, Json(serde_json::to_value(&status).unwrap())),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Node not found"})),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to read node status"})),
        ),
    }
}

async fn get_proposals<E: EventStore, N: NodeStatusStore, G: GovernanceStore>(
    State(state): State<Arc<AppState<E, N, G>>>,
) -> impl IntoResponse {
    match state.gov_store.active_proposals() {
        Ok(proposals) => (StatusCode::OK, Json(serde_json::to_value(&proposals).unwrap())),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to read proposals"})),
        ),
    }
}

async fn post_proposal<E: EventStore, N: NodeStatusStore, G: GovernanceStore>(
    State(state): State<Arc<AppState<E, N, G>>>,
    Json(draft): Json<ProposalDraft>,
) -> impl IntoResponse {
    let mut id_bytes = [0u8; 32];
    // Deterministic ID from title hash
    for (i, b) in draft.title.bytes().enumerate() {
        id_bytes[i % 32] ^= b;
    }

    let proposal = Proposal {
        id: ProposalHash(id_bytes),
        proposer_zone: ZoneId(draft.proposer_zone),
        title: draft.title,
        action: draft.action,
        quorum: draft.quorum,
        voting_deadline: Timestamp::from_secs(draft.voting_deadline_secs),
        status: ProposalStatus::Active,
    };

    match state.gov_store.store_proposal(&proposal) {
        Ok(()) => (StatusCode::CREATED, Json(serde_json::to_value(&proposal).unwrap())),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to store proposal"})),
        ),
    }
}

async fn post_vote<E: EventStore, N: NodeStatusStore, G: GovernanceStore>(
    State(state): State<Arc<AppState<E, N, G>>>,
    Path(id_hex): Path<String>,
    Json(req): Json<VoteRequest>,
) -> impl IntoResponse {
    let mut id_bytes = [0u8; 32];
    for (i, b) in id_hex.bytes().enumerate().take(32) {
        id_bytes[i] = b;
    }

    let vote = match req.vote.as_str() {
        "yes" => Vote::Yes,
        "no" => Vote::No,
        _ => Vote::Abstain,
    };

    let signed_vote = SignedVote {
        voter_zone: ZoneId(req.voter_zone),
        vote,
    };

    match state.gov_store.record_vote(&ProposalHash(id_bytes), &signed_vote) {
        Ok(()) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{MockEventStore, MockGovernanceStore, MockNodeStatusStore, NodeStatus};
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;
    use verdant_core::types::{
        ConfirmedEvent, EventCategory, NodeId, PestSpecies, RecommendedAction,
    };

    fn test_event() -> ConfirmedEvent {
        ConfirmedEvent {
            event_id: 42,
            category: EventCategory::Pest {
                species_hint: Some(PestSpecies::Unknown),
            },
            confidence: 0.90,
            affected_zone: ZoneId([1, 0, 0, 0]),
            corroborating_count: 3,
            recommended_action: RecommendedAction::AlertOnly,
            timestamp: Timestamp::from_secs(1000),
        }
    }

    fn test_node_status() -> NodeStatus {
        NodeStatus {
            node_id: NodeId([1; 8]),
            zone_id: ZoneId([1, 0, 0, 0]),
            last_seen: Timestamp::from_secs(500),
            battery_level: 0.85,
            graph_version: 42,
            neighbor_count: 5,
            uptime_secs: 7200,
        }
    }

    fn build_test_router(
        event_store: MockEventStore,
        node_store: MockNodeStatusStore,
        gov_store: MockGovernanceStore,
    ) -> Router {
        build_router(AppState {
            event_store,
            node_store,
            gov_store,
        })
    }

    #[tokio::test]
    async fn get_events_queries_store_with_since_param() {
        let mut event_store = MockEventStore::new();
        event_store
            .expect_events_since()
            .withf(|ts| ts.0 == 1000)
            .times(1)
            .returning(|_| Ok(vec![test_event()]));

        let app = build_test_router(event_store, MockNodeStatusStore::new(), MockGovernanceStore::new());

        let resp = app
            .oneshot(
                Request::get("/api/events?since=1000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_nodes_returns_all_statuses() {
        let mut node_store = MockNodeStatusStore::new();
        node_store
            .expect_all_statuses()
            .times(1)
            .returning(|| Ok(vec![(NodeId([1; 8]), test_node_status())]));

        let app = build_test_router(MockEventStore::new(), node_store, MockGovernanceStore::new());

        let resp = app
            .oneshot(
                Request::get("/api/nodes")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn post_proposal_stores_and_returns_201() {
        let mut gov_store = MockGovernanceStore::new();
        gov_store
            .expect_store_proposal()
            .times(1)
            .returning(|_| Ok(()));

        let app = build_test_router(MockEventStore::new(), MockNodeStatusStore::new(), gov_store);

        let body = serde_json::json!({
            "proposer_zone": [1, 0, 0, 0],
            "title": "Upgrade firmware to v2.1",
            "action": "FirmwareUpdate",
            "quorum": 0.5,
            "voting_deadline_secs": 86400
        });

        let resp = app
            .oneshot(
                Request::post("/api/governance/proposals")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn get_proposals_returns_list() {
        let mut gov_store = MockGovernanceStore::new();
        gov_store
            .expect_active_proposals()
            .times(1)
            .returning(|| {
                Ok(vec![Proposal {
                    id: ProposalHash([0; 32]),
                    proposer_zone: ZoneId([1, 0, 0, 0]),
                    title: "Test".into(),
                    action: "Test".into(),
                    quorum: 0.5,
                    voting_deadline: Timestamp::from_secs(86400),
                    status: ProposalStatus::Active,
                }])
            });

        let app = build_test_router(MockEventStore::new(), MockNodeStatusStore::new(), gov_store);

        let resp = app
            .oneshot(
                Request::get("/api/governance/proposals")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_node_returns_404_when_not_found() {
        let mut node_store = MockNodeStatusStore::new();
        node_store
            .expect_status_for()
            .returning(|_| Ok(None));

        let app = build_test_router(MockEventStore::new(), node_store, MockGovernanceStore::new());

        let resp = app
            .oneshot(
                Request::get("/api/nodes/ff")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
