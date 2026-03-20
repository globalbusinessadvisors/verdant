#!/usr/bin/env bash
set -euo pipefail

# -------------------------------------------------------------------
# sim-scenario.sh — Run pre-configured simulation scenarios.
#
# Usage:
#   ./tools/sim-scenario.sh SCENARIO [--nodes N]
#
# Scenarios: flood, pest, partition, governance, stress
# -------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

NODES=50
SCENARIO=""

usage() {
    cat <<EOF
Usage: $(basename "$0") SCENARIO [OPTIONS]

Run a Verdant mesh simulation scenario.

Scenarios:
  flood         Flood event detection and downstream alert propagation
  pest          Pest spread detection across zone boundaries
  partition     Mesh self-healing after node failure
  governance    Governance proposal submission and voting
  stress        1000-node stress test

Options:
  --nodes N     Number of nodes (default: 50, stress always uses 1000)
  --help        Show this help message

Examples:
  $(basename "$0") flood
  $(basename "$0") pest --nodes 100
  $(basename "$0") stress
EOF
    exit 0
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --nodes) NODES="$2"; shift 2 ;;
        --help)  usage ;;
        -*)      echo "Error: Unknown option $1" >&2; usage ;;
        *)       SCENARIO="$1"; shift ;;
    esac
done

if [[ -z "$SCENARIO" ]]; then
    echo "Error: Scenario required." >&2
    usage
fi

cd "$PROJECT_ROOT"

case "$SCENARIO" in
    flood)
        echo "==> Running flood scenario ($NODES nodes)..."
        cargo test -p verdant-sim --test integration -- flood_detected_and_downstream_alerted --nocapture
        echo "==> Running full pipeline flood test..."
        cargo test -p verdant-sim --test full_pipeline -- flood_event_flows_through_to_api --nocapture
        ;;
    pest)
        echo "==> Running pest scenario ($NODES nodes)..."
        cargo test -p verdant-sim --test integration -- pest_detection_across_zone_boundary --nocapture
        echo "==> Running full pipeline pest test..."
        cargo test -p verdant-sim --test full_pipeline -- pest_spread_confirmed_in_dashboard_api --nocapture
        ;;
    partition)
        echo "==> Running partition recovery scenario ($NODES nodes)..."
        cargo test -p verdant-sim --test integration -- mesh_self_heals_after_node_failure --nocapture
        ;;
    governance)
        echo "==> Running governance scenario..."
        cargo test -p verdant-sim --test full_pipeline -- governance_proposal_vote_tally_via_api --nocapture
        ;;
    stress)
        echo "==> Running 1000-node stress test..."
        cargo bench -p verdant-sim --bench sim_benchmarks -- "1000-node"
        echo ""
        echo "==> Running 50-node integration stress..."
        cargo test -p verdant-sim --test integration -- fifty_node_simulation_runs_without_panic --nocapture
        ;;
    *)
        echo "Error: Unknown scenario '$SCENARIO'" >&2
        echo "Valid scenarios: flood, pest, partition, governance, stress" >&2
        exit 1
        ;;
esac

echo ""
echo "==> Scenario '$SCENARIO' complete."
