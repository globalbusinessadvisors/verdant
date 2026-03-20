import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import { GovernancePortal } from "../GovernancePortal";
import type { ProposalWithVotes } from "../../lib/types";

const proposal: ProposalWithVotes = {
  id: "abc123",
  proposer_zone: "z1",
  title: "Increase monitoring",
  action: "set_interval 5m",
  quorum: 0.6,
  voting_deadline: 1_700_000_000_000,
  status: "Active",
  yes_count: 6,
  no_count: 2,
  abstain_count: 1,
};

describe("GovernancePortal", () => {
  it("shows vote counts", () => {
    render(
      <GovernancePortal
        proposals={[proposal]}
        isLoading={false}
        onSubmitProposal={vi.fn()}
        onCastVote={vi.fn()}
        onVerify={vi.fn()}
      />,
    );

    const voteCounts = screen.getByTestId("vote-counts");
    expect(voteCounts.textContent).toContain("6");
    expect(voteCounts.textContent).toContain("2");
  });

  it("has verify button", () => {
    render(
      <GovernancePortal
        proposals={[proposal]}
        isLoading={false}
        onSubmitProposal={vi.fn()}
        onCastVote={vi.fn()}
        onVerify={vi.fn()}
      />,
    );

    expect(screen.getByTestId("verify-button")).toBeInTheDocument();
  });

  it("calls verifyVoteIntegrity when verify button clicked", async () => {
    const user = userEvent.setup();
    const onVerify = vi.fn().mockResolvedValue({
      valid: true,
      total_votes: 8,
      discrepancies: 0,
    });

    render(
      <GovernancePortal
        proposals={[proposal]}
        isLoading={false}
        onSubmitProposal={vi.fn()}
        onCastVote={vi.fn()}
        onVerify={onVerify}
      />,
    );

    await user.click(screen.getByTestId("verify-button"));

    expect(onVerify).toHaveBeenCalledWith("abc123");

    const result = await screen.findByTestId("verify-result");
    expect(result.textContent).toContain("Valid");
    expect(result.textContent).toContain("8 votes");
    expect(result.textContent).toContain("0 discrepancies");
  });
});
