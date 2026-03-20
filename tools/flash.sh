#!/usr/bin/env bash
set -euo pipefail

# -------------------------------------------------------------------
# flash.sh — Build and flash Verdant firmware to an ESP32-S3 node.
#
# Usage:
#   ./tools/flash.sh /dev/ttyUSB0 [--zone ZONE_ID] [--node-index N]
#
# Prerequisites:
#   - Rust xtensa toolchain (espup)
#   - espflash (cargo install espflash)
# -------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

ZONE_ID=0
NODE_INDEX=0
PORT=""
TARGET="xtensa-esp32s3-none-elf"
PACKAGE="verdant-firmware"

usage() {
    cat <<EOF
Usage: $(basename "$0") PORT [OPTIONS]

Flash Verdant firmware to an ESP32-S3 node.

Arguments:
  PORT                Serial port (e.g., /dev/ttyUSB0)

Options:
  --zone ZONE_ID      Zone identifier (0-255, default: 0)
  --node-index N      Node index within zone (0-255, default: 0)
  --help              Show this help message

Examples:
  $(basename "$0") /dev/ttyUSB0
  $(basename "$0") /dev/ttyUSB0 --zone 3 --node-index 7
EOF
    exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --zone)
            ZONE_ID="$2"
            shift 2
            ;;
        --node-index)
            NODE_INDEX="$2"
            shift 2
            ;;
        --help)
            usage
            ;;
        -*)
            echo "Error: Unknown option $1" >&2
            usage
            ;;
        *)
            PORT="$1"
            shift
            ;;
    esac
done

if [[ -z "$PORT" ]]; then
    echo "Error: Serial port required." >&2
    usage
fi

echo "==> Building firmware for $TARGET..."
cd "$PROJECT_ROOT"
cargo build --release -p "$PACKAGE" --target "$TARGET"

FIRMWARE_BIN="target/$TARGET/release/$PACKAGE"

if [[ ! -f "$FIRMWARE_BIN" ]]; then
    echo "Error: Firmware binary not found at $FIRMWARE_BIN" >&2
    exit 1
fi

echo "==> Flashing to $PORT (zone=$ZONE_ID, node_index=$NODE_INDEX)..."
espflash flash \
    --monitor \
    --port "$PORT" \
    "$FIRMWARE_BIN"

echo "==> Writing node identity to config partition..."
# Node identity is encoded as: [zone_id(4 bytes), node_index(1 byte)]
# Written to the 'nvs' partition via espflash partition-table or esptool.
ZONE_HEX=$(printf '%02x000000' "$ZONE_ID")
NODE_HEX=$(printf '%02x' "$NODE_INDEX")

python3 -c "
import sys
zone = bytes.fromhex('$ZONE_HEX')
node = bytes.fromhex('$NODE_HEX')
identity = zone + node + b'\\x00' * 27  # 32-byte identity block
sys.stdout.buffer.write(identity)
" > /tmp/verdant_node_identity.bin

espflash write-bin \
    --port "$PORT" \
    0x3F0000 \
    /tmp/verdant_node_identity.bin

echo "==> Generating Dilithium keypair..."
# The firmware generates its own Dilithium keypair on first boot
# and stores the public key in a designated flash region.
# This is handled by the firmware's storage module, not by this script.
# The keypair is derived from the ESP32's hardware-unique eFuse ID.

echo "==> Flash complete."
echo "    Zone:       $ZONE_ID"
echo "    Node Index: $NODE_INDEX"
echo "    Port:       $PORT"
echo ""
echo "The node will generate its Dilithium keypair on first boot."
