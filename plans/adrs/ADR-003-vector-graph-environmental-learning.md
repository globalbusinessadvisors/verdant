# ADR-003: Vector Graph for Environmental Learning

**Status:** Accepted
**Date:** 2026-03-20
**Deciders:** Verdant Core Team
**SPARC Refs:** FR-N3, FR-I1, FR-I2, FR-I5, FR-I6, G2, G7

## Context

Each node must learn what "normal" looks and sounds like for its exact location over months and years, then detect anomalies (pest infestations, flood precursors, fire indicators) against that learned baseline. The learning system must operate within 200 KB of RAM and execute in ~80M CPU cycles per wake cycle on an ESP32 without FPU.

Candidate approaches:
1. **Rule-based thresholds** — simple but brittle; requires manual tuning per location
2. **Classical ML (decision trees, SVM)** — requires training data we don't have at deployment
3. **Autoencoder neural network** — effective but too large for ESP32 memory/compute
4. **Vector graph with incremental learning** — fixed memory, no training phase, learns from first day

## Decision

Use a **self-learning vector graph** — a fixed-capacity graph of embedding vectors with similarity edges, seasonal centroids, and incremental merge/evict operations.

## Rationale

- **No training phase required.** The graph starts empty and populates itself from real observations. By day 90, it has a rich model of the local environment.
- **Fixed memory footprint.** Max 2048 nodes × ~100 bytes = ~200 KB. When full, least-visited nodes are evicted (LRU). Memory never grows unboundedly.
- **Seasonal awareness.** Centroids are bucketed by week-of-year, so "normal February" is different from "normal July." Anomalies are detected against the right seasonal baseline.
- **Compressed deltas for propagation.** When a node learns a new threat pattern (e.g., emerald ash borer RF signature), it can compute a compressed delta of changed graph nodes and propagate it to neighbors — enabling mesh-wide collective learning.
- **Fixed-point math.** Using `micromath` and quantized embeddings (i8 or i16), all vector operations fit within the ESP32's integer ALU. No FPU required.

## Consequences

- **Positive:** Self-calibrating, fixed memory, compressible deltas, runs on ESP32
- **Negative:** Lower precision than floating-point models; requires careful quantization; anomaly detection is less expressive than deep learning
- **Mitigated by:** Using cosine similarity (which is scale-invariant and works well with quantized vectors); combining RF features with environmental sensor features for richer embeddings

## London School TDD Approach

```rust
// The VectorGraph is a pure domain object — no I/O dependencies.
// However, it collaborates with:
//   - EmbeddingProjector (transforms raw features into fixed-dim vectors)
//   - SeasonalClock (determines which seasonal bucket to update)
//   - DeltaCompressor (serializes changes for propagation)
//
// London School: mock these collaborators to test graph behavior in isolation.

pub trait EmbeddingProjector {
    fn project(&self, raw_features: &[f32]) -> Embedding;
}

pub trait SeasonalClock {
    fn current_slot(&self) -> SeasonSlot;
}

pub trait DeltaCompressor {
    fn compress(&self, delta: &GraphDelta) -> Result<CompressedDelta, CompressionError>;
    fn decompress(&self, data: &[u8]) -> Result<GraphDelta, CompressionError>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn update_merges_into_nearest_node_when_below_threshold() {
        // GIVEN: a graph with one existing node at embedding [1.0, 0.0, 0.0]
        let mut graph = VectorGraph::new(2048);
        let existing = Embedding::new(&[1.0, 0.0, 0.0]);
        graph.insert(existing.clone());

        // AND: a seasonal clock returning a fixed slot
        let mut mock_clock = MockSeasonalClock::new();
        mock_clock.expect_current_slot()
            .returning(|| SeasonSlot::new(3, 12)); // march week 3

        // WHEN: we update with a very similar embedding (below merge threshold)
        let similar = Embedding::new(&[0.99, 0.01, 0.0]);
        graph.update(similar, &mock_clock);

        // THEN: graph still has 1 node (merged, not inserted)
        assert_eq!(graph.node_count(), 1);
        // AND: the node's visit count increased
        assert_eq!(graph.nodes()[0].visit_count, 2);
    }

    #[test]
    fn update_inserts_new_node_when_above_threshold() {
        let mut graph = VectorGraph::new(2048);
        let existing = Embedding::new(&[1.0, 0.0, 0.0]);
        graph.insert(existing);

        let mut mock_clock = MockSeasonalClock::new();
        mock_clock.expect_current_slot()
            .returning(|| SeasonSlot::new(3, 12));

        // WHEN: we update with a very different embedding
        let different = Embedding::new(&[0.0, 1.0, 0.0]);
        graph.update(different, &mock_clock);

        // THEN: graph has 2 nodes
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn update_evicts_least_visited_when_at_capacity() {
        let mut graph = VectorGraph::new(3); // tiny capacity for test
        let mut mock_clock = MockSeasonalClock::new();
        mock_clock.expect_current_slot()
            .returning(|| SeasonSlot::new(6, 1));

        // Fill to capacity with 3 distinct embeddings
        graph.insert_with_visits(Embedding::new(&[1.0, 0.0, 0.0]), 100);
        graph.insert_with_visits(Embedding::new(&[0.0, 1.0, 0.0]), 1); // least visited
        graph.insert_with_visits(Embedding::new(&[0.0, 0.0, 1.0]), 50);

        // WHEN: inserting a 4th distinct embedding
        let new_emb = Embedding::new(&[0.5, 0.5, 0.0]);
        graph.update(new_emb.clone(), &mock_clock);

        // THEN: still at capacity
        assert_eq!(graph.node_count(), 3);
        // AND: the least-visited node was evicted
        assert!(graph.nearest_neighbor(&Embedding::new(&[0.0, 1.0, 0.0])).distance > 0.5);
    }

    #[test]
    fn compute_delta_asks_compressor_to_compress_changes() {
        let mut graph = VectorGraph::new(2048);
        let mut mock_compressor = MockDeltaCompressor::new();

        // Insert some nodes, advancing the version
        graph.insert(Embedding::new(&[1.0, 0.0, 0.0]));
        graph.insert(Embedding::new(&[0.0, 1.0, 0.0]));
        let version_after_insert = graph.version();

        // Insert more
        graph.insert(Embedding::new(&[0.0, 0.0, 1.0]));

        // THEN: compressor receives a delta with 1 added node
        mock_compressor.expect_compress()
            .withf(|delta| delta.added.len() == 1 && delta.removed.is_empty())
            .times(1)
            .returning(|_| Ok(CompressedDelta::test_data()));

        graph.compute_delta(version_after_insert, &mock_compressor);
    }

    #[test]
    fn anomaly_score_is_high_when_embedding_far_from_seasonal_baseline() {
        let mut graph = VectorGraph::new(2048);
        let mut mock_clock = MockSeasonalClock::new();
        let march_slot = SeasonSlot::new(3, 12);
        mock_clock.expect_current_slot().returning(move || march_slot);

        // Train a seasonal baseline for march
        for _ in 0..100 {
            let normal = Embedding::new(&[1.0, 0.0, 0.0]);
            graph.update(normal, &mock_clock);
        }

        // WHEN: we score an embedding that's orthogonal to the baseline
        let anomalous = Embedding::new(&[0.0, 1.0, 0.0]);
        let score = graph.anomaly_score(&anomalous, &mock_clock);

        // THEN: anomaly score exceeds threshold
        assert!(score > 0.85);
    }
}
```
