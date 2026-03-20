use proptest::prelude::*;
use verdant_core::config::EMBEDDING_DIM;
use verdant_core::types::{Embedding, SeasonSlot};
use verdant_vector::embedding::cosine_distance;
use verdant_vector::graph::VectorGraph;

/// Generate an arbitrary embedding with values in [-1000, 1000].
fn arb_embedding() -> impl Strategy<Value = Embedding> {
    proptest::array::uniform(prop::num::i16::ANY.prop_map(|v| (v % 1000) as i16))
        .prop_map(|data: [i16; EMBEDDING_DIM]| Embedding { data })
}

/// A SeasonalClock that always returns a fixed slot.
struct FixedClock(SeasonSlot);
impl verdant_core::traits::SeasonalClock for FixedClock {
    fn current_slot(&self) -> SeasonSlot {
        self.0
    }
}

proptest! {
    /// The graph must never exceed its configured capacity.
    #[test]
    fn graph_never_exceeds_capacity(
        embeddings in proptest::collection::vec(arb_embedding(), 0..200)
    ) {
        let mut graph = VectorGraph::new(64); // small capacity for fast tests
        let clock = FixedClock(SeasonSlot::new(10));

        for emb in embeddings {
            graph.update(emb, &clock);
        }

        prop_assert!(graph.node_count() <= 64);
    }

    /// Anomaly score must always be in [0.0, 1.0].
    #[test]
    fn anomaly_score_bounded_0_to_1(
        a in arb_embedding(),
        b in arb_embedding()
    ) {
        let score = cosine_distance(&a, &b);
        prop_assert!(score >= 0.0, "Score {} is negative", score);
        prop_assert!(score <= 1.0, "Score {} exceeds 1.0", score);
    }

    /// Inserting the same embedding N times must result in exactly 1 node.
    #[test]
    fn identical_embeddings_merge(
        base in arb_embedding(),
        count in 2..50usize
    ) {
        let mut graph = VectorGraph::new(2048);
        let clock = FixedClock(SeasonSlot::new(5));

        for _ in 0..count {
            graph.update(base.clone(), &clock);
        }

        prop_assert_eq!(graph.node_count(), 1);
        prop_assert_eq!(graph.nodes()[0].visit_count, count as u32);
    }

    /// Graph version must be monotonically non-decreasing.
    #[test]
    fn version_monotonically_increases(
        embeddings in proptest::collection::vec(arb_embedding(), 1..50)
    ) {
        let mut graph = VectorGraph::new(2048);
        let clock = FixedClock(SeasonSlot::new(10));
        let mut prev = graph.version();

        for emb in embeddings {
            graph.update(emb, &clock);
            let new = graph.version();
            prop_assert!(new >= prev, "Version decreased: {:?} -> {:?}", prev, new);
            prev = new;
        }
    }

    /// Cosine distance must be symmetric: d(a,b) == d(b,a).
    #[test]
    fn cosine_distance_symmetric(a in arb_embedding(), b in arb_embedding()) {
        let d1 = cosine_distance(&a, &b);
        let d2 = cosine_distance(&b, &a);
        prop_assert!(
            (d1 - d2).abs() < 0.0001,
            "Asymmetric: d(a,b)={}, d(b,a)={}",
            d1, d2
        );
    }
}
