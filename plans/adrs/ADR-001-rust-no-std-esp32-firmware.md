# ADR-001: Rust `no_std` for ESP32 Node Firmware

**Status:** Accepted
**Date:** 2026-03-20
**Deciders:** Verdant Core Team
**SPARC Refs:** FR-N1 through FR-N8, NFR-1, NFR-2, NFR-5, NFR-8

## Context

Each Verdant mesh node runs on an ESP32-S3 microcontroller with 512 KB SRAM and 4 MB flash. The firmware must perform WiFi CSI capture, sensor fusion, vector graph learning, mesh protocol participation, post-quantum cryptography, and power management — all within a 50 mW average power budget. The firmware will be deployed to thousands of nodes in remote, physically inaccessible locations where bugs have high remediation cost.

Candidate approaches:
1. **C/C++ with ESP-IDF** — industry standard for ESP32, massive ecosystem
2. **MicroPython** — rapid prototyping, but 10-50x slower, GC pauses
3. **Rust `no_std` with `esp-hal`** — memory safety without runtime overhead
4. **Rust `std` with `esp-idf-svc`** — easier APIs but larger binary, higher RAM

## Decision

Use **Rust `no_std`** with `esp-hal` and `embassy` async executor for all node firmware.

## Rationale

- **Memory safety is non-negotiable at scale.** A use-after-free on one node in a remote forest means a truck roll. Rust eliminates this class of bugs at compile time.
- **Zero-cost abstractions fit the compute budget.** The 400M-cycle-per-wake-cycle budget leaves no room for runtime overhead. Rust's monomorphization and lack of GC mean the firmware runs as fast as equivalent C.
- **`no_std` over `std`** because `esp-idf-svc` (the `std` layer) consumes ~180 KB of additional RAM. With only 512 KB total and 128 KB reserved for WiFi buffers, we cannot afford this.
- **`embassy` async** gives cooperative multitasking without an RTOS thread stack per task. Each `embassy` task costs ~200 bytes vs. ~4 KB for a FreeRTOS thread.
- **Trait-based HAL abstraction** means the vector graph, mesh protocol, and SAFLA crates can be tested on a host machine (x86) by swapping in mock implementations — critical for London School TDD.

## Consequences

- **Positive:** Compile-time memory safety, deterministic performance, testable on host via trait mocks
- **Negative:** Steeper learning curve for contributors; `no_std` ecosystem is smaller; some ESP32 features (e.g., WiFi CSI) require unsafe FFI to ESP-IDF C APIs
- **Mitigated by:** Wrapping all unsafe FFI in `verdant-sense/hal.rs` behind safe trait interfaces; comprehensive London School TDD mocking at trait boundaries

## London School TDD Approach

```rust
// The trait boundary that enables London School testing:
// Firmware depends on abstractions, not hardware.

// In verdant-core/src/traits.rs:
pub trait CsiCapture {
    fn capture(&mut self, duration_ms: u32) -> Result<CsiFrame, SenseError>;
}

pub trait EnvironmentalSensor {
    fn read(&mut self) -> Result<SensorReading, SenseError>;
}

pub trait MeshTransport {
    fn send(&mut self, msg: &MeshFrame) -> Result<(), TransportError>;
    fn receive(&mut self) -> Result<Option<MeshFrame>, TransportError>;
}

pub trait FlashStorage {
    fn read_block(&self, addr: u32, buf: &mut [u8]) -> Result<(), StorageError>;
    fn write_block(&mut self, addr: u32, data: &[u8]) -> Result<(), StorageError>;
}

// In tests (London School: mock collaborators, verify interactions):
#[cfg(test)]
mod tests {
    use mockall::predicate::*;
    use super::*;

    mock! {
        CsiHw {}
        impl CsiCapture for CsiHw {
            fn capture(&mut self, duration_ms: u32) -> Result<CsiFrame, SenseError>;
        }
    }

    mock! {
        Transport {}
        impl MeshTransport for Transport {
            fn send(&mut self, msg: &MeshFrame) -> Result<(), TransportError>;
            fn receive(&mut self) -> Result<Option<MeshFrame>, TransportError>;
        }
    }

    #[test]
    fn node_broadcasts_anomaly_when_score_exceeds_threshold() {
        // GIVEN: a node with a trained vector graph and a mock transport
        let mut mock_csi = MockCsiHw::new();
        let mut mock_transport = MockTransport::new();

        // CSI returns data that will produce an anomaly
        mock_csi.expect_capture()
            .with(eq(5000))
            .returning(|_| Ok(CsiFrame::anomalous_test_data()));

        // THEN: we expect the transport to be called with an anomaly broadcast
        mock_transport.expect_send()
            .withf(|msg| matches!(msg.payload, Payload::AnomalyReport(_)))
            .times(1)
            .returning(|_| Ok(()));

        mock_transport.expect_receive()
            .returning(|| Ok(None));

        // WHEN: the node runs one sense-learn-detect cycle
        let mut node = NodeLoop::new(mock_csi, mock_sensor, mock_transport, graph);
        node.run_cycle();

        // Mock expectations are verified on drop (London School style)
    }
}
```
