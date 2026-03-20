import type {
  Alert,
  AlertRule,
  ConfirmedEvent,
  NodeStatus,
  Proposal,
  ProposalDraft,
  SyncOperation,
  SyncResult,
  Unsubscribe,
  VerificationResult,
  Vote,
} from "./types";

export interface MeshDataPort {
  fetchNodeStatuses(): Promise<NodeStatus[]>;
  fetchEvents(since: number): Promise<ConfirmedEvent[]>;
  subscribeEvents(
    callback: (event: ConfirmedEvent) => void,
  ): Unsubscribe;
}

export interface OfflineSyncPort {
  getLocalState<T>(key: string): Promise<T | null>;
  setLocalState<T>(key: string, value: T): Promise<void>;
  queueForSync(operation: SyncOperation): Promise<void>;
  processSyncQueue(): Promise<SyncResult>;
}

export interface GovernancePort {
  fetchProposals(): Promise<Proposal[]>;
  submitProposal(draft: ProposalDraft): Promise<string>;
  castVote(proposalId: string, vote: Vote): Promise<void>;
  verifyVoteIntegrity(proposalId: string): Promise<VerificationResult>;
}

export interface AlertPort {
  fetchAlerts(): Promise<Alert[]>;
  configureRule(rule: AlertRule): Promise<void>;
  dismissAlert(alertId: string): Promise<void>;
}
