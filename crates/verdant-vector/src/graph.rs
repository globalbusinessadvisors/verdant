use verdant_core::config::MAX_GRAPH_NODES;
use verdant_core::traits::{EmbeddingProjector, SeasonalClock};
use verdant_core::types::{Embedding, RawFeatures, SeasonSlot, Version};

use crate::delta::{CompressedDelta, CompressedNode, DeltaError};
use crate::embedding::{cosine_distance, lerp};
use crate::seasonal::SeasonalBaselines;

/// Default merge threshold — embeddings closer than this are merged.
const DEFAULT_MERGE_THRESHOLD: f32 = 0.10;

/// EMA weight for the new sample when merging.
const EMA_ALPHA: f32 = 0.05;

/// A single node in the vector graph.
#[derive(Clone, Debug)]
pub struct GraphNode {
    /// Index-based identity within the graph.
    pub id: u16,
    /// The learned embedding vector.
    pub embedding: Embedding,
    /// Number of observations merged into this node.
    pub visit_count: u32,
    /// Graph version at which this node was last modified.
    pub last_modified: Version,
}

/// Self-learning vector graph — the core environmental intelligence model.
///
/// Maintains a bounded set of embedding nodes, similarity edges (implicit
/// via nearest-neighbor queries), and seasonal baselines. Supports
/// incremental learning, anomaly detection, and compressed delta export.
pub struct VectorGraph {
    nodes: heapless::Vec<GraphNode, MAX_GRAPH_NODES>,
    baselines: SeasonalBaselines,
    version: Version,
    merge_threshold: f32,
    capacity: usize,
    next_id: u16,
}

