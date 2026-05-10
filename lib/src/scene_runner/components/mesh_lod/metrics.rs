//! Per-Scene + global stats for the mesh-LOD pass.

use std::sync::Mutex;

use super::classifier::{Classification, SkipReason};

#[derive(Default, Clone, Debug)]
pub struct LodStats {
    pub eligible: u32,
    pub skipped_no_mesh: u32,
    pub skipped_blend_shapes: u32,
    pub skipped_avatar: u32,
    pub skipped_skinned: u32,
    pub skipped_already_has_lods: u32,
    pub skipped_too_small: u32,
    pub skipped_tween: u32,
    pub skipped_modifier: u32,
    pub meshes_baked: u32,
    pub bake_cache_hits: u32,
    pub bake_failed: u32,
    pub source_index_total: u64,
    pub lod0_index_total: u64,
    pub shadow_meshes_baked: u32,
    pub shadow_source_index_total: u64,
    pub shadow_index_total: u64,
}

impl LodStats {
    pub fn record(&mut self, c: &Classification) {
        match c {
            Classification::Eligible => self.eligible = self.eligible.saturating_add(1),
            Classification::Skip(r) => {
                let s = match r {
                    SkipReason::NoMesh => &mut self.skipped_no_mesh,
                    SkipReason::BlendShapes => &mut self.skipped_blend_shapes,
                    SkipReason::AvatarAncestor => &mut self.skipped_avatar,
                    SkipReason::SkinnedAncestor => &mut self.skipped_skinned,
                    SkipReason::AlreadyHasLods => &mut self.skipped_already_has_lods,
                    SkipReason::TooSmall => &mut self.skipped_too_small,
                    SkipReason::HasTween => &mut self.skipped_tween,
                    SkipReason::HasModifier => &mut self.skipped_modifier,
                };
                *s = s.saturating_add(1);
            }
        }
    }

    pub fn to_summary_string(&self) -> String {
        format!(
            "eligible={} baked={} cache_hits={} bake_failed={} src_idx={} lod0_idx={} shadow_baked={} shadow_src_idx={} shadow_idx={} no_mesh={} blend={} avatar={} skinned={} has_lods={} small={} tween={} modifier={}",
            self.eligible,
            self.meshes_baked,
            self.bake_cache_hits,
            self.bake_failed,
            self.source_index_total,
            self.lod0_index_total,
            self.shadow_meshes_baked,
            self.shadow_source_index_total,
            self.shadow_index_total,
            self.skipped_no_mesh,
            self.skipped_blend_shapes,
            self.skipped_avatar,
            self.skipped_skinned,
            self.skipped_already_has_lods,
            self.skipped_too_small,
            self.skipped_tween,
            self.skipped_modifier,
        )
    }
}

static GLOBAL_STATS: Mutex<LodStats> = Mutex::new(LodStats {
    eligible: 0,
    skipped_no_mesh: 0,
    skipped_blend_shapes: 0,
    skipped_avatar: 0,
    skipped_skinned: 0,
    skipped_already_has_lods: 0,
    skipped_too_small: 0,
    skipped_tween: 0,
    skipped_modifier: 0,
    meshes_baked: 0,
    bake_cache_hits: 0,
    bake_failed: 0,
    source_index_total: 0,
    lod0_index_total: 0,
    shadow_meshes_baked: 0,
    shadow_source_index_total: 0,
    shadow_index_total: 0,
});

pub fn record_global(c: &Classification) {
    if let Ok(mut g) = GLOBAL_STATS.lock() {
        g.record(c);
    }
}

pub fn record_bake(source_idx: u64, lod0_idx: u64) {
    if let Ok(mut g) = GLOBAL_STATS.lock() {
        g.meshes_baked = g.meshes_baked.saturating_add(1);
        g.source_index_total = g.source_index_total.saturating_add(source_idx);
        g.lod0_index_total = g.lod0_index_total.saturating_add(lod0_idx);
    }
}

pub fn record_cache_hit() {
    if let Ok(mut g) = GLOBAL_STATS.lock() {
        g.bake_cache_hits = g.bake_cache_hits.saturating_add(1);
    }
}

#[allow(dead_code)] pub fn record_shadow_bake(source_idx: u64, shadow_idx: u64) {
    if let Ok(mut g) = GLOBAL_STATS.lock() {
        g.shadow_meshes_baked = g.shadow_meshes_baked.saturating_add(1);
        g.shadow_source_index_total = g.shadow_source_index_total.saturating_add(source_idx);
        g.shadow_index_total = g.shadow_index_total.saturating_add(shadow_idx);
    }
}

pub fn record_bake_failed() {
    if let Ok(mut g) = GLOBAL_STATS.lock() {
        g.bake_failed = g.bake_failed.saturating_add(1);
    }
}

pub fn drain_global_stats() -> String {
    let snap = match GLOBAL_STATS.lock() {
        Ok(mut g) => {
            let s = g.clone();
            *g = LodStats::default();
            s
        }
        Err(_) => return String::new(),
    };
    snap.to_summary_string()
}
