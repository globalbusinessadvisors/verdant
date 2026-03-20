# ADR-011: Data Sovereignty and Privacy Architecture

**Status:** Accepted
**Date:** 2026-03-20
**Deciders:** Verdant Core Team
**SPARC Refs:** FR-G5, FR-M4, G8, Constraints (No PII, No cloud)

## Context

Vermont has a deep cultural commitment to privacy and local control. The mesh collects intimate data: wildlife movements, farm conditions, and — in the presence monitoring use case — whether an elderly person is moving through their home. This data must be protected with the same rigor as medical records, but without any centralized infrastructure to protect.

The system must satisfy:
1. Presence data never leaves the property boundary
2. Environmental data is shared only with explicit landowner consent
3. No node knows both sender and recipient of any message
4. No central server exists that can be subpoenaed
5. All data handling is auditable

## Decision

Implement a **three-tier data sovereignty model** with property-local data, zone-encrypted shared data, and mesh-wide anonymous telemetry.

## Design

### Tier 1: Property-Local (Never Leaves)
- Presence detection (RF activity sensing in homes)
- Raw sensor readings
- Per-node vector graph details
- Local alert triggers

**Implementation:** These data types are tagged `Sovereignty::Local` at the type level. The mesh transport layer refuses to serialize or transmit any value with this tag. This is enforced at compile time using Rust's type system (a `Local<T>` wrapper that does not implement `Serialize` for mesh frames).

### Tier 2: Zone-Shared (Opt-in, Encrypted)
- Aggregated environmental readings
- Anomaly scores (not raw data)
- Seasonal baseline summaries
- Credit ledger entries

**Implementation:** Encrypted with the zone's shared key (CRYSTALS-Kyber). Only zone members can decrypt. Landowner opts in per data type via governance interface. Default is opt-out.

### Tier 3: Mesh-Wide (Anonymous, Public)
- Confirmed event notifications (pest, flood, fire)
- Pattern deltas (compressed vector graph updates)
- Governance proposals and votes
- Network health metrics

**Implementation:** Transmitted via QuDAG with onion routing. Messages carry no identifying information beyond the originating zone (not individual node or landowner).

## Rationale

- **Compile-time enforcement** of Tier 1 is the strongest guarantee. If `Local<PresenceReading>` doesn't implement `Serialize` for `MeshFrame`, it is physically impossible for firmware to transmit it — no bug, no misconfiguration, no malicious firmware update can change this without changing the type system.
- **Opt-out default** for Tier 2 respects Vermont's privacy culture. Landowners explicitly choose to share, rather than having to remember to restrict.
- **Anonymity by design** for Tier 3 means even mesh-wide data can't be traced to a specific landowner.

## Consequences

- **Positive:** Strongest possible privacy guarantees; compile-time enforcement; no trust in any third party
- **Negative:** Limits utility of aggregated data for research/policy; makes debugging harder (can't inspect a specific node's state remotely)
- **Mitigated by:** Landowners can voluntarily publish anonymized research datasets via governance vote; debugging requires physical access to the node (which is appropriate for Tier 1 data)

## London School TDD Approach

```rust
// The sovereignty model is enforced at the type level.
// London School tests verify that components INTERACT correctly
// with the sovereignty boundaries.

/// Wrapper that prevents mesh serialization.
/// Local<T> deliberately does NOT implement MeshSerialize.
pub struct Local<T>(T);

impl<T> Local<T> {
    pub fn value(&self) -> &T { &self.0 }
}

// This line would fail to compile:
// impl<T: Serialize> MeshSerialize for Local<T> { ... }
// Because we never write it. Compile-time privacy.

/// Tier 2 data requires zone key for serialization
pub struct ZoneEncrypted<T> {
    inner: T,
    zone_id: ZoneId,
}

pub trait DataClassifier {
    fn classify(&self, data: &SensorOutput) -> DataTier;
}

pub trait ConsentRegistry {
    fn has_consent(&self, zone: ZoneId, data_type: DataType) -> bool;
}

pub trait AnonymizingTransport {
    fn send_anonymous(&mut self, payload: &[u8], dest_zone: ZoneId) -> Result<(), TransportError>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn presence_data_classified_as_local() {
        let classifier = DefaultDataClassifier::new();
        let presence = SensorOutput::Presence(PresenceReading { active: true });

        let tier = classifier.classify(&presence);
        assert_eq!(tier, DataTier::Local);
    }

    #[test]
    fn zone_shared_data_not_sent_without_consent() {
        let mut mock_consent = MockConsentRegistry::new();
        let mut mock_transport = MockAnonymizingTransport::new();

        // GIVEN: zone has NOT consented to share soil moisture
        mock_consent.expect_has_consent()
            .with(eq(zone_a), eq(DataType::SoilMoisture))
            .returning(|_, _| false);

        // THEN: transport is never called
        mock_transport.expect_send_anonymous().times(0);

        let sharer = DataSharer::new(mock_consent, mock_transport);
        let result = sharer.try_share(zone_a, DataType::SoilMoisture, &soil_data);
        assert!(matches!(result, Err(SharingError::NoConsent)));
    }

    #[test]
    fn zone_shared_data_sent_when_consent_given() {
        let mut mock_consent = MockConsentRegistry::new();
        let mut mock_transport = MockAnonymizingTransport::new();

        // GIVEN: zone has consented
        mock_consent.expect_has_consent()
            .with(eq(zone_a), eq(DataType::SoilMoisture))
            .returning(|_, _| true);

        // THEN: transport sends anonymously
        mock_transport.expect_send_anonymous()
            .times(1)
            .returning(|_, _| Ok(()));

        let sharer = DataSharer::new(mock_consent, mock_transport);
        sharer.try_share(zone_a, DataType::SoilMoisture, &soil_data).unwrap();
    }

    #[test]
    fn mesh_wide_events_sent_via_anonymous_transport() {
        let mut mock_transport = MockAnonymizingTransport::new();

        // THEN: confirmed event is sent anonymously (no node ID, just zone)
        mock_transport.expect_send_anonymous()
            .withf(|payload, _| {
                // Verify payload contains no node-level identifiers
                let decoded: ConfirmedEvent = deserialize(payload);
                decoded.source_node.is_none()
                    && decoded.affected_zone.is_some()
            })
            .times(1)
            .returning(|_, _| Ok(()));

        let publisher = EventPublisher::new(mock_transport);
        publisher.publish_confirmed(confirmed_flood_event());
    }

    // Compile-time test: this test verifies that Local<T> cannot be serialized.
    // If someone adds MeshSerialize for Local<T>, this test (and the build) should fail.
    // This is verified by a negative compile test (trybuild crate).
    #[test]
    fn local_data_cannot_be_mesh_serialized() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/compile_fail/local_not_serializable.rs");
    }

    // tests/compile_fail/local_not_serializable.rs:
    // use verdant_core::{Local, MeshSerialize};
    // fn try_serialize(data: Local<PresenceReading>) {
    //     data.mesh_serialize(); // This should fail to compile
    // }
}
```
