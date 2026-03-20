# Verdant Architecture Decision Records

## Index

| ADR | Title | Status | Domain |
|-----|-------|--------|--------|
| [ADR-001](ADR-001-rust-no-std-esp32-firmware.md) | Rust `no_std` for ESP32 Node Firmware | Accepted | Firmware |
| [ADR-002](ADR-002-qudag-post-quantum-protocol.md) | QuDAG Post-Quantum Mesh Protocol | Accepted | Mesh Communication |
| [ADR-003](ADR-003-vector-graph-environmental-learning.md) | Vector Graph for Environmental Learning | Accepted | Environmental Intelligence |
| [ADR-004](ADR-004-decentralized-governance-daa.md) | Decentralized Autonomous Application Governance | Accepted | Governance |
| [ADR-005](ADR-005-redb-gateway-storage.md) | redb for Gateway Storage | Accepted | Gateway |
| [ADR-006](ADR-006-offline-first-pwa-dashboard.md) | Offline-First PWA Dashboard | Accepted | Dashboard UI |
| [ADR-007](ADR-007-london-school-tdd-strategy.md) | London School TDD as Primary Testing Methodology | Accepted | Cross-cutting |
| [ADR-008](ADR-008-mesh-topology-routing.md) | Terrain-Aware Mesh Topology and Routing | Accepted | Mesh Communication |
| [ADR-009](ADR-009-safla-self-healing.md) | SAFLA Self-Aware Feedback Loop Architecture | Accepted | Self-Healing |
| [ADR-010](ADR-010-agentic-robotics-integration.md) | Agentic Robotics Integration | Accepted | Robotics |
| [ADR-011](ADR-011-data-sovereignty-privacy.md) | Data Sovereignty and Privacy Architecture | Accepted | Cross-cutting |

## ADR Format

Each ADR follows the standard structure:
- **Context** — what problem we're solving and what alternatives exist
- **Decision** — what we chose and why
- **Consequences** — trade-offs, positive and negative
- **London School TDD Approach** — concrete Rust/TypeScript test examples showing how to test the decision's implementation using mock-based, outside-in TDD
