# ADR-004: Decentralized Autonomous Application (DAA) Governance

**Status:** Accepted
**Date:** 2026-03-20
**Deciders:** Verdant Core Team
**SPARC Refs:** FR-G2, FR-G5, G5, G10

## Context

The Verdant mesh must be collectively owned by Vermont landowners — no corporation, utility, or government agency controls it. Governance decisions (adding nodes, updating firmware, allocating credits, changing policies) must be made transparently and verifiably without a central authority. Vermont's culture of town meeting democracy provides the governance model.

Candidate approaches:
1. **Centralized admin panel** — simple but violates data sovereignty; single point of failure/control
2. **Blockchain-based DAO** — decentralized but requires internet connectivity and token economics
3. **On-mesh DAA (Decentralized Autonomous Application)** — governance runs entirely on the mesh, no internet required, no tokens, zone-based voting
4. **Federated model** — each gateway has authority over its zone, gateways negotiate bilaterally

## Decision

Use an **on-mesh DAA** with zone-based voting, quorum requirements, and the QuDAG protocol for transparent, tamper-evident governance records.

## Rationale

- **No internet required.** Unlike blockchain DAOs, the DAA runs entirely on the mesh. Governance votes propagate via QuDAG messages between nodes.
- **Zone-based, not individual.** Zones (10-50 nodes owned by a single landowner or community) vote as a unit. This mirrors Vermont's town meeting model where each town has a voice.
- **Quorum prevents minority takeover.** Different action types require different quorum levels (e.g., firmware updates require supermajority; adding a node requires simple majority).
- **DAG provides auditability.** Every proposal, vote, and outcome is recorded in the DAG — tamper-evident without a blockchain's energy/connectivity requirements.
- **No token economics.** Credits for sharing environmental data are tracked as ledger entries in the DAG, not as tradeable tokens. This avoids financialization and regulatory complexity.

## Consequences

- **Positive:** Fully sovereign; works offline; mirrors Vermont culture; no token/blockchain dependency
- **Negative:** Governance is eventually-consistent (votes may take hours to propagate across partitioned mesh); requires zone key management
- **Mitigated by:** Voting deadlines measured in days (not minutes); zone key ceremonies documented as part of node provisioning

## London School TDD Approach

```rust
pub trait ProposalBroadcaster {
    fn broadcast_proposal(&mut self, proposal: &Proposal) -> Result<(), GovernanceError>;
    fn broadcast_vote(&mut self, proposal_id: &ProposalHash, vote: &SignedVote) -> Result<(), GovernanceError>;
}

pub trait ZoneRegistry {
    fn active_zone_count(&self) -> usize;
    fn is_valid_zone(&self, zone_id: &ZoneId) -> bool;
}

pub trait GovernanceExecutor {
    fn execute(&mut self, action: &GovernanceAction) -> Result<(), GovernanceError>;
}

pub trait Clock {
    fn now(&self) -> Timestamp;
}

#[cfg(test)]
mod tests {
    #[test]
    fn submit_proposal_broadcasts_to_mesh() {
        let mut mock_broadcaster = MockProposalBroadcaster::new();

        // THEN: proposal is broadcast exactly once
        mock_broadcaster.expect_broadcast_proposal()
            .withf(|p| p.title == "Add monitoring nodes to Zone Bravo")
            .times(1)
            .returning(|_| Ok(()));

        let mut gov = GovernanceEngine::new(
            mock_broadcaster, mock_registry, mock_executor, mock_clock
        );
        let proposal = Proposal::new(
            zone_alpha,
            "Add monitoring nodes to Zone Bravo",
            GovernanceAction::AddNode { zone: zone_bravo, count: 5 },
            0.5, // quorum
        );

        gov.submit_proposal(proposal).unwrap();
    }

    #[test]
    fn tally_returns_passed_when_majority_votes_yes() {
        let mut mock_registry = MockZoneRegistry::new();
        mock_registry.expect_active_zone_count().returning(|| 10);

        let mut mock_executor = MockGovernanceExecutor::new();
        // THEN: the action is executed
        mock_executor.expect_execute()
            .withf(|a| matches!(a, GovernanceAction::UpdatePolicy { .. }))
            .times(1)
            .returning(|_| Ok(()));

        let mut mock_clock = MockClock::new();
        mock_clock.expect_now().returning(|| Timestamp::from_secs(1_000_000));

        let mut gov = GovernanceEngine::new(
            mock_broadcaster, mock_registry, mock_executor, mock_clock
        );

        // GIVEN: a proposal with deadline in the past
        let mut proposal = Proposal::new(
            zone_alpha, "Update flood threshold",
            GovernanceAction::UpdatePolicy { key: "flood_threshold".into(), value: "0.9".into() },
            0.5,
        );
        proposal.voting_deadline = Timestamp::from_secs(999_999); // past

        // AND: 6 of 10 zones voted yes, 2 voted no
        for i in 0..6 { proposal.record_vote(zone(i), Vote::Yes); }
        for i in 6..8 { proposal.record_vote(zone(i), Vote::No); }

        // WHEN: we tally
        let result = gov.tally(&proposal);

        // THEN: proposal passed (6 > 8/2, quorum met: 8/10 >= 0.5)
        assert_eq!(result, TallyResult::Passed);
    }

    #[test]
    fn tally_returns_failed_quorum_when_too_few_votes() {
        let mut mock_registry = MockZoneRegistry::new();
        mock_registry.expect_active_zone_count().returning(|| 10);

        let mut mock_executor = MockGovernanceExecutor::new();
        // THEN: action is NOT executed
        mock_executor.expect_execute().times(0);

        let mut gov = GovernanceEngine::new(
            mock_broadcaster, mock_registry, mock_executor, mock_clock
        );

        let mut proposal = expired_proposal(0.5); // quorum = 50%
        // Only 4 of 10 zones voted (below quorum)
        for i in 0..4 { proposal.record_vote(zone(i), Vote::Yes); }

        let result = gov.tally(&proposal);
        assert_eq!(result, TallyResult::FailedQuorum);
    }

    #[test]
    fn tally_returns_pending_when_deadline_not_reached() {
        let mut mock_clock = MockClock::new();
        mock_clock.expect_now().returning(|| Timestamp::from_secs(100));

        let mut mock_executor = MockGovernanceExecutor::new();
        mock_executor.expect_execute().times(0);

        let mut gov = GovernanceEngine::new(
            mock_broadcaster, mock_registry, mock_executor, mock_clock
        );

        let proposal = Proposal {
            voting_deadline: Timestamp::from_secs(200), // future
            ..valid_proposal()
        };

        let result = gov.tally(&proposal);
        assert_eq!(result, TallyResult::Pending);
    }

    #[test]
    fn rejects_vote_from_unknown_zone() {
        let mut mock_registry = MockZoneRegistry::new();
        mock_registry.expect_is_valid_zone()
            .with(eq(unknown_zone))
            .returning(|_| false);

        let mut mock_broadcaster = MockProposalBroadcaster::new();
        // THEN: vote is NOT broadcast
        mock_broadcaster.expect_broadcast_vote().times(0);

        let mut gov = GovernanceEngine::new(
            mock_broadcaster, mock_registry, mock_executor, mock_clock
        );

        let result = gov.cast_vote(proposal_id, Vote::Yes, unknown_zone, &zone_key);
        assert!(matches!(result, Err(GovernanceError::UnknownZone(_))));
    }
}
```
