#!/usr/bin/env bash
set -euo pipefail

# -------------------------------------------------------------------
# build-dashboard.sh — Build the Verdant dashboard PWA and optionally
# deploy to a gateway host.
#
# Usage:
#   ./tools/build-dashboard.sh [--gateway-url URL] [--deploy HOST]
#
# Prerequisites:
#   - Node.js >= 18
#   - npm
# -------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
UI_DIR="$PROJECT_ROOT/ui"

GATEWAY_URL=""
DEPLOY_HOST=""
DEPLOY_USER="pi"
REMOTE_STATIC="/opt/verdant/static"

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Build the Verdant dashboard PWA.

Options:
  --gateway-url URL   Gateway API base URL for production build
                      (default: same-origin, i.e., served by gateway)
  --deploy HOST       SCP dist/ to HOST after building
  --user USERNAME     SSH user for deploy (default: pi)
  --help              Show this help message

Examples:
  $(basename "$0")
  $(basename "$0") --gateway-url http://192.168.1.100:8080
  $(basename "$0") --deploy raspberrypi.local
EOF
    exit 0
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --gateway-url) GATEWAY_URL="$2"; shift 2 ;;
        --deploy)      DEPLOY_HOST="$2"; shift 2 ;;
        --user)        DEPLOY_USER="$2"; shift 2 ;;
        --help)        usage ;;
        *)             echo "Error: Unknown option $1" >&2; usage ;;
    esac
done

echo "==> Installing dependencies..."
cd "$UI_DIR"
npm ci --prefer-offline 2>/dev/null || npm install

if [[ -n "$GATEWAY_URL" ]]; then
    echo "==> Setting gateway URL: $GATEWAY_URL"
    export VITE_GATEWAY_URL="$GATEWAY_URL"
fi

echo "==> Building dashboard PWA..."
npm run build

DIST="$UI_DIR/dist"

if [[ ! -d "$DIST" ]]; then
    echo "Error: Build output not found at $DIST" >&2
    exit 1
fi

SIZE=$(du -sh "$DIST" | cut -f1)
echo "==> Build complete: $DIST ($SIZE)"

# Build WASM verifier if wasm-pack is available
if command -v wasm-pack &> /dev/null; then
    echo "==> Building QuDAG WASM verifier..."
    cd "$PROJECT_ROOT/crates/verdant-qudag-wasm"
    wasm-pack build --target web --release
    mkdir -p "$DIST/pkg"
    cp pkg/verdant_qudag_wasm_bg.wasm "$DIST/pkg/"
    cp pkg/verdant_qudag_wasm.js "$DIST/pkg/"
    echo "    WASM verifier included in build."
else
    echo "    wasm-pack not found — skipping WASM verifier build."
fi

if [[ -n "$DEPLOY_HOST" ]]; then
    REMOTE="$DEPLOY_USER@$DEPLOY_HOST"
    echo "==> Deploying to $DEPLOY_HOST..."
    ssh "$REMOTE" "sudo mkdir -p $REMOTE_STATIC"
    rsync -az --delete "$DIST/" "$REMOTE:$REMOTE_STATIC/"
    echo "==> Deployed to $REMOTE_STATIC on $DEPLOY_HOST"
    echo "    Dashboard: http://$DEPLOY_HOST:8080"
else
    echo ""
    echo "To deploy, run:"
    echo "  $(basename "$0") --deploy raspberrypi.local"
fi
