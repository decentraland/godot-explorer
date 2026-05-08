//! Per-Scene + global stats for occluder auto-gen.

use std::sync::Mutex;

use super::classifier::{Classification, SkipReason};

#[derive(Default, Clone, Debug)]
pub struct OccluderStats {
    pub eligible: u32,
    pub added: u32,
    pub skipped_no_mesh: u32,
    pub skipped_not_visible: u32,
    pub skipped_avatar: u32,
    pub skipped_hud_or_ui: u32,
    pub skipped_too_small: u32,
    pub skipped_too_thin: u32,
    pub skipped_transparent: u32,
    pub skipped_already_occluded: u32,
    pub aabb_diag_sum: f32,
}

impl OccluderStats {
    pub fn record(&mut self, c: &Classification) {
        match c {
            Classification::Eligible => self.eligible = self.eligible.saturating_add(1),
            Classification::Skip(r) => {
                let s = match r {
                    SkipReason::NoMesh => &mut self.skipped_no_mesh,
                    SkipReason::NotVisible => &mut self.skipped_not_visible,
                    SkipReason::AvatarAncestor => &mut self.skipped_avatar,
                    SkipReason::HudOrUi => &mut self.skipped_hud_or_ui,
                    SkipReason::TooSmall => &mut self.skipped_too_small,
                    SkipReason::TooThin => &mut self.skipped_too_thin,
                    SkipReason::Transparent => &mut self.skipped_transparent,
                    SkipReason::AlreadyOccluded => &mut self.skipped_already_occluded,
                };
                *s = s.saturating_add(1);
            }
        }
    }

    pub fn to_summary_string(&self) -> String {
        let avg = if self.added > 0 {
            self.aabb_diag_sum / self.added as f32
        } else {
            0.0
        };
        format!(
            "eligible={} added={} avg_diag_m={:.1} no_mesh={} not_visible={} avatar={} hud={} small={} thin={} transparent={} already={}",
            self.eligible,
            self.added,
            avg,
            self.skipped_no_mesh,
            self.skipped_not_visible,
            self.skipped_avatar,
            self.skipped_hud_or_ui,
            self.skipped_too_small,
            self.skipped_too_thin,
            self.skipped_transparent,
            self.skipped_already_occluded,
        )
    }
}

static GLOBAL_STATS: Mutex<OccluderStats> = Mutex::new(OccluderStats {
    eligible: 0,
    added: 0,
    skipped_no_mesh: 0,
    skipped_not_visible: 0,
    skipped_avatar: 0,
    skipped_hud_or_ui: 0,
    skipped_too_small: 0,
    skipped_too_thin: 0,
    skipped_transparent: 0,
    skipped_already_occluded: 0,
    aabb_diag_sum: 0.0,
});

pub fn record_global(c: &Classification) {
    if let Ok(mut g) = GLOBAL_STATS.lock() {
        g.record(c);
    }
}

pub fn record_added(diag_m: f32) {
    if let Ok(mut g) = GLOBAL_STATS.lock() {
        g.added = g.added.saturating_add(1);
        g.aabb_diag_sum += diag_m;
    }
}

pub fn drain_global_stats() -> String {
    let snap = match GLOBAL_STATS.lock() {
        Ok(mut g) => {
            let s = g.clone();
            *g = OccluderStats::default();
            s
        }
        Err(_) => return String::new(),
    };
    snap.to_summary_string()
}
