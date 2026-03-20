import { renderHook, waitFor, act } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { useGovernance } from "./useGovernance";
import type { GovernancePort } from "../lib/ports";
import type { ProposalDraft } from "../lib/types";

function mockGovernancePort(
  overrides: Partial<GovernancePort> = {},
): GovernancePort {
  return {
    fetchProposals: vi.fn().mockResolvedValue([]),
    submitProposal: vi.fn().mockResolvedValue("proposal-123"),
    castVote: vi.fn().mockResolvedValue(undefined),
    verifyVoteIntegrity: vi
      .fn()
      .mockResolvedValue({ valid: true, total_votes: 5, discrepancies: 0 }),
    ...overrides,
  };
}

const sampleDraft: ProposalDraft = {
  proposer_zone: "aabb0011",
  title: "Increase monitoring frequency",
  action: "set_interval 5m",
  quorum: 0.6,
  voting_deadline_secs: 1700000000,
};

describe("useGovernance", () => {
  it("submit_proposal_calls_port", async () => {
    const port = mockGovernancePort();

    const { result } = renderHook(() => useGovernance(port));

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    let id: string | undefined;
    await act(async () => {
      id = await result.current.submitProposal(sampleDraft);
    });

    expect(id).toBe("proposal-123");
    expect(port.submitProposal).toHaveBeenCalledWith(sampleDraft);
  });

  it("cast_vote_calls_port", async () => {
    const port = mockGovernancePort();

    const { result } = renderHook(() => useGovernance(port));

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    await act(async () => {
      await result.current.castVote("proposal-456", "Yes");
    });

    expect(port.castVote).toHaveBeenCalledWith("proposal-456", "Yes");
  });
});
