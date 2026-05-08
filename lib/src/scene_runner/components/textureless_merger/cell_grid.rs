//! Spatial bucketing constants and helpers.
//!
//! 32 m horizontal cell size: smaller cells lose the draw-call win, larger
//! ones blow up frustum culling because the merged AABB covers too much.

use godot::builtin::Vector3;

pub const CELL_SIZE_M: f32 = 32.0;
pub const MIN_BUCKET_SIZE: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BucketKey {
    pub transparency: i32,
    pub cull_mode: i32,
    pub cx: i32,
    pub cz: i32,
}

#[inline]
pub fn cell_for(world_pos: Vector3) -> (i32, i32) {
    (
        (world_pos.x / CELL_SIZE_M).floor() as i32,
        (world_pos.z / CELL_SIZE_M).floor() as i32,
    )
}

#[inline]
pub fn cell_center(cx: i32, cz: i32) -> Vector3 {
    Vector3::new(
        (cx as f32 + 0.5) * CELL_SIZE_M,
        0.0,
        (cz as f32 + 0.5) * CELL_SIZE_M,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_zero_origin() {
        assert_eq!(cell_for(Vector3::new(0.0, 0.0, 0.0)), (0, 0));
        assert_eq!(cell_for(Vector3::new(0.1, 99.0, 0.1)), (0, 0));
    }

    #[test]
    fn cell_positive_boundaries() {
        assert_eq!(cell_for(Vector3::new(31.999, 0.0, 0.0)), (0, 0));
        assert_eq!(cell_for(Vector3::new(32.0, 0.0, 0.0)), (1, 0));
        assert_eq!(cell_for(Vector3::new(63.999, 0.0, 32.0)), (1, 1));
    }

    #[test]
    fn cell_negative_boundaries() {
        // floor() rounds toward -inf, so negatives land in the cell to the left.
        assert_eq!(cell_for(Vector3::new(-0.001, 0.0, 0.0)), (-1, 0));
        assert_eq!(cell_for(Vector3::new(-32.0, 0.0, -32.0)), (-1, -1));
        assert_eq!(cell_for(Vector3::new(-32.001, 0.0, 0.0)), (-2, 0));
    }
}
