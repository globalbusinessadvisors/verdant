import { Routes, Route } from "react-router-dom";
import { MeshMap } from "./components/MeshMap";
import { EventTimeline } from "./components/EventTimeline";
import { AlertManager } from "./components/AlertManager";
import { GovernancePortal } from "./components/GovernancePortal";
import { FarmDashboard } from "./components/FarmDashboard";
import { RobotOverview } from "./components/RobotOverview";
import { FloodTracker } from "./components/FloodTracker";
import type {
  ConfirmedEvent,
  FloodPreemptiveAlert,
  NodeStatus,
  ProposalWithVotes,
  RobotStatus,
  SensorReading,
  Alert,
  ZoneId,
} from "./lib/types";

// Placeholder data for initial render — replaced by hooks wired to ports at integration time
const EMPTY_NODES: NodeStatus[] = [];
const EMPTY_EVENTS: ConfirmedEvent[] = [];
const EMPTY_PROPOSALS: ProposalWithVotes[] = [];
const EMPTY_FLOOD: FloodPreemptiveAlert[] = [];
const EMPTY_ROBOTS: RobotStatus[] = [];
const EMPTY_READINGS: SensorReading[] = [];
const EMPTY_ALERTS: Alert[] = [];
const EMPTY_ZONES: ZoneId[] = [];

const noop = () => Promise.resolve("" as string);
const noopVoid = () => Promise.resolve();

export function App() {
  return (
    <Routes>
      <Route
        path="/"
        element={<MeshMap nodes={EMPTY_NODES} events={EMPTY_EVENTS} />}
      />
      <Route
        path="/events"
        element={<EventTimeline events={EMPTY_EVENTS} />}
      />
      <Route
        path="/alerts"
        element={
          <AlertManager
            alerts={EMPTY_ALERTS}
            zones={EMPTY_ZONES}
            onConfigureRule={noopVoid}
            onDismiss={noopVoid}
          />
        }
      />
      <Route
        path="/governance"
        element={
          <GovernancePortal
            proposals={EMPTY_PROPOSALS}
            isLoading={false}
            onSubmitProposal={noop}
            onCastVote={noopVoid}
            onVerify={() =>
              Promise.resolve({ valid: true, total_votes: 0, discrepancies: 0 })
            }
          />
        }
      />
      <Route
        path="/farm"
        element={<FarmDashboard readings={EMPTY_READINGS} />}
      />
      <Route
        path="/flood"
        element={<FloodTracker alerts={EMPTY_FLOOD} />}
      />
      <Route
        path="/robots"
        element={<RobotOverview robots={EMPTY_ROBOTS} />}
      />
    </Routes>
  );
}
