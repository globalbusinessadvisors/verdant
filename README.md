<p align="center">
  <img src="https://img.shields.io/badge/Rust-no__std-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust no_std" />
  <img src="https://img.shields.io/badge/TypeScript-React_PWA-3178C6?style=for-the-badge&logo=typescript&logoColor=white" alt="TypeScript" />
  <img src="https://img.shields.io/badge/ESP32--S3-Embedded-E7352C?style=for-the-badge&logo=espressif&logoColor=white" alt="ESP32-S3" />
  <img src="https://img.shields.io/badge/Post--Quantum-FIPS_204-6C3483?style=for-the-badge&logoColor=white" alt="Post-Quantum" />
  <img src="https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-blue?style=for-the-badge" alt="License" />
</p>

<p align="center">
  <img src="https://img.shields.io/badge/tests-297_passing-2ea44f?style=flat-square" alt="Tests" />
  <img src="https://img.shields.io/badge/crates-11-orange?style=flat-square" alt="Crates" />
  <img src="https://img.shields.io/badge/Rust_LOC-11.7k-informational?style=flat-square" alt="Rust LOC" />
  <img src="https://img.shields.io/badge/TypeScript_LOC-2k-informational?style=flat-square" alt="TS LOC" />
  <img src="https://img.shields.io/badge/WASM-141_KB-success?style=flat-square" alt="WASM size" />
</p>

---

<h1 align="center">Verdant</h1>
<h3 align="center">Self-Healing Forest Mesh Intelligence for Vermont</h3>

<p align="center">
A peer-to-peer neural fabric for forests, farms, watersheds, and rural communities.<br/>
Zero cloud dependency. Zero cameras. Quantum-resistant. Solar-powered.
</p>

---

## Overview

Verdant deploys a network of solar-powered ESP32-S3 sensor nodes across Vermont's forests and farmland. Each node learns its local environment through RF sensing and environmental sensors, shares anomaly intelligence across a self-healing mesh, and surfaces confirmed events — pest outbreaks, flooding, fire, infrastructure damage — through a decentralized governance layer where communities control their own data.

