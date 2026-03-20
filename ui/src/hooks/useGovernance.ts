import { useCallback, useEffect, useState } from "react";
import type { GovernancePort } from "../lib/ports";
import type { Proposal, ProposalDraft, VerificationResult, Vote } from "../lib/types";

export interface UseGovernanceResult {
  proposals: Proposal[];
  isLoading: boolean;
  submitProposal: (draft: ProposalDraft) => Promise<string>;
  castVote: (proposalId: string, vote: Vote) => Promise<void>;
  verifyIntegrity: (proposalId: string) => Promise<VerificationResult>;
  refresh: () => Promise<void>;
}

export function useGovernance(
  governancePort: GovernancePort,
): UseGovernanceResult {
  const [proposals, setProposals] = useState<Proposal[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const fetched = await governancePort.fetchProposals();
      setProposals(fetched);
    } finally {
      setIsLoading(false);
    }
  }, [governancePort]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const submitProposal = useCallback(
    async (draft: ProposalDraft): Promise<string> => {
      const id = await governancePort.submitProposal(draft);
      await refresh();
      return id;
    },
    [governancePort, refresh],
  );

  const castVote = useCallback(
    async (proposalId: string, vote: Vote): Promise<void> => {
      await governancePort.castVote(proposalId, vote);
      await refresh();
    },
    [governancePort, refresh],
  );

  const verifyIntegrity = useCallback(
    (proposalId: string) => governancePort.verifyVoteIntegrity(proposalId),
    [governancePort],
  );

  return { proposals, isLoading, submitProposal, castVote, verifyIntegrity, refresh };
}
