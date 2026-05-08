//! Stats accumulators for the textureless mesh merger.

use std::sync::Mutex;

use super::classifier::{Classification, SkipReason};

#[derive(Default, Clone, Debug)]
pub struct MergerStats {
    pub mergeable: u32,
    pub skipped_no_mesh: u32,
    pub skipped_not_visible: u32,
    pub skipped_collider_name: u32,
    pub skipped_skeleton_ancestor: u32,
    pub skipped_animation_player_ancestor: u32,
    pub skipped_avatar_ancestor: u32,
    pub skipped_blend_shapes: u32,
    pub skipped_textured: u32,
    pub skipped_shader_material: u32,
    pub skipped_no_material: u32,
    pub skipped_multi_surface: u32,
    pub skipped_has_tween: u32,
    pub skipped_has_modifier: u32,
}

impl MergerStats {
    pub fn record(&mut self, classification: &Classification) {
        match classification {
            Classification::Mergeable { .. } => self.mergeable = self.mergeable.saturating_add(1),
            Classification::Skip(reason) => {
                let slot = match reason {
                    SkipReason::NoMesh => &mut self.skipped_no_mesh,
                    SkipReason::NotVisible => &mut self.skipped_not_visible,
                    SkipReason::ColliderName => &mut self.skipped_collider_name,
                    SkipReason::SkeletonAncestor => &mut self.skipped_skeleton_ancestor,
                    SkipReason::AnimationPlayerAncestor => {
                        &mut self.skipped_animation_player_ancestor
                    }
                    SkipReason::AvatarAncestor => &mut self.skipped_avatar_ancestor,
                    SkipReason::BlendShapes => &mut self.skipped_blend_shapes,
                    SkipReason::Textured => &mut self.skipped_textured,
                    SkipReason::ShaderMaterial => &mut self.skipped_shader_material,
                    SkipReason::NoMaterial => &mut self.skipped_no_material,
                    SkipReason::MultiSurface => &mut self.skipped_multi_surface,
                    SkipReason::HasTween => &mut self.skipped_has_tween,
                    SkipReason::HasModifier => &mut self.skipped_has_modifier,
                };
                *slot = slot.saturating_add(1);
            }
        }
    }

    pub fn merge_into(&self, dst: &mut MergerStats) {
        dst.mergeable = dst.mergeable.saturating_add(self.mergeable);
        dst.skipped_no_mesh = dst.skipped_no_mesh.saturating_add(self.skipped_no_mesh);
        dst.skipped_not_visible = dst
            .skipped_not_visible
            .saturating_add(self.skipped_not_visible);
        dst.skipped_collider_name = dst
            .skipped_collider_name
            .saturating_add(self.skipped_collider_name);
        dst.skipped_skeleton_ancestor = dst
            .skipped_skeleton_ancestor
            .saturating_add(self.skipped_skeleton_ancestor);
        dst.skipped_animation_player_ancestor = dst
            .skipped_animation_player_ancestor
            .saturating_add(self.skipped_animation_player_ancestor);
        dst.skipped_avatar_ancestor = dst
            .skipped_avatar_ancestor
            .saturating_add(self.skipped_avatar_ancestor);
        dst.skipped_blend_shapes = dst
            .skipped_blend_shapes
            .saturating_add(self.skipped_blend_shapes);
        dst.skipped_textured = dst.skipped_textured.saturating_add(self.skipped_textured);
        dst.skipped_shader_material = dst
            .skipped_shader_material
            .saturating_add(self.skipped_shader_material);
        dst.skipped_no_material = dst
            .skipped_no_material
            .saturating_add(self.skipped_no_material);
        dst.skipped_multi_surface = dst
            .skipped_multi_surface
            .saturating_add(self.skipped_multi_surface);
        dst.skipped_has_tween = dst
            .skipped_has_tween
            .saturating_add(self.skipped_has_tween);
        dst.skipped_has_modifier = dst
            .skipped_has_modifier
            .saturating_add(self.skipped_has_modifier);
    }

    pub fn to_summary_string(&self) -> String {
        format!(
            "mergeable={} no_mesh={} not_visible={} collider={} skeleton={} anim_player={} avatar={} blend={} textured={} shader_mat={} no_mat={} multi_surf={} tween={} modifier={}",
            self.mergeable,
            self.skipped_no_mesh,
            self.skipped_not_visible,
            self.skipped_collider_name,
            self.skipped_skeleton_ancestor,
            self.skipped_animation_player_ancestor,
            self.skipped_avatar_ancestor,
            self.skipped_blend_shapes,
            self.skipped_textured,
            self.skipped_shader_material,
            self.skipped_no_material,
            self.skipped_multi_surface,
            self.skipped_has_tween,
            self.skipped_has_modifier,
        )
    }
}

/// Global accumulator across all scenes. Drained by the bench runner via
/// `drain_global_stats()` (similar to `update_scene::drain_state_timing`).
static GLOBAL_STATS: Mutex<MergerStats> = Mutex::new(MergerStats {
    mergeable: 0,
    skipped_no_mesh: 0,
    skipped_not_visible: 0,
    skipped_collider_name: 0,
    skipped_skeleton_ancestor: 0,
    skipped_animation_player_ancestor: 0,
    skipped_avatar_ancestor: 0,
    skipped_blend_shapes: 0,
    skipped_textured: 0,
    skipped_shader_material: 0,
    skipped_no_material: 0,
    skipped_multi_surface: 0,
    skipped_has_tween: 0,
    skipped_has_modifier: 0,
});

pub fn record_global(classification: &Classification) {
    if let Ok(mut g) = GLOBAL_STATS.lock() {
        g.record(classification);
    }
}

/// Snapshot the global stats without resetting. Used by HUD readers that
/// poll continuously.
pub fn snapshot_global_stats() -> MergerStats {
    GLOBAL_STATS
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default()
}

/// Drain the global stats and return a multiline summary, then reset to zero.
/// Mirrors `drain_state_timing` so the bench runner can capture a sample
/// window cleanly.
pub fn drain_global_stats() -> String {
    let snapshot = match GLOBAL_STATS.lock() {
        Ok(mut g) => {
            let s = g.clone();
            *g = MergerStats::default();
            s
        }
        Err(_) => return String::new(),
    };
    snapshot.to_summary_string()
}
