use verdant_core::types::{ProposalHash, Timestamp, ZoneId};

use crate::db::{
    DbError, GovernanceStore, Proposal, SignedVote, Vote,
};

/// Governance tally result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TallyResult {
    Pending,
    Passed,
    Rejected,
    FailedQuorum,
}

/// Governance errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernanceError {
    UnknownZone,
    StoreError(DbError),
    ProposalNotFound,
}

/// Governance engine — tallies votes on proposals per ADR-004 rules.
pub struct GovernanceEngine {
    /// Registered zone IDs eligible to vote.
    registered_zones: Vec<ZoneId>,
}

impl GovernanceEngine {
    pub fn new(registered_zones: Vec<ZoneId>) -> Self {
        Self { registered_zones }
    }

    /// Total number of registered zones.
    pub fn zone_count(&self) -> usize {
        self.registered_zones.len()
    }

    /// Submit a new proposal (stores it via the store trait).
    pub fn submit_proposal(
        &self,
        proposal: &Proposal,
        store: &impl GovernanceStore,
    ) -> Result<(), GovernanceError> {
        store
            .store_proposal(proposal)
            .map_err(GovernanceError::StoreError)
    }

    /// Cast a vote on a proposal. Rejects votes from unregistered zones.
    pub fn cast_vote(
        &self,
        proposal_id: &ProposalHash,
        vote: &SignedVote,
        store: &impl GovernanceStore,
    ) -> Result<(), GovernanceError> {
        if !self.registered_zones.contains(&vote.voter_zone) {
            return Err(GovernanceError::UnknownZone);
        }
        store
            .record_vote(proposal_id, vote)
            .map_err(GovernanceError::StoreError)
    }

    /// Tally votes for a proposal.
    ///
    /// - If `now` < deadline → `Pending`
    /// - If votes cast < zones * quorum → `FailedQuorum`
    /// - If yes > votes_cast / 2 → `Passed`
    /// - Else → `Rejected`
    pub fn tally(
        &self,
        proposal: &Proposal,
        votes: &[SignedVote],
        now: Timestamp,
    ) -> TallyResult {
        if now < proposal.voting_deadline {
            return TallyResult::Pending;
        }

        let total_zones = self.registered_zones.len() as f32;
        let votes_cast = votes.len() as f32;

        if votes_cast < total_zones * proposal.quorum {
            return TallyResult::FailedQuorum;
        }

        let yes_count = votes.iter().filter(|v| v.vote == Vote::Yes).count();
        if yes_count as f32 > votes_cast / 2.0 {
            TallyResult::Passed
        } else {
            TallyResult::Rejected
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{MockGovernanceStore, ProposalStatus};

    fn zones(n: usize) -> Vec<ZoneId> {
        (0..n).map(|i| ZoneId([i as u8, 0, 0, 0])).collect()
    }

    fn zone(id: u8) -> ZoneId {
        ZoneId([id, 0, 0, 0])
    }

    fn test_proposal(quorum: f32) -> Proposal {
        Proposal {
            id: ProposalHash([1; 32]),
            proposer_zone: zone(0),
            title: "Test proposal".into(),
            action: "UpdatePolicy".into(),
            quorum,
            voting_deadline: Timestamp::from_secs(1000),
            status: ProposalStatus::Active,
        }
    }

    fn yes_vote(z: u8) -> SignedVote {
        SignedVote {
            voter_zone: zone(z),
            vote: Vote::Yes,
        }
    }

    fn no_vote(z: u8) -> SignedVote {
        SignedVote {
            voter_zone: zone(z),
            vote: Vote::No,
        }
    }

    #[test]
    fn tally_passes_with_majority() {
        let engine = GovernanceEngine::new(zones(10));
        let proposal = test_proposal(0.5);

        let mut votes = Vec::new();
        for i in 0..6 {
            votes.push(yes_vote(i));
        }
        for i in 6..8 {
            votes.push(no_vote(i));
        }

        let result = engine.tally(&proposal, &votes, Timestamp::from_secs(2000));
        assert_eq!(result, TallyResult::Passed);
    }

    #[test]
    fn tally_rejected_when_majority_votes_no() {
        let engine = GovernanceEngine::new(zones(10));
        let proposal = test_proposal(0.5);

        let mut votes = Vec::new();
        for i in 0..3 {
            votes.push(yes_vote(i));
        }
        for i in 3..8 {
            votes.push(no_vote(i));
        }

        let result = engine.tally(&proposal, &votes, Timestamp::from_secs(2000));
        assert_eq!(result, TallyResult::Rejected);
    }

    #[test]
    fn tally_fails_quorum_with_too_few_votes() {
        let engine = GovernanceEngine::new(zones(10));
        let proposal = test_proposal(0.5); // need 5 votes minimum

        let votes: Vec<SignedVote> = (0..4).map(|i| yes_vote(i)).collect();

        let result = engine.tally(&proposal, &votes, Timestamp::from_secs(2000));
        assert_eq!(result, TallyResult::FailedQuorum);
    }

    #[test]
    fn tally_pending_before_deadline() {
        let engine = GovernanceEngine::new(zones(10));
        let proposal = test_proposal(0.5);

        let votes: Vec<SignedVote> = (0..8).map(|i| yes_vote(i)).collect();

        let result = engine.tally(&proposal, &votes, Timestamp::from_secs(500)); // before deadline
        assert_eq!(result, TallyResult::Pending);
    }

    #[test]
    fn rejects_vote_from_unknown_zone() {
        let engine = GovernanceEngine::new(zones(5)); // zones 0-4
        let store = MockGovernanceStore::new();

        let vote = SignedVote {
            voter_zone: zone(99), // unknown
            vote: Vote::Yes,
        };

        let result = engine.cast_vote(&ProposalHash([1; 32]), &vote, &store);
        assert_eq!(result, Err(GovernanceError::UnknownZone));
    }

    #[test]
    fn cast_vote_stores_via_trait() {
        let engine = GovernanceEngine::new(zones(5));
        let mut store = MockGovernanceStore::new();

        store
            .expect_record_vote()
            .times(1)
            .returning(|_, _| Ok(()));

        let vote = yes_vote(2);
        engine
            .cast_vote(&ProposalHash([1; 32]), &vote, &store)
            .unwrap();
    }

    #[test]
    fn submit_proposal_stores_via_trait() {
        let engine = GovernanceEngine::new(zones(5));
        let mut store = MockGovernanceStore::new();

        store
            .expect_store_proposal()
            .times(1)
            .returning(|_| Ok(()));

        engine
            .submit_proposal(&test_proposal(0.5), &store)
            .unwrap();
    }
}
