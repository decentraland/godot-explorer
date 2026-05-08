//! Per-Scene + global stats for the material atlas.

use std::sync::Mutex;

use super::classifier::{Classification, SkipReason};

#[derive(Default, Clone, Debug)]
pub struct MergerStats {
    pub mergeable: u32,
    pub skipped_no_mesh: u32,
    pub skipped_not_visible: u32,
    pub skipped_collider: u32,
    pub skipped_skeleton: u32,
    pub skipped_anim_player: u32,
    pub skipped_avatar: u32,
    pub skipped_blend: u32,
    pub skipped_shader_mat: u32,
    pub skipped_no_material: u32,
    pub skipped_multi_surface: u32,
    pub skipped_tween: u32,
    pub skipped_modifier: u32,
    pub skipped_unsupported_transparency: u32,
    pub skipped_unsupported_feature: u32,
    pub layers_allocated: u32,
    pub mis_replaced: u32,
    pub atlas_full_skips: u32,
}

impl MergerStats {
    pub fn record(&mut self, c: &Classification) {
        match c {
            Classification::Mergeable { .. } => self.mergeable = self.mergeable.saturating_add(1),
            Classification::Skip(r) => {
                let s = match r {
                    SkipReason::NoMesh => &mut self.skipped_no_mesh,
                    SkipReason::NotVisible => &mut self.skipped_not_visible,
                    SkipReason::ColliderName => &mut self.skipped_collider,
                    SkipReason::SkeletonAncestor => &mut self.skipped_skeleton,
                    SkipReason::AnimationPlayerAncestor => &mut self.skipped_anim_player,
                    SkipReason::AvatarAncestor => &mut self.skipped_avatar,
                    SkipReason::BlendShapes => &mut self.skipped_blend,
                    SkipReason::ShaderMaterial => &mut self.skipped_shader_mat,
                    SkipReason::NoMaterial => &mut self.skipped_no_material,
                    SkipReason::MultiSurface => &mut self.skipped_multi_surface,
                    SkipReason::HasTween => &mut self.skipped_tween,
                    SkipReason::HasModifier => &mut self.skipped_modifier,
                    SkipReason::UnsupportedTransparency => &mut self.skipped_unsupported_transparency,
                    SkipReason::UnsupportedFeature => &mut self.skipped_unsupported_feature,
                };
                *s = s.saturating_add(1);
            }
        }
    }

    pub fn to_summary_string(&self) -> String {
        format!(
            "mergeable={} layers={} mis_replaced={} atlas_full={} no_mesh={} not_visible={} collider={} skeleton={} anim={} avatar={} blend={} shader_mat={} no_mat={} multi_surf={} tween={} modifier={} unsup_transp={} unsup_feat={}",
            self.mergeable,
            self.layers_allocated,
            self.mis_replaced,
            self.atlas_full_skips,
            self.skipped_no_mesh,
            self.skipped_not_visible,
            self.skipped_collider,
            self.skipped_skeleton,
            self.skipped_anim_player,
            self.skipped_avatar,
            self.skipped_blend,
            self.skipped_shader_mat,
            self.skipped_no_material,
            self.skipped_multi_surface,
            self.skipped_tween,
            self.skipped_modifier,
            self.skipped_unsupported_transparency,
            self.skipped_unsupported_feature,
        )
    }
}

static GLOBAL_STATS: Mutex<MergerStats> = Mutex::new(MergerStats {
    mergeable: 0,
    skipped_no_mesh: 0,
    skipped_not_visible: 0,
    skipped_collider: 0,
    skipped_skeleton: 0,
    skipped_anim_player: 0,
    skipped_avatar: 0,
    skipped_blend: 0,
    skipped_shader_mat: 0,
    skipped_no_material: 0,
    skipped_multi_surface: 0,
    skipped_tween: 0,
    skipped_modifier: 0,
    skipped_unsupported_transparency: 0,
    skipped_unsupported_feature: 0,
    layers_allocated: 0,
    mis_replaced: 0,
    atlas_full_skips: 0,
});

pub fn record_global(c: &Classification) {
    if let Ok(mut g) = GLOBAL_STATS.lock() {
        g.record(c);
    }
}

pub fn record_layer_alloc() {
    if let Ok(mut g) = GLOBAL_STATS.lock() {
        g.layers_allocated = g.layers_allocated.saturating_add(1);
    }
}

pub fn record_mi_replace() {
    if let Ok(mut g) = GLOBAL_STATS.lock() {
        g.mis_replaced = g.mis_replaced.saturating_add(1);
    }
}

pub fn record_atlas_full() {
    if let Ok(mut g) = GLOBAL_STATS.lock() {
        g.atlas_full_skips = g.atlas_full_skips.saturating_add(1);
    }
}

pub fn drain_global_stats() -> String {
    let snap = match GLOBAL_STATS.lock() {
        Ok(mut g) => {
            let s = g.clone();
            *g = MergerStats::default();
            s
        }
        Err(_) => return String::new(),
    };
    snap.to_summary_string()
}
