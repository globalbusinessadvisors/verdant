// Domain types mirroring the verdant-gateway API contract.

/** 8-byte hardware-derived node identifier (hex-encoded). */
export type NodeId = string;

/** 4-byte zone identifier (hex-encoded). */
export type ZoneId = string;

/** Content-addressed proposal hash (hex-encoded). */
export type ProposalHash = string;

/** Millisecond-precision monotonic timestamp. */
export type Timestamp = number;

// ---------------------------------------------------------------------------
// Event classification
// ---------------------------------------------------------------------------

export type PestSpecies =
  | "EmeraldAshBorer"
  | "HemlockWoollyAdelgid"
  | "BeechLeafDisease"
  | "Unknown";

export type FloodSeverity = "Watch" | "Warning" | "Emergency";

export type InfrastructureType =
  | "PipeFreeze"
  | "RoadWashout"
  | "PowerLine"
  | "BridgeDamage";

export type MovementType = "Migration" | "Corridor" | "Unusual";

export type ClimateType =
  | "FreezeThaw"
  | "Drought"
  | "ExtremeHeat"
  | "IceStorm";

export type EventCategory =
  | { type: "Pest"; species_hint?: PestSpecies }
  | { type: "Flood"; severity: FloodSeverity; upstream_origin?: ZoneId }
  | { type: "Fire"; smoke_density: number }
  | { type: "Infrastructure"; sub_type: InfrastructureType }
  | { type: "Wildlife"; movement_type: MovementType }
  | { type: "Climate"; sub_type: ClimateType };

export type MissionType =
  | "SeedDispersal"
  | "WaterSensorDeploy"
  | "SupplyDelivery"
  | "VisualInspection"
  | "EmergencyRelay";

export type RecommendedAction =
  | { type: "AlertOnly" }
  | { type: "AlertAndMonitor" }
  | { type: "DispatchRobot"; mission_type: MissionType }
  | { type: "EvacuationWarning" }
  | { type: "PreemptiveAlert" };

// ---------------------------------------------------------------------------
// Core domain models
// ---------------------------------------------------------------------------

/** Event confirmed by cross-node spatial-temporal consensus. */
export interface ConfirmedEvent {
  event_id: number;
  category: EventCategory;
  confidence: number;
  affected_zone: ZoneId;
  corroborating_count: number;
  recommended_action: RecommendedAction;
  timestamp: Timestamp;
}

/** Status of a mesh node as reported to the gateway. */
export interface NodeStatus {
  node_id: NodeId;
  zone_id: ZoneId;
  last_seen: Timestamp;
  battery_level: number;
  graph_version: number;
  neighbor_count: number;
  uptime_secs: number;
}

// ---------------------------------------------------------------------------
// Governance
// ---------------------------------------------------------------------------

export type ProposalStatus = "Active" | "Passed" | "Rejected" | "FailedQuorum";

export interface Proposal {
  id: ProposalHash;
  proposer_zone: ZoneId;
  title: string;
  action: string;
  quorum: number;
  voting_deadline: Timestamp;
  status: ProposalStatus;
}

export type Vote = "Yes" | "No" | "Abstain";

export interface SignedVote {
  voter_zone: ZoneId;
  vote: Vote;
}

export interface ProposalDraft {
  proposer_zone: string;
  title: string;
  action: string;
  quorum: number;
  voting_deadline_secs: number;
}

export interface VoteRequest {
  voter_zone: string;
  vote: string;
}

export interface VerificationResult {
  valid: boolean;
  total_votes: number;
  discrepancies: number;
}

// ---------------------------------------------------------------------------
// Alerts
// ---------------------------------------------------------------------------

export interface FloodPreemptiveAlert {
  origin_event_id: number;
  target_zone: ZoneId;
  estimated_arrival_secs: number;
  severity: FloodSeverity;
  soil_saturation: number;
  timestamp: Timestamp;
}

export type Alert =
  | { type: "FloodPreemptive"; alert: FloodPreemptiveAlert }
  | { type: "Confirmed"; event: ConfirmedEvent };

export interface AlertRule {
  id: string;
  zone_id: ZoneId;
  category_filter?: EventCategory["type"];
  min_severity?: FloodSeverity;
  enabled: boolean;
}

// ---------------------------------------------------------------------------
// Robots
// ---------------------------------------------------------------------------

export type RobotState = "Idle" | "Navigating" | "Executing";

export interface RobotMission {
  mission_type: MissionType;
  target_zone: ZoneId;
  progress: number;
}

export interface RobotStatus {
  robot_id: NodeId;
  zone_id: ZoneId;
  state: RobotState;
  battery_level: number;
  current_mission?: RobotMission;
  last_seen: Timestamp;
  safety_ok: boolean;
}

// ---------------------------------------------------------------------------
// Sensor readings
// ---------------------------------------------------------------------------

export interface SensorReading {
  zone_id: ZoneId;
  temperature: number;
  humidity: number;
  soil_moisture: number;
  pressure: number;
  timestamp: Timestamp;
}

// ---------------------------------------------------------------------------
// Governance display
// ---------------------------------------------------------------------------

export interface ProposalWithVotes extends Proposal {
  yes_count: number;
  no_count: number;
  abstain_count: number;
}

// ---------------------------------------------------------------------------
// Offline sync
// ---------------------------------------------------------------------------

export interface SyncOperation {
  id: string;
  endpoint: string;
  method: "POST" | "PUT" | "DELETE";
  body: unknown;
  created_at: Timestamp;
}

export interface SyncResult {
  synced: number;
  failed: number;
  remaining: number;
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

export type Unsubscribe = () => void;
