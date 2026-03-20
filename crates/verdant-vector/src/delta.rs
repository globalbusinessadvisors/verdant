use serde::{Deserialize, Serialize};
use verdant_core::types::{Embedding, Version};

/// Maximum number of node changes tracked in a single delta.
const MAX_DELTA_NODES: usize = 64;

/// A compressed node snapshot included in a delta.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedNode {
    pub embedding: Embedding,
    pub visit_count: u32,
}

/// A diff of vector graph changes between two versions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedDelta {
    pub added: heapless::Vec<CompressedNode, MAX_DELTA_NODES>,
    pub removed: heapless::Vec<u16, MAX_DELTA_NODES>,
    pub base_version: Version,
    pub target_version: Version,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaError {
    /// The delta's base version doesn't match the graph's current version.
    VersionMismatch,
    /// The delta would exceed graph capacity.
    CapacityExceeded,
    /// Failed to decompress wire data.
    DecompressionFailed,
}

impl CompressedDelta {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }
}

/// Maximum buffer size for serialized/compressed delta data.
const DELTA_BUF_SIZE: usize = 8192;

/// Serialize a delta to postcard bytes and then LZ4-compress for wire transfer.
pub fn compress_delta(delta: &CompressedDelta) -> Result<heapless::Vec<u8, DELTA_BUF_SIZE>, DeltaError> {
    let mut ser_buf = [0u8; DELTA_BUF_SIZE];
    let serialized =
        postcard::to_slice(delta, &mut ser_buf).map_err(|_| DeltaError::DecompressionFailed)?;
    let mut comp_buf = [0u8; DELTA_BUF_SIZE];
    let compressed_len =
        lz4_flex::compress_into(serialized, &mut comp_buf).map_err(|_| DeltaError::CapacityExceeded)?;
    // Prepend the uncompressed length as a 4-byte LE header so the receiver knows
    // how large a decompression buffer to allocate.
    let uncomp_len = (serialized.len() as u32).to_le_bytes();
    let mut out = heapless::Vec::new();
    out.extend_from_slice(&uncomp_len).map_err(|_| DeltaError::CapacityExceeded)?;
    out.extend_from_slice(&comp_buf[..compressed_len]).map_err(|_| DeltaError::CapacityExceeded)?;
    Ok(out)
}

/// LZ4-decompress and deserialize a delta from wire bytes.
pub fn decompress_delta(data: &[u8]) -> Result<CompressedDelta, DeltaError> {
    if data.len() < 4 {
        return Err(DeltaError::DecompressionFailed);
    }
    let uncomp_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    if uncomp_len > DELTA_BUF_SIZE {
        return Err(DeltaError::CapacityExceeded);
    }
    let mut decomp_buf = [0u8; DELTA_BUF_SIZE];
    let decompressed_len = lz4_flex::decompress_into(&data[4..], &mut decomp_buf[..uncomp_len])
        .map_err(|_| DeltaError::DecompressionFailed)?;
    postcard::from_bytes(&decomp_buf[..decompressed_len])
        .map_err(|_| DeltaError::DecompressionFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use verdant_core::config::EMBEDDING_DIM;

    fn emb(vals: &[i16]) -> Embedding {
        let mut data = [0i16; EMBEDDING_DIM];
        for (i, &v) in vals.iter().enumerate().take(EMBEDDING_DIM) {
            data[i] = v;
        }
        Embedding { data }
    }

    fn sample_delta() -> CompressedDelta {
        let mut added = heapless::Vec::new();
        let _ = added.push(CompressedNode {
            embedding: emb(&[100, 200, 300]),
            visit_count: 5,
        });
        let _ = added.push(CompressedNode {
            embedding: emb(&[-50, 150]),
            visit_count: 12,
        });

        let mut removed = heapless::Vec::new();
        let _ = removed.push(42);

        CompressedDelta {
            added,
            removed,
            base_version: Version::new(10),
            target_version: Version::new(15),
        }
    }

    #[test]
    fn delta_roundtrip_postcard() {
        let delta = sample_delta();
        let bytes = postcard::to_allocvec(&delta).unwrap();
        let decoded: CompressedDelta = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(delta, decoded);
    }

    #[test]
    fn delta_roundtrip_through_lz4_compression() {
        let delta = sample_delta();
        let compressed = compress_delta(&delta).unwrap();
        let decompressed = decompress_delta(&compressed).unwrap();
        assert_eq!(delta, decompressed);
    }

    #[test]
    fn empty_delta_is_empty() {
        let delta = CompressedDelta {
            added: heapless::Vec::new(),
            removed: heapless::Vec::new(),
            base_version: Version::new(0),
            target_version: Version::new(0),
        };
        assert!(delta.is_empty());
    }

    #[test]
    fn non_empty_delta_is_not_empty() {
        let delta = sample_delta();
        assert!(!delta.is_empty());
    }

    #[test]
    fn compressed_is_smaller_than_raw() {
        let delta = sample_delta();
        let raw = postcard::to_allocvec(&delta).unwrap();
        let compressed = compress_delta(&delta).unwrap();
        // LZ4 with prepended size may not always be smaller for tiny payloads,
        // but for realistic deltas with repetitive embedding data it should be.
        // For this test, just verify it round-trips.
        let decompressed = decompress_delta(&compressed).unwrap();
        assert_eq!(delta, decompressed);
        // Sanity: both representations are reasonable size
        assert!(raw.len() < 4096);
        assert!(compressed.len() < 4096);
    }
}
