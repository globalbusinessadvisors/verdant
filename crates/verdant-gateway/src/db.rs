use std::fmt;

use serde::{Deserialize, Serialize};
use verdant_core::types::{
    ConfirmedEvent, NodeId, ProposalHash, Timestamp, ZoneId,
};

/// Database errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbError {
    NotFound,
    WriteFailed,
    ReadFailed,
    SerializationFailed,
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for DbError {}

/// Event identifier (auto-incrementing).
pub type EventId = u64;

/// Status of a mesh node as reported to the gateway.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeStatus {
    pub node_id: NodeId,
    pub zone_id: ZoneId,
    pub last_seen: Timestamp,
    pub battery_level: f32,
    pub graph_version: u64,
    pub neighbor_count: u16,
    pub uptime_secs: u64,
}

/// A governance proposal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: ProposalHash,
    pub proposer_zone: ZoneId,
    pub title: String,
    pub action: String,
    pub quorum: f32,
    pub voting_deadline: Timestamp,
    pub status: ProposalStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    FailedQuorum,
}

/// A signed vote on a proposal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SignedVote {
    pub voter_zone: ZoneId,
    pub vote: Vote,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vote {
    Yes,
    No,
    Abstain,
}

// ---------------------------------------------------------------------------
// Store traits (mocked for testing, implemented with redb for production)
// ---------------------------------------------------------------------------

/// Persists and queries confirmed events.
#[cfg_attr(test, mockall::automock)]
pub trait EventStore: Send + Sync {
    fn store_event(&self, event: &ConfirmedEvent) -> Result<EventId, DbError>;
    fn events_since(&self, since: Timestamp) -> Result<Vec<ConfirmedEvent>, DbError>;
    fn events_in_zone(&self, zone: ZoneId, limit: usize) -> Result<Vec<ConfirmedEvent>, DbError>;
}

/// Persists and queries node status reports.
#[cfg_attr(test, mockall::automock)]
pub trait NodeStatusStore: Send + Sync {
    fn update_status(&self, status: &NodeStatus) -> Result<(), DbError>;
    fn all_statuses(&self) -> Result<Vec<(NodeId, NodeStatus)>, DbError>;
    fn status_for(&self, node_id: NodeId) -> Result<Option<NodeStatus>, DbError>;
}

/// Persists governance proposals and votes.
#[cfg_attr(test, mockall::automock)]
pub trait GovernanceStore: Send + Sync {
    fn store_proposal(&self, proposal: &Proposal) -> Result<(), DbError>;
    fn active_proposals(&self) -> Result<Vec<Proposal>, DbError>;
    fn record_vote(&self, proposal_id: &ProposalHash, vote: &SignedVote) -> Result<(), DbError>;
    fn votes_for(&self, proposal_id: &ProposalHash) -> Result<Vec<SignedVote>, DbError>;
}