impl VectorGraph {
    /// Create a new empty graph with the given capacity (capped at `MAX_GRAPH_NODES`).
    pub fn new(capacity: usize) -> Self {
        Self {
            nodes: heapless::Vec::new(),
            baselines: SeasonalBaselines::new(),
            version: Version::new(0),
            merge_threshold: DEFAULT_MERGE_THRESHOLD,
            capacity: capacity.min(MAX_GRAPH_NODES),
            next_id: 0,
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn nodes(&self) -> &[GraphNode] {
        &self.nodes
    }

    pub fn seasonal_baseline(&self, slot: &SeasonSlot) -> Option<&Embedding> {
        self.baselines.get(slot)
    }

    /// Find the nearest existing node to `query` by cosine distance.
    ///
    /// Returns `(node_index, distance)` or `None` if the graph is empty.
    pub fn nearest_neighbor(&self, query: &Embedding) -> Option<(usize, f32)> {
        if self.nodes.is_empty() {
            return None;
        }
        let mut best_idx = 0;
        let mut best_dist = f32::MAX;
        for (i, node) in self.nodes.iter().enumerate() {
            let dist = cosine_distance(&node.embedding, query);
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }
        Some((best_idx, best_dist))
    }

    /// Update the graph with a new embedding observation.
    ///
    /// 1. Find nearest existing node.
    /// 2. If distance < merge_threshold → EMA merge.
    /// 3. Else if capacity available → insert new node.
    /// 4. Else → evict least-visited node, insert new.
    /// 5. Update seasonal centroid.
    pub fn update(&mut self, embedding: Embedding, clock: &impl SeasonalClock) {
        self.version.increment();

        let slot = clock.current_slot();
        self.baselines.update(slot, &embedding);

        if let Some((idx, dist)) = self.nearest_neighbor(&embedding)
            && dist < self.merge_threshold
        {
            // Merge via exponential moving average
            let node = &mut self.nodes[idx];
            node.embedding = lerp(&node.embedding, &embedding, EMA_ALPHA);
            node.visit_count += 1;
            node.last_modified = self.version;
            return;
        }

        // Insert new node
        let new_node = GraphNode {
            id: self.next_id,
            embedding,
            visit_count: 1,
            last_modified: self.version,
        };
        self.next_id = self.next_id.wrapping_add(1);

        if self.nodes.len() < self.capacity {
            let _ = self.nodes.push(new_node);
        } else {
            // Evict least-visited node
            let victim_idx = self
                .nodes
                .iter()
                .enumerate()
                .min_by_key(|(_, n)| n.visit_count)
                .map(|(i, _)| i)
                .unwrap(); // safe: capacity > 0 implies nodes non-empty
            self.nodes[victim_idx] = new_node;
        }
    }

    /// Project raw features into an embedding and update the graph.
    ///
    /// Delegates to `projector` for the feature→embedding transform,
    /// then calls [`Self::update`].
    pub fn encode_and_update(
        &mut self,
        raw: RawFeatures,
        projector: &impl EmbeddingProjector,
        clock: &impl SeasonalClock,
    ) -> Embedding {
        let embedding = projector.project(&raw);
        self.update(embedding.clone(), clock);
        embedding
    }

    /// Compute the anomaly score of an embedding against the seasonal baseline.
    ///
    /// Returns `[0.0, 1.0]` where `0.0` = normal, `1.0` = maximally anomalous.
    /// If no baseline exists for the current season, returns `0.0` (no opinion).
    pub fn anomaly_score(&self, embedding: &Embedding, clock: &impl SeasonalClock) -> f32 {
        let slot = clock.current_slot();
        match self.baselines.get(&slot) {
            Some(baseline) => cosine_distance(embedding, baseline),
            None => 0.0, // no baseline yet — can't judge
        }
    }

    /// Compute a delta of all nodes modified since `since_version`.
    pub fn compute_delta(&self, since_version: Version) -> CompressedDelta {
        let mut added = heapless::Vec::new();
        for node in &self.nodes {
            if node.last_modified > since_version {
                let _ = added.push(CompressedNode {
                    embedding: node.embedding.clone(),
                    visit_count: node.visit_count,
                });
            }
        }

        CompressedDelta {
            added,
            removed: heapless::Vec::new(),
            base_version: since_version,
            target_version: self.version,
        }
    }

    /// Apply a delta received from another node.
    ///
    /// Each node in the delta is inserted or merged into the graph
    /// using the same merge-or-insert logic as [`Self::update`].
    pub fn apply_delta(
        &mut self,
        delta: &CompressedDelta,
        clock: &impl SeasonalClock,
    ) -> Result<(), DeltaError> {
        for cn in &delta.added {
            self.update(cn.embedding.clone(), clock);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use mockall::mock;
    use verdant_core::config::EMBEDDING_DIM;

    mock! {
        pub Clock {}
        impl SeasonalClock for Clock {
            fn current_slot(&self) -> SeasonSlot;
        }
    }

    mock! {
        pub Projector {}
        impl EmbeddingProjector for Projector {
            fn project(&self, raw_features: &RawFeatures) -> Embedding;
        }
    }

    fn emb(vals: &[i16]) -> Embedding {
        let mut data = [0i16; EMBEDDING_DIM];
        for (i, &v) in vals.iter().enumerate().take(EMBEDDING_DIM) {
            data[i] = v;
        }
        Embedding { data }
    }

    fn default_clock() -> MockClock {
        let mut clock = MockClock::new();
        clock
            .expect_current_slot()
            .returning(|| SeasonSlot::new(10));
        clock
    }

    // ---- London School TDD tests ----

    #[test]
    fn update_merges_into_nearest_when_below_threshold() {
        let mut graph = VectorGraph::new(2048);
        let clock = default_clock();

        let e1 = emb(&[1000, 500, 250]);
        graph.update(e1, &clock);
        assert_eq!(graph.node_count(), 1);

        // Very similar embedding — should merge
        let e2 = emb(&[1005, 498, 252]);
        graph.update(e2, &clock);
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.nodes()[0].visit_count, 2);
    }

    #[test]
    fn update_inserts_new_node_when_above_threshold() {
        let mut graph = VectorGraph::new(2048);
        let clock = default_clock();

        graph.update(emb(&[1000, 0, 0, 0]), &clock);

        // Orthogonal embedding — should insert new node
        let mut e2 = [0i16; EMBEDDING_DIM];
        e2[1] = 1000;
        graph.update(Embedding { data: e2 }, &clock);
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn update_evicts_least_visited_at_capacity() {
        let mut graph = VectorGraph::new(3);
        let clock = default_clock();

        // Fill with 3 distinct orthogonal embeddings
        let mut e1 = [0i16; EMBEDDING_DIM];
        e1[0] = 1000;
        let mut e2 = [0i16; EMBEDDING_DIM];
        e2[1] = 1000;
        let mut e3 = [0i16; EMBEDDING_DIM];
        e3[2] = 1000;

        graph.update(Embedding { data: e1 }, &clock);
        // Visit e1 again to increase its count
        graph.update(Embedding { data: e1 }, &clock);
        graph.update(Embedding { data: e2 }, &clock);
        graph.update(Embedding { data: e3 }, &clock);
        assert_eq!(graph.node_count(), 3);

        // e2 has visit_count=1 (least visited, e1 has 2)
        // Insert a 4th distinct embedding
        let mut e4 = [0i16; EMBEDDING_DIM];
        e4[3] = 1000;
        graph.update(Embedding { data: e4 }, &clock);

        // Still at capacity
        assert_eq!(graph.node_count(), 3);

        // e2 or e3 (visit_count=1) should have been evicted
        let counts: std::vec::Vec<u32> = graph.nodes().iter().map(|n| n.visit_count).collect();
        // The new node has visit_count=1, and we should have kept e1 (visit_count=2)
        assert!(counts.contains(&2), "e1 (visit_count=2) should survive eviction");
    }

    #[test]
    fn encode_and_update_delegates_to_projector() {
        let mut graph = VectorGraph::new(2048);
        let clock = default_clock();

        let mut projector = MockProjector::new();
        projector
            .expect_project()
            .times(1)
            .returning(|_| emb(&[500, 600, 700]));

        let raw = RawFeatures {
            data: heapless::Vec::new(),
        };
        let result = graph.encode_and_update(raw, &projector, &clock);
        assert_eq!(result.data[0], 500);
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn encode_and_update_calls_clock() {
        let mut graph = VectorGraph::new(2048);

        let mut clock = MockClock::new();
        clock
            .expect_current_slot()
            .times(1..)
            .returning(|| SeasonSlot::new(25));

        let mut projector = MockProjector::new();
        projector
            .expect_project()
            .returning(|_| emb(&[100]));

        let raw = RawFeatures {
            data: heapless::Vec::new(),
        };
        graph.encode_and_update(raw, &projector, &clock);
    }

    #[test]
    fn anomaly_score_high_when_far_from_baseline() {
        let mut graph = VectorGraph::new(2048);
        let clock = default_clock();

        // Train a baseline
        for _ in 0..50 {
            graph.update(emb(&[1000, 0, 0, 0]), &clock);
        }

        // Query with orthogonal embedding
        let mut anomalous = [0i16; EMBEDDING_DIM];
        anomalous[1] = 1000;
        let score = graph.anomaly_score(&Embedding { data: anomalous }, &clock);
        assert!(score > 0.4, "Expected high anomaly score, got {score}");
    }

    #[test]
    fn anomaly_score_low_when_near_baseline() {
        let mut graph = VectorGraph::new(2048);
        let clock = default_clock();

        for _ in 0..50 {
            graph.update(emb(&[1000, 500, 250]), &clock);
        }

        let score = graph.anomaly_score(&emb(&[990, 505, 248]), &clock);
        assert!(score < 0.1, "Expected low anomaly score, got {score}");
    }

    #[test]
    fn anomaly_score_zero_when_no_baseline() {
        let graph = VectorGraph::new(2048);
        let clock = default_clock();
        let score = graph.anomaly_score(&emb(&[1000, 0, 0, 0]), &clock);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn compute_delta_captures_new_nodes() {
        let mut graph = VectorGraph::new(2048);
        let clock = default_clock();

        graph.update(emb(&[100, 0, 0, 0]), &clock);
        let v1 = graph.version();

        let mut e2 = [0i16; EMBEDDING_DIM];
        e2[1] = 100;
        graph.update(Embedding { data: e2 }, &clock);

        let delta = graph.compute_delta(v1);
        assert!(!delta.is_empty());
        assert!(!delta.added.is_empty());
        assert_eq!(delta.base_version, v1);
        assert_eq!(delta.target_version, graph.version());
    }

    #[test]
    fn compute_delta_empty_when_no_changes() {
        let mut graph = VectorGraph::new(2048);
        let clock = default_clock();
        graph.update(emb(&[100]), &clock);

        let v = graph.version();
        let delta = graph.compute_delta(v);
        assert!(delta.is_empty());
    }

    #[test]
    fn apply_delta_adds_nodes() {
        let mut graph = VectorGraph::new(2048);
        let clock = default_clock();

        let mut added = heapless::Vec::new();
        let _ = added.push(CompressedNode {
            embedding: emb(&[100, 200, 300]),
            visit_count: 5,
        });

        let delta = CompressedDelta {
            added,
            removed: heapless::Vec::new(),
            base_version: Version::new(0),
            target_version: Version::new(1),
        };

        graph.apply_delta(&delta, &clock).unwrap();
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn version_increases_on_update() {
        let mut graph = VectorGraph::new(2048);
        let clock = default_clock();
        let v0 = graph.version();
        graph.update(emb(&[100]), &clock);
        assert!(graph.version() > v0);
    }

    #[test]
    fn nearest_neighbor_finds_closest() {
        let mut graph = VectorGraph::new(2048);
        let clock = default_clock();

        let mut e1 = [0i16; EMBEDDING_DIM];
        e1[0] = 1000;
        let mut e2 = [0i16; EMBEDDING_DIM];
        e2[1] = 1000;
        graph.update(Embedding { data: e1 }, &clock);
        graph.update(Embedding { data: e2 }, &clock);

        let query = emb(&[900, 0, 0, 0]); // closest to e1
        let (idx, _) = graph.nearest_neighbor(&query).unwrap();
        assert_eq!(graph.nodes()[idx].embedding.data[0], 1000);
    }
}
