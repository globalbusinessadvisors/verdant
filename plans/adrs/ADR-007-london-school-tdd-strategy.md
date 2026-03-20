# ADR-007: London School TDD as Primary Testing Methodology

**Status:** Accepted
**Date:** 2026-03-20
**Deciders:** Verdant Core Team
**SPARC Refs:** All crates, all layers

## Context

Verdant spans from bare-metal ESP32 firmware to a TypeScript PWA dashboard. Testing must work across:
- `no_std` Rust on embedded (no filesystem, no network, no OS)
- `std` Rust on gateway (async I/O, database, WebSocket)
- TypeScript in browser (React components, Service Workers, WASM)

The codebase has deep dependency chains: firmware → sensing → vector graph → mesh → crypto → flash storage. Traditional integration testing requires real hardware (ESP32 boards, radios, sensors), making fast feedback impossible during development.

Candidate approaches:
1. **Detroit/Chicago School TDD** (state-based testing) — test by asserting final state; requires real or faked collaborators to produce realistic state
2. **London School TDD** (interaction-based testing) — test by verifying interactions with mock collaborators; enables testing each layer independently
3. **No TDD** (write tests after) — faster initial development but higher defect rate at integration
4. **Property-based testing only** — excellent for algorithmic correctness but doesn't verify wiring between components

## Decision

Use **London School TDD** as the primary methodology, supplemented by **property-based tests** for algorithmic modules and **integration tests** for end-to-end verification in the simulator.

## Rationale

### Why London School for Verdant

1. **Hardware independence.** London School's mock-heavy approach lets us test firmware logic on x86 without an ESP32. By mocking `CsiCapture`, `FlashStorage`, `MeshTransport`, etc., the node's sense-learn-detect-communicate loop is fully testable on a developer laptop.

2. **Outside-in development.** We start from the acceptance test ("node broadcasts anomaly when score exceeds threshold") and work inward, discovering the interfaces we need. This produces minimal, well-defined trait boundaries between crates.

3. **Fast feedback.** A full `cargo test` run across all crates takes <10 seconds (no hardware, no I/O). This enables red-green-refactor cycles of <30 seconds.

4. **Interaction verification catches wiring bugs.** The most common bugs in a mesh system aren't algorithmic — they're wiring bugs: "node forgot to forward this message type" or "gateway didn't persist this event." London School tests verify that Component A *actually called* Component B with the right arguments.

5. **Trait-driven design.** London School naturally produces a codebase organized around traits (interfaces), which is exactly what we need for `no_std`/`std` portability. The same `MeshTransport` trait has an ESP32 WiFi implementation and a test mock implementation.

### Where London School Does NOT Apply

- **VectorGraph internals** — the graph's nearest-neighbor, merge, and eviction algorithms are pure math. These use property-based testing (`proptest`) to verify invariants: "graph never exceeds capacity," "merge preserves centroid accuracy within epsilon."
- **QuDAG DAG validation** — DAG parent verification is a pure function. Property tests verify: "valid DAGs are accepted," "orphaned messages are rejected," "forked DAGs are detected."
- **End-to-end scenarios** — `verdant-sim` runs 100+ virtual nodes and verifies system-level behavior (flood detection, mesh convergence, governance voting). These are integration tests, not unit tests.

## The Three-Layer Testing Pyramid

```
                    ┌─────────────┐
                    │  E2E / Sim  │  verdant-sim: 100+ virtual nodes
                    │  (few, slow)│  Historical flood replay
                    ├─────────────┤  Governance voting across zones
                   ╱               ╲
                  ╱   Property-     ╲  VectorGraph invariants
                 ╱    Based Tests    ╲  DAG validation properties
                ╱     (moderate)      ╲  Routing convergence proofs
               ├───────────────────────┤
              ╱                         ╲
             ╱   London School Mocks     ╲  Node loop interactions
            ╱   (many, fast, <10s total)  ╲  Mesh protocol handshakes
           ╱                               ╲  Gateway API → Store
          ╱                                 ╲  SAFLA consensus flow
         ╱                                   ╲  Dashboard → Gateway
        └─────────────────────────────────────┘
```

## Implementation Guidelines

### Rust: `mockall` crate

```rust
// 1. Define the trait in the domain crate (no mock dependency)
pub trait MeshTransport {
    fn send(&mut self, msg: &MeshFrame) -> Result<(), TransportError>;
    fn receive(&mut self) -> Result<Option<MeshFrame>, TransportError>;
}

// 2. In the test module, use mockall to generate mocks
#[cfg(test)]
use mockall::{automock, predicate::*};

#[cfg(test)]
#[automock]
trait MeshTransport { /* ... */ }

// 3. Test interactions
#[test]
fn node_forwards_relay_message_when_ttl_positive() {
    let mut mock = MockMeshTransport::new();

    // Verify the interaction: send() is called with a decremented TTL
    mock.expect_send()
        .withf(|msg| msg.ttl == 4)  // original was 5, decremented
        .times(1)
        .returning(|_| Ok(()));

    let mut node = RelayNode::new(mock);
    node.handle_relay(message_with_ttl(5));
}
```

### TypeScript: `ts-mockito` or `jest.fn()`

```typescript
// 1. Define the port (interface)
interface AlertNotifier {
  notify(alert: Alert): Promise<void>;
  dismiss(alertId: string): Promise<void>;
}

// 2. Mock and verify
it("notifies when flood event confirmed", async () => {
  const mockNotifier = mock<AlertNotifier>();
  when(mockNotifier.notify(anything())).thenResolve();

  const alertManager = new AlertManager(instance(mockNotifier));
  await alertManager.handleEvent(confirmedFloodEvent());

  verify(mockNotifier.notify(
    objectContaining({ type: "flood", severity: "critical" })
  )).once();
});
```

### Red-Green-Refactor Cycle

```
1. RED:   Write a failing test that describes the desired interaction
          "when node detects anomaly, it should call transport.send with AnomalyReport"

2. GREEN: Write the minimal production code to make the test pass
          (implement the if-statement and the send call)

3. REFACTOR: Clean up without changing behavior
             (extract method, rename, simplify)

4. REPEAT: Next interaction in the outside-in chain
           "when transport.send succeeds, node should call storage.log_event"
```

### Outside-In Development Order

```
For each feature (e.g., "flood detection and alerting"):

1. Start at the OUTER boundary (acceptance test in verdant-sim):
   "When upstream node detects rising water, downstream node receives alert within 60s"

2. Work INWARD to the node loop:
   "When anomaly_score > threshold AND category == Flood, call transport.broadcast(FloodAlert)"

3. Work INWARD to SAFLA consensus:
   "When 3+ neighbors corroborate flood anomaly, emit ConfirmedEvent::Flood"

4. Work INWARD to vector graph:
   "When soil_moisture spikes AND pressure drops, anomaly_score > 0.85"

5. Work INWARD to the sensor layer:
   "When raw ADC reads above calibration curve, SensorReading.soil_moisture > 0.9"

Each step discovers the trait interface the next layer needs to provide.
```

## Consequences

- **Positive:** Fast test suite (<10s); firmware testable without hardware; natural trait-driven architecture; catches wiring bugs
- **Negative:** Mock-heavy tests can be brittle if traits change frequently; risk of testing the mocks instead of the system; refactoring trait signatures requires updating many tests
- **Mitigated by:** Keeping traits narrow and stable (Single Responsibility); supplementing with property tests for algorithmic code; running E2E sim tests before merge