The system operates without cell towers, without cloud services, and without cameras. All intelligence runs on-mesh or on local Raspberry Pi gateways. Communities govern the network through direct democratic voting modeled on Vermont's town meeting tradition.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Dashboard PWA (TypeScript)                │
│         Leaflet Map · Event Timeline · Governance           │
│         Farm Dashboard · Flood Tracker · Robots             │
├─────────────────────────────────────────────────────────────┤
│                  WASM Verifier (ml-dsa)                     │
│         In-browser vote integrity verification              │
├─────────────────────────────────────────────────────────────┤
│                Gateway Daemon (Raspberry Pi)                 │
│      axum REST/WS · redb storage · Mesh Bridge · Sync      │
├─────────────────────────────────────────────────────────────┤
│                    Mesh Network Layer                        │
│   QuDAG Protocol · BLE Discovery · Terrain-Aware Routing    │
│   Onion Routing · Partition Recovery · Adaptive Compression │
├─────────────────────────────────────────────────────────────┤
│                   Intelligence Layer                         │
│    Vector Graph Learning · Seasonal Baselines · Consensus   │
│    Flood Propagation · Cross-Zone Anomaly Correlation       │
├─────────────────────────────────────────────────────────────┤
│               Node Firmware (ESP32-S3, no_std)              │
│  WiFi CSI Capture · Sensor Fusion · Power Management · OTA │
│  CRYSTALS-Dilithium Signatures · CRYSTALS-Kyber KEM        │
└─────────────────────────────────────────────────────────────┘
```

## Workspace Crates

| Crate | Description | `no_std` |
|-------|-------------|----------|
| **`verdant-core`** | Shared types, traits, error types, and configuration constants | Yes |
| **`verdant-vector`** | Vector graph neural network — incremental learning, seasonal baselines, anomaly scoring | Yes |
| **`verdant-qudag`** | QuDAG protocol — DAG message ordering, post-quantum crypto, onion routing, bloom dedup | Yes |
| **`verdant-mesh`** | Mesh networking — BLE/WiFi discovery, distance-vector routing, partition recovery | Yes |
| **`verdant-sense`** | Sensor abstraction — WiFi CSI capture, environmental sensors, feature fusion | Yes |
| **`verdant-safla`** | Self-aware feedback loop — health monitoring, anomaly consensus, flood propagation | Yes |
| **`verdant-firmware`** | ESP32-S3 node firmware — duty cycling, power management, OTA updates, flash storage | Yes |
| **`verdant-gateway`** | Raspberry Pi gateway — axum HTTP/WS API, redb persistence, mesh bridge, governance | No |
| **`verdant-robotics`** | Agentic robotics — swarm coordination, quantum magnetometer navigation, mission planning | Yes |
| **`verdant-sim`** | Discrete-event simulator — terrain modeling, weather generation, scenario testing | No |
| **`verdant-qudag-wasm`** | WASM bridge — ML-DSA-65 signature verification, DAG validation in browser | No |

## Key Capabilities

### Environmental Intelligence
- **WiFi CSI sensing** — detect environmental changes through RF signal patterns, no cameras
- **Vector graph learning** — each node builds a 200 KB self-learning model of its environment
- **Seasonal baselines** — week-of-year bucketed centroids eliminate false positives from normal seasonal variation
- **Cross-node consensus** — spatial-temporal correlation across multiple nodes confirms events before alerting

### Mesh Networking
- **QuDAG protocol** — DAG-based causal ordering without blockchain overhead
- **Post-quantum security** — CRYSTALS-Dilithium (FIPS 204) signatures, CRYSTALS-Kyber (FIPS 203) key encapsulation
- **Onion routing** — minimum 3-hop anonymous relay protects node privacy
- **Self-healing** — partition detection and autonomous reconnection in < 60 seconds
- **Terrain-aware routing** — cost functions account for distance, packet delivery ratio, and link quality

### Decentralized Governance
- **Zone-based voting** — 10–50 nodes per zone, quorum requirements per action type
- **Proposal lifecycle** — submit, vote, tally, execute — all on-mesh
- **In-browser verification** — WASM-compiled ML-DSA-65 verifier lets users independently validate vote integrity
- **No trusted gateway** — the browser verifies cryptographic proofs directly

### Offline-First Dashboard
- **Progressive Web App** — works on phone, tablet, laptop
- **IndexedDB caching** — full offline data access with background sync
- **Service worker** — cache-first for assets, network-first for API
- **Leaflet maps** — topographic mesh visualization with node health indicators

## Quick Start

### Prerequisites

- Rust 1.82+ (stable)
- Node.js 18+
- wasm-pack (for WASM verifier)

### Build & Test

```bash
# Run all Rust tests (280 tests)
cargo test --workspace

# Run UI tests (17 tests)
cd ui && npm install && npm test

# Build the dashboard PWA
./tools/build-dashboard.sh

# Build WASM verifier (141 KB)
cd crates/verdant-qudag-wasm && wasm-pack build --target web --release
```

### Run Simulations

```bash
# Flood detection and downstream alert propagation
./tools/sim-scenario.sh flood

# Pest spread detection across zone boundaries
./tools/sim-scenario.sh pest

# Mesh self-healing after node failure
./tools/sim-scenario.sh partition

# Governance proposal and voting flow
./tools/sim-scenario.sh governance

# 1000-node stress benchmark
./tools/sim-scenario.sh stress
```

### Deploy to Hardware

```bash
# Flash ESP32-S3 node
./tools/flash.sh /dev/ttyUSB0 --zone 3 --node-index 7

# Deploy gateway to Raspberry Pi
./tools/deploy-gateway.sh --host raspberrypi.local

