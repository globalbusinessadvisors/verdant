use verdant_core::config::EMBEDDING_DIM;
use verdant_core::types::{Embedding, SeasonSlot};

/// Running mean over embeddings for a single seasonal slot.
#[derive(Clone, Debug)]
pub struct RunningMean {
    /// Current mean embedding.
    pub mean: Embedding,
    /// Number of observations folded in.
    pub count: u32,
}

/// Seasonal baselines — one running-mean centroid per week of year.
pub struct SeasonalBaselines {
    slots: [Option<RunningMean>; 52],
}

impl SeasonalBaselines {
    pub const fn new() -> Self {
        // const-init: 52 Nones
        Self { slots: [const { None }; 52] }
    }

    /// Fold a new embedding into the running mean for `slot`.
    pub fn update(&mut self, slot: SeasonSlot, embedding: &Embedding) {
        let idx = slot.week as usize;
        if idx >= 52 {
            return;
        }

        match &mut self.slots[idx] {
            Some(rm) => {
                rm.count += 1;
                let n = rm.count as f32;
                // Incremental mean: mean' = mean + (x - mean) / n
                for i in 0..EMBEDDING_DIM {
                    let diff = (embedding.data[i] as f32) - (rm.mean.data[i] as f32);
                    let new_val = (rm.mean.data[i] as f32) + diff / n;
                    rm.mean.data[i] = new_val.clamp(-32768.0, 32767.0) as i16;
                }
            }
            slot_ref @ None => {
                *slot_ref = Some(RunningMean {
                    mean: embedding.clone(),
                    count: 1,
                });
            }
        }
    }

    /// Get the centroid embedding for a seasonal slot (if any observations exist).
    pub fn get(&self, slot: &SeasonSlot) -> Option<&Embedding> {
        let idx = slot.week as usize;
        if idx >= 52 {
            return None;
        }
        self.slots[idx].as_ref().map(|rm| &rm.mean)
    }

    /// Number of the 52 weekly slots that have at least one observation.
    pub fn coverage(&self) -> u8 {
        self.slots.iter().filter(|s| s.is_some()).count() as u8
    }
}

impl Default for SeasonalBaselines {
    fn default() -> Self {
        Self::new()
    }
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

    #[test]
    fn empty_baselines_has_zero_coverage() {
        let bl = SeasonalBaselines::new();
        assert_eq!(bl.coverage(), 0);
    }

    #[test]
    fn single_update_sets_mean() {
        let mut bl = SeasonalBaselines::new();
        let slot = SeasonSlot::new(10);
        let e = emb(&[100, 200]);
        bl.update(slot, &e);
        let mean = bl.get(&slot).unwrap();
        assert_eq!(mean.data[0], 100);
        assert_eq!(mean.data[1], 200);
        assert_eq!(bl.coverage(), 1);
    }

    #[test]
    fn running_mean_converges() {
        let mut bl = SeasonalBaselines::new();
        let slot = SeasonSlot::new(5);
        bl.update(slot, &emb(&[100]));
        bl.update(slot, &emb(&[200]));
        let mean = bl.get(&slot).unwrap();
        assert_eq!(mean.data[0], 150); // (100+200)/2
    }

    #[test]
    fn running_mean_three_values() {
        let mut bl = SeasonalBaselines::new();
        let slot = SeasonSlot::new(5);
        bl.update(slot, &emb(&[90]));
        bl.update(slot, &emb(&[120]));
        bl.update(slot, &emb(&[150]));
        let mean = bl.get(&slot).unwrap();
        assert_eq!(mean.data[0], 120); // (90+120+150)/3 = 120
    }

    #[test]
    fn different_slots_independent() {
        let mut bl = SeasonalBaselines::new();
        bl.update(SeasonSlot::new(0), &emb(&[100]));
        bl.update(SeasonSlot::new(1), &emb(&[200]));
        assert_eq!(bl.get(&SeasonSlot::new(0)).unwrap().data[0], 100);
        assert_eq!(bl.get(&SeasonSlot::new(1)).unwrap().data[0], 200);
        assert_eq!(bl.coverage(), 2);
    }

    #[test]
    fn get_returns_none_for_empty_slot() {
        let bl = SeasonalBaselines::new();
        assert!(bl.get(&SeasonSlot::new(30)).is_none());
    }
}
