#[allow(unused_imports)]
use micromath::F32Ext; // provides f32::sqrt() in no_std
use verdant_core::config::EMBEDDING_DIM;
use verdant_core::types::Embedding;

/// Compute the dot product of two embeddings using integer arithmetic.
pub fn dot_product(a: &Embedding, b: &Embedding) -> i64 {
    let mut sum: i64 = 0;
    for i in 0..EMBEDDING_DIM {
        sum += (a.data[i] as i64) * (b.data[i] as i64);
    }
    sum
}

/// Compute the squared magnitude of an embedding.
pub fn magnitude_squared(e: &Embedding) -> i64 {
    dot_product(e, e)
}

/// Compute cosine distance between two embeddings.
///
/// Returns a value in `[0.0, 1.0]` where `0.0` means identical
/// direction and `1.0` means orthogonal or opposing.
///
/// Uses integer dot products and `micromath` for the final sqrt.
pub fn cosine_distance(a: &Embedding, b: &Embedding) -> f32 {
    let dot = dot_product(a, b) as f32;
    let mag_a = magnitude_squared(a) as f32;
    let mag_b = magnitude_squared(b) as f32;

    let denom = (mag_a * mag_b).sqrt();
    if denom < 1.0 {
        return 1.0; // one or both are zero vectors
    }

    let similarity = (dot / denom).clamp(-1.0, 1.0);

    // Convert similarity to distance: 0 = identical, 1 = orthogonal
    (1.0 - similarity) * 0.5
}

/// Linear interpolation between two embeddings for EMA merge.
///
/// Returns `a * (1 - alpha) + b * alpha`, quantized back to i16.
/// For the standard EMA merge, `alpha = 0.05` (new sample weight).
pub fn lerp(a: &Embedding, b: &Embedding, alpha: f32) -> Embedding {
    let one_minus_alpha = 1.0 - alpha;
    let mut result = Embedding::zero();
    for i in 0..EMBEDDING_DIM {
        let val = (a.data[i] as f32) * one_minus_alpha + (b.data[i] as f32) * alpha;
        result.data[i] = val.clamp(-32768.0, 32767.0) as i16;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emb(vals: &[i16]) -> Embedding {
        let mut data = [0i16; EMBEDDING_DIM];
        for (i, &v) in vals.iter().enumerate().take(EMBEDDING_DIM) {
            data[i] = v;
        }
        Embedding { data }
    }

    #[test]
    fn dot_product_basic() {
        let a = emb(&[1, 2, 3]);
        let b = emb(&[4, 5, 6]);
        assert_eq!(dot_product(&a, &b), 1 * 4 + 2 * 5 + 3 * 6);
    }

    #[test]
    fn magnitude_squared_basic() {
        let a = emb(&[3, 4]);
        assert_eq!(magnitude_squared(&a), 9 + 16);
    }

    #[test]
    fn cosine_distance_identical() {
        let a = emb(&[100, 200, 300]);
        assert!(cosine_distance(&a, &a) < 0.001);
    }

    #[test]
    fn cosine_distance_orthogonal() {
        let a = emb(&[1000, 0, 0, 0]);
        let mut b_data = [0i16; EMBEDDING_DIM];
        b_data[1] = 1000;
        let b = Embedding { data: b_data };
        let d = cosine_distance(&a, &b);
        assert!((d - 0.5).abs() < 0.01); // orthogonal = 0.5 in our metric
    }

    #[test]
    fn cosine_distance_opposite() {
        let a = emb(&[1000, 0]);
        let b = emb(&[-1000, 0]);
        let d = cosine_distance(&a, &b);
        assert!((d - 1.0).abs() < 0.01);
    }

    #[test]
    fn cosine_distance_zero_vector() {
        let a = emb(&[100, 200]);
        let b = Embedding::zero();
        assert_eq!(cosine_distance(&a, &b), 1.0);
    }

    #[test]
    fn cosine_distance_symmetric() {
        let a = emb(&[100, 200, -300, 400]);
        let b = emb(&[-50, 150, 250, -100]);
        let d1 = cosine_distance(&a, &b);
        let d2 = cosine_distance(&b, &a);
        assert!((d1 - d2).abs() < 0.0001);
    }

    #[test]
    fn lerp_alpha_zero() {
        let a = emb(&[100, 200]);
        let b = emb(&[300, 400]);
        let result = lerp(&a, &b, 0.0);
        assert_eq!(result.data[0], 100);
        assert_eq!(result.data[1], 200);
    }

    #[test]
    fn lerp_alpha_one() {
        let a = emb(&[100, 200]);
        let b = emb(&[300, 400]);
        let result = lerp(&a, &b, 1.0);
        assert_eq!(result.data[0], 300);
        assert_eq!(result.data[1], 400);
    }

    #[test]
    fn lerp_midpoint() {
        let a = emb(&[100, 200]);
        let b = emb(&[300, 400]);
        let result = lerp(&a, &b, 0.5);
        assert_eq!(result.data[0], 200);
        assert_eq!(result.data[1], 300);
    }

    #[test]
    fn lerp_clamps_overflow() {
        let a = emb(&[32000]);
        let b = emb(&[32000]);
        let result = lerp(&a, &b, 0.5);
        assert_eq!(result.data[0], 32000);
    }
}
