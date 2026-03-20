#!/usr/bin/env bash
set -euo pipefail

# -------------------------------------------------------------------
# deploy-gateway.sh — Cross-compile and deploy the Verdant gateway
# to a Raspberry Pi.
#
# Usage:
#   ./tools/deploy-gateway.sh [--host raspberrypi.local] [--user pi]
#
# Prerequisites:
#   - cross (cargo install cross)
#   - SSH access to the target host
# -------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

HOST="raspberrypi.local"
USER="pi"
TARGET="aarch64-unknown-linux-gnu"
PACKAGE="verdant-gateway"
REMOTE_DIR="/opt/verdant"
SERVICE_NAME="verdant-gateway"

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Cross-compile and deploy the Verdant gateway to a Raspberry Pi.

Options:
  --host HOSTNAME     Target host (default: raspberrypi.local)
  --user USERNAME     SSH user (default: pi)
  --target TARGET     Rust target triple (default: aarch64-unknown-linux-gnu)
  --help              Show this help message

Examples:
  $(basename "$0")
  $(basename "$0") --host 192.168.1.100 --user admin
EOF
    exit 0
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --host)   HOST="$2";   shift 2 ;;
        --user)   USER="$2";   shift 2 ;;
        --target) TARGET="$2"; shift 2 ;;
        --help)   usage ;;
        *)        echo "Error: Unknown option $1" >&2; usage ;;
    esac
done

REMOTE="$USER@$HOST"

echo "==> Cross-compiling $PACKAGE for $TARGET..."
cd "$PROJECT_ROOT"
cross build --release -p "$PACKAGE" --target "$TARGET"

BINARY="target/$TARGET/release/$PACKAGE"

if [[ ! -f "$BINARY" ]]; then
    echo "Error: Binary not found at $BINARY" >&2
    exit 1
fi

echo "==> Creating remote directory structure..."
ssh "$REMOTE" "sudo mkdir -p $REMOTE_DIR/bin $REMOTE_DIR/data $REMOTE_DIR/static"

echo "==> Copying binary to $HOST..."
scp "$BINARY" "$REMOTE:/tmp/$PACKAGE"
ssh "$REMOTE" "sudo mv /tmp/$PACKAGE $REMOTE_DIR/bin/$PACKAGE && sudo chmod +x $REMOTE_DIR/bin/$PACKAGE"

echo "==> Installing systemd service..."
cat <<UNIT | ssh "$REMOTE" "sudo tee /etc/systemd/system/$SERVICE_NAME.service > /dev/null"
[Unit]
Description=Verdant Mesh Gateway
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=$REMOTE_DIR/bin/$PACKAGE
WorkingDirectory=$REMOTE_DIR
Environment=VERDANT_DB_PATH=$REMOTE_DIR/data/verdant.redb
Environment=VERDANT_STATIC_DIR=$REMOTE_DIR/static
Environment=VERDANT_LISTEN_ADDR=0.0.0.0:8080
Restart=on-failure
RestartSec=5
User=$USER

[Install]
WantedBy=multi-user.target
UNIT

echo "==> Enabling and starting service..."
ssh "$REMOTE" "sudo systemctl daemon-reload && sudo systemctl enable $SERVICE_NAME && sudo systemctl restart $SERVICE_NAME"

echo "==> Checking service status..."
ssh "$REMOTE" "systemctl is-active $SERVICE_NAME"

echo "==> Deployment complete."
echo "    Host:    $HOST"
echo "    Binary:  $REMOTE_DIR/bin/$PACKAGE"
echo "    Data:    $REMOTE_DIR/data/"
echo "    Static:  $REMOTE_DIR/static/"
echo "    Service: $SERVICE_NAME"
echo ""
echo "Dashboard: http://$HOST:8080"
