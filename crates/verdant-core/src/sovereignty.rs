use serde::{Deserialize, Serialize};

use crate::types::ZoneId;

/// Data sovereignty tier classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataTier {
    /// Never leaves the node/property.
    Local,
    /// Shared within a zone, encrypted with zone key, opt-in.
    ZoneShared,
    /// Mesh-wide, anonymous, freely transmittable.
    MeshWide,
}

/// Classifies a data type into a sovereignty tier.
pub trait DataClassifier {
    fn classify(&self) -> DataTier;
}

/// Tier 1: Property-local data. **NEVER** leaves the node.
///
/// This type deliberately does **not** implement [`serde::Serialize`].
/// Any attempt to serialize a `Local<T>` for mesh transport will fail at
/// compile time. This is the strongest privacy guarantee Verdant provides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Local<T>(T);

impl<T> Local<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn value(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

/// Tier 2: Zone-shared data. Encrypted with the zone's shared key. Opt-in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneEncrypted<T: Serialize> {
    pub inner: T,
    pub zone: ZoneId,
}

impl<T: Serialize> DataClassifier for ZoneEncrypted<T> {
    fn classify(&self) -> DataTier {
        DataTier::ZoneShared
    }
}

/// Tier 3: Mesh-wide anonymous data. Freely transmittable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshPublic<T: Serialize>(pub T);

impl<T: Serialize> DataClassifier for MeshPublic<T> {
    fn classify(&self) -> DataTier {
        DataTier::MeshWide
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_wraps_and_unwraps() {
        let local = Local::new(42u32);
        assert_eq!(*local.value(), 42);
        assert_eq!(local.into_inner(), 42);
    }

    #[test]
    fn zone_encrypted_classifies_as_zone_shared() {
        let enc = ZoneEncrypted {
            inner: 123u32,
            zone: ZoneId([1, 2, 3, 4]),
        };
        assert_eq!(enc.classify(), DataTier::ZoneShared);
    }

    #[test]
    fn mesh_public_classifies_as_mesh_wide() {
        let pub_data = MeshPublic(99u32);
        assert_eq!(pub_data.classify(), DataTier::MeshWide);
    }

    #[test]
    fn mesh_public_serializes() {
        let pub_data = MeshPublic(42u32);
        let bytes = postcard::to_allocvec(&pub_data).unwrap();
        let decoded: MeshPublic<u32> = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(pub_data, decoded);
    }

    #[test]
    fn zone_encrypted_serializes() {
        let enc = ZoneEncrypted {
            inner: 42u32,
            zone: ZoneId([1, 2, 3, 4]),
        };
        let bytes = postcard::to_allocvec(&enc).unwrap();
        let decoded: ZoneEncrypted<u32> = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(enc, decoded);
    }
}
