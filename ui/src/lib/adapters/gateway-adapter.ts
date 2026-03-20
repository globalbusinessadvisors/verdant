import type { MeshDataPort, GovernancePort, AlertPort } from "../ports";
import type {
  Alert,
  AlertRule,
  ConfirmedEvent,
  NodeStatus,
  Proposal,
  ProposalDraft,
  Unsubscribe,
  VerificationResult,
  Vote,
} from "../types";

const JSON_HEADERS = { "Content-Type": "application/json" } as const;

export class GatewayMeshAdapter implements MeshDataPort {
  constructor(
    private readonly baseUrl: string,
    private readonly wsUrl: string,
  ) {}

  async fetchNodeStatuses(): Promise<NodeStatus[]> {
    const res = await fetch(`${this.baseUrl}/api/nodes`);
    if (!res.ok) throw new Error(`fetchNodeStatuses: ${res.status}`);
    return res.json() as Promise<NodeStatus[]>;
  }

  async fetchEvents(since: number): Promise<ConfirmedEvent[]> {
    const url = `${this.baseUrl}/api/events?since=${since}`;
    const res = await fetch(url);
    if (!res.ok) throw new Error(`fetchEvents: ${res.status}`);
    return res.json() as Promise<ConfirmedEvent[]>;
  }

  subscribeEvents(
    callback: (event: ConfirmedEvent) => void,
  ): Unsubscribe {
    const ws = new WebSocket(`${this.wsUrl}/api/events/ws`);
    ws.onmessage = (msg) => {
      const event = JSON.parse(msg.data as string) as ConfirmedEvent;
      callback(event);
    };
    return () => ws.close();
  }
}

export class GatewayGovernanceAdapter implements GovernancePort {
  constructor(private readonly baseUrl: string) {}

  async fetchProposals(): Promise<Proposal[]> {
    const res = await fetch(`${this.baseUrl}/api/governance/proposals`);
    if (!res.ok) throw new Error(`fetchProposals: ${res.status}`);
    return res.json() as Promise<Proposal[]>;
  }

  async submitProposal(draft: ProposalDraft): Promise<string> {
    const res = await fetch(`${this.baseUrl}/api/governance/proposals`, {
      method: "POST",
      headers: JSON_HEADERS,
      body: JSON.stringify(draft),
    });
    if (!res.ok) throw new Error(`submitProposal: ${res.status}`);
    const body = (await res.json()) as { id: string };
    return body.id;
  }

  async castVote(proposalId: string, vote: Vote): Promise<void> {
    const res = await fetch(
      `${this.baseUrl}/api/governance/proposals/${proposalId}/vote`,
      {
        method: "POST",
        headers: JSON_HEADERS,
        body: JSON.stringify({ vote: vote.toLowerCase() }),
      },
    );
    if (!res.ok) throw new Error(`castVote: ${res.status}`);
  }

  async verifyVoteIntegrity(
    proposalId: string,
  ): Promise<VerificationResult> {
    const res = await fetch(
      `${this.baseUrl}/api/governance/proposals/${proposalId}/verify`,
    );
    if (!res.ok) throw new Error(`verifyVoteIntegrity: ${res.status}`);
    return res.json() as Promise<VerificationResult>;
  }
}

export class GatewayAlertAdapter implements AlertPort {
  constructor(private readonly baseUrl: string) {}

  async fetchAlerts(): Promise<Alert[]> {
    const res = await fetch(`${this.baseUrl}/api/alerts`);
    if (!res.ok) throw new Error(`fetchAlerts: ${res.status}`);
    return res.json() as Promise<Alert[]>;
  }

  async configureRule(rule: AlertRule): Promise<void> {
    const res = await fetch(`${this.baseUrl}/api/alerts/rules`, {
      method: "POST",
      headers: JSON_HEADERS,
      body: JSON.stringify(rule),
    });
    if (!res.ok) throw new Error(`configureRule: ${res.status}`);
  }

  async dismissAlert(alertId: string): Promise<void> {
    const res = await fetch(
      `${this.baseUrl}/api/alerts/${alertId}/dismiss`,
      { method: "POST" },
    );
    if (!res.ok) throw new Error(`dismissAlert: ${res.status}`);
  }
}