# Build and deploy dashboard
./tools/build-dashboard.sh --deploy raspberrypi.local
```

### Generate Terrain Data

```bash
# Synthetic Vermont Green Mountains terrain
python3 tools/generate-terrain.py --zones 10 --output terrain.json
```

## Performance

Benchmarked on standard CI hardware:

| Metric | Result |
|--------|--------|
| 1,000-node mesh convergence | **136 ms** |
| 50-node, 100-tick throughput | 2.3 ms |
| Vector graph training (200 ticks, 50 nodes) | 1.7 ms |
| Dilithium signature creation | 56 μs |
| Dilithium signature verification | 46 μs |
| WASM verifier binary size | 141 KB |
| Dashboard PWA build | 336 KB (gzipped: 107 KB) |

### Target Hardware Specifications

| Parameter | Target |
|-----------|--------|
| Node memory footprint | < 512 KB RAM, < 4 MB flash |
| Average power consumption | < 50 mW (solar sustainable) |
| Single-hop latency | < 500 ms |
| 10-hop end-to-end latency | < 10 seconds |
| Node boot time | < 3 seconds |
| Partition recovery time | < 60 seconds |
| Operating temperature | -40°C to +60°C |
| Hardware cost per node | < $25 at scale |

## Testing Strategy

Verdant uses **London School TDD** across all layers — mock collaborators, verify interactions.

| Layer | Framework | Tests |
|-------|-----------|-------|
| Rust unit + integration | `#[test]` + `mockall` + `proptest` | 280 |
| TypeScript components | Vitest + Testing Library + ts-mockito | 17 |
| E2E | Playwright | 7 scenarios |
| Benchmarks | Criterion | 5 benchmarks |
| Fuzz | cargo-fuzz / libfuzzer | 2 targets |

## CI Pipeline

The GitHub Actions pipeline runs on every PR:

1. `cargo fmt --check` — formatting
2. `cargo clippy -- -D warnings` — linting
3. `cargo test --workspace` — all Rust tests
4. `npm test` + `npm run build` — UI tests and build
5. `wasm-pack build` + size check — WASM verification
6. 5-minute fuzz budget (main branch only)

## Project Structure

```
verdant/
├── crates/
│   ├── verdant-core/          # Shared types & traits
│   ├── verdant-vector/        # Vector graph learning
│   ├── verdant-qudag/         # QuDAG protocol & crypto
│   │   └── fuzz/              # Fuzz targets
│   ├── verdant-mesh/          # Mesh networking
│   ├── verdant-sense/         # Sensor abstraction
│   ├── verdant-safla/         # Self-healing feedback loop
│   ├── verdant-firmware/      # ESP32-S3 firmware
│   ├── verdant-gateway/       # Raspberry Pi gateway
│   ├── verdant-robotics/      # Agentic robotics
│   ├── verdant-sim/           # Network simulator
│   │   ├── tests/             # Integration tests
│   │   └── benches/           # Performance benchmarks
│   └── verdant-qudag-wasm/    # WASM bridge
├── ui/
│   ├── src/
│   │   ├── components/        # React feature components
│   │   ├── hooks/             # Data hooks (port-based)
│   │   ├── lib/               # Types, ports, adapters
│   │   └── workers/           # Service worker
│   └── e2e/                   # Playwright E2E tests
├── tools/
│   ├── flash.sh               # ESP32 flashing
│   ├── deploy-gateway.sh      # Pi deployment
│   ├── build-dashboard.sh     # PWA build & deploy
│   ├── sim-scenario.sh        # Simulation runner
│   └── generate-terrain.py    # Vermont DEM processing
├── plans/
│   ├── sparc.md               # System specification
│   ├── adrs/                  # Architecture decisions
│   └── ddd/                   # Domain model & testing
└── .github/workflows/ci.yml   # CI pipeline
```

## Architecture Decisions

| ADR | Decision | Rationale |
|-----|----------|-----------|
| 001 | Rust `no_std` with `esp-hal` | Memory safety on embedded without runtime overhead |
| 002 | QuDAG post-quantum protocol | Quantum resistance with DAG efficiency (no blockchain) |
| 003 | Vector graph learning | Fixed-capacity incremental learning within 200 KB RAM |
| 004 | Decentralized governance (DAA) | Town meeting-style democracy, zone-based voting |
| 005 | redb for gateway storage | Pure Rust, ACID, crash-safe, single-file, low-resource |
| 006 | Offline-first PWA | Works without connectivity, syncs when available |
| 007 | London School TDD | Mock collaborators, verify interactions, port-based testing |
| 008 | Terrain-aware mesh routing | Distance-vector with RF propagation cost model |
| 009 | SAFLA self-healing protocol | Autonomous health monitoring and topology optimization |
| 010 | Agentic robotics integration | GPS-denied navigation, swarm coordination |
| 011 | Data sovereignty and privacy | No PII storage, presence data stays local |

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE) at your option.

---

<p align="center">
  <i>Built for Vermont's forests, farms, and communities.</i><br/>
  <i>No cloud. No cameras. No subscriptions. Just the land.</i>
</p>
