import { useState } from "react";
import type { ProposalDraft, ProposalWithVotes, VerificationResult, Vote } from "../lib/types";

export interface GovernancePortalProps {
  proposals: ProposalWithVotes[];
  isLoading: boolean;
  onSubmitProposal: (draft: ProposalDraft) => Promise<string>;
  onCastVote: (proposalId: string, vote: Vote) => Promise<void>;
  onVerify: (proposalId: string) => Promise<VerificationResult>;
}

export function GovernancePortal({
  proposals,
  isLoading,
  onSubmitProposal,
  onCastVote,
  onVerify,
}: GovernancePortalProps) {
  const [title, setTitle] = useState("");
  const [action, setAction] = useState("");
  const [zone, setZone] = useState("");
  const [quorum, setQuorum] = useState("0.6");
  const [verifyResults, setVerifyResults] = useState<Record<string, VerificationResult>>({});

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    await onSubmitProposal({
      proposer_zone: zone,
      title,
      action,
      quorum: parseFloat(quorum),
      voting_deadline_secs: Math.floor(Date.now() / 1000) + 7 * 86400,
    });
    setTitle("");
    setAction("");
  };

  const handleVerify = async (id: string) => {
    const result = await onVerify(id);
    setVerifyResults((prev) => ({ ...prev, [id]: result }));
  };

  if (isLoading) return <p>Loading proposals…</p>;

  return (
    <section aria-label="Governance portal">
      <form onSubmit={handleSubmit} aria-label="Create proposal">
        <label>
          Zone
          <input value={zone} onChange={(e) => setZone(e.target.value)} required />
        </label>
        <label>
          Title
          <input value={title} onChange={(e) => setTitle(e.target.value)} required />
        </label>
        <label>
          Action
          <input value={action} onChange={(e) => setAction(e.target.value)} required />
        </label>
        <label>
          Quorum
          <input type="number" step="0.1" min="0" max="1" value={quorum} onChange={(e) => setQuorum(e.target.value)} required />
        </label>
        <button type="submit">Submit Proposal</button>
      </form>

      <ul aria-label="Proposals" style={{ listStyle: "none", padding: 0 }}>
        {proposals.map((p) => (
          <li key={p.id} data-testid="proposal-item" style={{ padding: "12px 0", borderBottom: "1px solid #e2e8f0" }}>
            <strong>{p.title}</strong>
            <span style={{ marginLeft: 8 }}>({p.status})</span>
            <div data-testid="vote-counts">
              {p.yes_count} / {p.no_count}
              {p.abstain_count > 0 && <span> ({p.abstain_count} abstain)</span>}
            </div>
            {p.status === "Active" && (
              <div style={{ display: "flex", gap: 8, marginTop: 4 }}>
                <button onClick={() => onCastVote(p.id, "Yes")}>Vote Yes</button>
                <button onClick={() => onCastVote(p.id, "No")}>Vote No</button>
                <button onClick={() => onCastVote(p.id, "Abstain")}>Abstain</button>
              </div>
            )}
            <button onClick={() => handleVerify(p.id)} data-testid="verify-button" style={{ marginTop: 4 }}>
              Verify
            </button>
            {verifyResults[p.id] != null && (
              <div data-testid="verify-result">
                {verifyResults[p.id]!.valid ? "Valid" : "Invalid"} — {verifyResults[p.id]!.total_votes} votes, {verifyResults[p.id]!.discrepancies} discrepancies
              </div>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}
