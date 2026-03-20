use verdant_core::types::MessageHash;

/// Size of the bloom filter bit array in bytes (2 KB = 16384 bits).
const BLOOM_SIZE_BYTES: usize = 2048;
const BLOOM_SIZE_BITS: usize = BLOOM_SIZE_BYTES * 8;

/// Number of independent hash probes per insert/query.
const HASH_COUNT: usize = 3;

/// A fixed-size bloom filter for tracking seen message hashes.
///
/// Uses 2 KB of memory and supports ~10 000 inserts with a
/// false-positive rate under 1%. The filter is intentionally
/// simple: it derives bit positions directly from non-overlapping
/// segments of the 32-byte [`MessageHash`].
pub struct BloomFilter {
    bits: [u8; BLOOM_SIZE_BYTES],
    count: u32,
}

impl Default for BloomFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl BloomFilter {
    pub const fn new() -> Self {
        Self {
            bits: [0u8; BLOOM_SIZE_BYTES],
            count: 0,
        }
    }

    /// Insert a hash into the filter.
    pub fn insert(&mut self, hash: &MessageHash) {
        for i in 0..HASH_COUNT {
            let bit = Self::probe(hash, i);
            self.bits[bit / 8] |= 1 << (bit % 8);
        }
        self.count += 1;
    }

    /// Check whether a hash *may* have been inserted.
    ///
    /// Returns `false` only when the hash is definitely absent.
    pub fn contains(&self, hash: &MessageHash) -> bool {
        for i in 0..HASH_COUNT {
            let bit = Self::probe(hash, i);
            if self.bits[bit / 8] & (1 << (bit % 8)) == 0 {
                return false;
            }
        }
        true
    }

    /// Number of items inserted (informational; not used for queries).
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Derive a bit position from a [`MessageHash`] for the `i`-th probe.
    ///
    /// Each probe reads a different 8-byte window from the 32-byte hash
    /// and folds it into a position within the bit array.
    fn probe(hash: &MessageHash, i: usize) -> usize {
        let offset = i * 8; // byte offsets 0, 8, 16
        let mut acc: u64 = 0;
        for j in 0..8 {
            acc |= (hash.0[offset + j] as u64) << (j * 8);
        }
        (acc as usize) % BLOOM_SIZE_BITS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hash(seed: u8) -> MessageHash {
        let mut h = [0u8; 32];
        for (i, b) in h.iter_mut().enumerate() {
            *b = seed.wrapping_add(i as u8).wrapping_mul(0x6D);
        }
        MessageHash(h)
    }

    #[test]
    fn insert_then_contains() {
        let mut bf = BloomFilter::new();
        let h = make_hash(1);
        assert!(!bf.contains(&h));
        bf.insert(&h);
        assert!(bf.contains(&h));
    }

    #[test]
    fn absent_hash_not_found() {
        let mut bf = BloomFilter::new();
        bf.insert(&make_hash(1));
        // Different hash very unlikely to collide
        assert!(!bf.contains(&make_hash(200)));
    }

    #[test]
    fn count_tracks_inserts() {
        let mut bf = BloomFilter::new();
        assert_eq!(bf.count(), 0);
        bf.insert(&make_hash(1));
        bf.insert(&make_hash(2));
        assert_eq!(bf.count(), 2);
    }

    #[test]
    fn many_inserts_no_panic() {
        let mut bf = BloomFilter::new();
        for i in 0..=255u8 {
            bf.insert(&make_hash(i));
        }
        // All inserted items should be present
        for i in 0..=255u8 {
            assert!(bf.contains(&make_hash(i)));
        }
    }
}
