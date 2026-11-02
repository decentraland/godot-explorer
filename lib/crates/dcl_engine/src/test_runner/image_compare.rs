//! Pixel comparison of the snapshot tests, on raw image bytes so it needs no engine.

/// How alike two RGB8 buffers are, 1.0 being identical: `1 - rms` of the channel differences.
/// Buffers of different lengths share nothing (0.0).
pub fn rgb_similarity(a: &[u8], b: &[u8]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let squared: u64 = a
        .iter()
        .zip(b)
        .map(|(x, y)| {
            let diff = i64::from(*x) - i64::from(*y);
            (diff * diff) as u64
        })
        .sum();
    let pixels = a.len() as f64 / 3.0;
    let denominator = 3.0 * f64::from(u8::MAX).powi(2) * pixels;
    (1.0 - squared as f64 / denominator).sqrt()
}

/// The per-channel absolute difference, as an image of the same shape.
pub fn rgb_difference(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b).map(|(x, y)| x.abs_diff(*y)).collect()
}

/// The share of RGBA8 pixels whose four channels differ by less than `tolerance` in total
/// (a channel is 0..1); the second value marks the differing pixels red on black.
pub fn matching_pixels(a: &[u8], b: &[u8], tolerance: f32) -> Option<(f32, Vec<u8>)> {
    if a.len() != b.len() || a.is_empty() || !a.len().is_multiple_of(4) {
        return None;
    }
    let mut matching = 0usize;
    let mut diff = Vec::with_capacity(a.len());
    for (pa, pb) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let total: u32 = pa
            .iter()
            .zip(pb)
            .map(|(x, y)| u32::from(x.abs_diff(*y)))
            .sum();
        if (total as f32) / 255.0 < tolerance {
            matching += 1;
            diff.extend_from_slice(&[0, 0, 0, 255]);
        } else {
            diff.extend_from_slice(&[255, 0, 0, 255]);
        }
    }
    Some((matching as f32 / (a.len() / 4) as f32, diff))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_images_are_fully_similar() {
        let image = vec![10u8, 20, 30, 200, 100, 0];
        assert!((rgb_similarity(&image, &image) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn opposite_images_share_nothing() {
        let black = vec![0u8; 12];
        let white = vec![255u8; 12];
        assert!(rgb_similarity(&black, &white).abs() < 1e-12);
    }

    #[test]
    fn different_shapes_share_nothing() {
        assert_eq!(rgb_similarity(&[0, 0, 0], &[0, 0, 0, 0, 0, 0]), 0.0);
        assert_eq!(rgb_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn the_difference_is_per_channel_and_absolute() {
        assert_eq!(
            rgb_difference(&[10, 200, 5], &[20, 100, 5]),
            vec![10, 100, 0]
        );
    }

    #[test]
    fn pixels_within_the_tolerance_match() {
        let a = vec![0, 0, 0, 255, 100, 100, 100, 255];
        let b = vec![1, 0, 0, 255, 200, 100, 100, 255];
        let (share, diff) = matching_pixels(&a, &b, 0.01).expect("same shape");
        assert!((share - 0.5).abs() < 1e-6);
        assert_eq!(diff, vec![0, 0, 0, 255, 255, 0, 0, 255]);
    }

    #[test]
    fn rgba_buffers_must_be_whole_pixels_of_one_shape() {
        assert!(matching_pixels(&[0, 0, 0], &[0, 0, 0], 0.01).is_none());
        assert!(matching_pixels(&[0; 4], &[0; 8], 0.01).is_none());
    }
}
