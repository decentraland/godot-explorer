//! Descriptor Set Tracker
//!
//! Tracks creation of Vulkan descriptor sets through material and texture operations.
//! Each StandardMaterial3D creation and duplication allocates descriptor sets.
//! This helps debug VK_ERROR_OUT_OF_POOL_MEMORY errors on mobile devices.

use std::sync::atomic::{AtomicU64, Ordering};

/// Counter for StandardMaterial3D::new_gd() calls
pub static MATERIAL_NEW_COUNT: AtomicU64 = AtomicU64::new(0);

/// Counter for material .duplicate() calls
pub static MATERIAL_DUPLICATE_COUNT: AtomicU64 = AtomicU64::new(0);

/// Counter for texture set operations (each texture uses descriptors)
pub static TEXTURE_SET_COUNT: AtomicU64 = AtomicU64::new(0);

/// Counter for mesh duplications
pub static MESH_DUPLICATE_COUNT: AtomicU64 = AtomicU64::new(0);

/// Counter for material disposals/frees
pub static MATERIAL_DISPOSED_COUNT: AtomicU64 = AtomicU64::new(0);

/// Increment the new material counter
#[inline]
pub fn track_material_new() {
    let count = MATERIAL_NEW_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if count.is_multiple_of(100) {
        tracing::info!("[DescriptorTracker] Material NEW count: {}", count);
    }
}

/// Increment the material duplicate counter
#[inline]
pub fn track_material_duplicate() {
    let count = MATERIAL_DUPLICATE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if count.is_multiple_of(100) {
        tracing::info!("[DescriptorTracker] Material DUPLICATE count: {}", count);
    }
}

/// Increment the texture set counter
#[inline]
pub fn track_texture_set() {
    let count = TEXTURE_SET_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if count.is_multiple_of(500) {
        tracing::info!("[DescriptorTracker] Texture SET count: {}", count);
    }
}

/// Increment the mesh duplicate counter
#[inline]
pub fn track_mesh_duplicate() {
    let count = MESH_DUPLICATE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if count.is_multiple_of(100) {
        tracing::info!("[DescriptorTracker] Mesh DUPLICATE count: {}", count);
    }
}

/// Increment the material disposed counter
#[inline]
pub fn track_material_disposed() {
    MATERIAL_DISPOSED_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Get the current counts
pub fn get_counts() -> DescriptorCounts {
    DescriptorCounts {
        material_new: MATERIAL_NEW_COUNT.load(Ordering::Relaxed),
        material_duplicate: MATERIAL_DUPLICATE_COUNT.load(Ordering::Relaxed),
        texture_set: TEXTURE_SET_COUNT.load(Ordering::Relaxed),
        mesh_duplicate: MESH_DUPLICATE_COUNT.load(Ordering::Relaxed),
        material_disposed: MATERIAL_DISPOSED_COUNT.load(Ordering::Relaxed),
    }
}

/// Get estimated total descriptor sets allocated
/// Each material uses approximately 10-20 descriptor sets (for various textures/uniforms)
pub fn get_estimated_descriptor_sets() -> u64 {
    let counts = get_counts();
    // Conservative estimate: each material = ~10 descriptor sets
    (counts.material_new + counts.material_duplicate) * 10
}

/// Reset all counters (useful for testing)
pub fn reset_counts() {
    MATERIAL_NEW_COUNT.store(0, Ordering::Relaxed);
    MATERIAL_DUPLICATE_COUNT.store(0, Ordering::Relaxed);
    TEXTURE_SET_COUNT.store(0, Ordering::Relaxed);
    MESH_DUPLICATE_COUNT.store(0, Ordering::Relaxed);
    MATERIAL_DISPOSED_COUNT.store(0, Ordering::Relaxed);
    tracing::info!("[DescriptorTracker] Counters reset");
}

/// Log current descriptor tracking stats
pub fn log_stats() {
    let counts = get_counts();
    let active_materials =
        (counts.material_new + counts.material_duplicate).saturating_sub(counts.material_disposed);
    let estimated_descriptors = get_estimated_descriptor_sets();

    tracing::info!(
        "[DescriptorTracker] Stats: new={}, dup={}, disposed={}, active~={}, textures={}, meshes={}, est_descriptors={}",
        counts.material_new,
        counts.material_duplicate,
        counts.material_disposed,
        active_materials,
        counts.texture_set,
        counts.mesh_duplicate,
        estimated_descriptors
    );
}

#[derive(Debug, Clone, Copy)]
pub struct DescriptorCounts {
    pub material_new: u64,
    pub material_duplicate: u64,
    pub texture_set: u64,
    pub mesh_duplicate: u64,
    pub material_disposed: u64,
}

impl DescriptorCounts {
    pub fn total_materials(&self) -> u64 {
        self.material_new + self.material_duplicate
    }

    pub fn active_materials(&self) -> u64 {
        self.total_materials()
            .saturating_sub(self.material_disposed)
    }
}
