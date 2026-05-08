//! Per-Scene + global stats for auto-distance-cull.

use std::sync::Mutex;

use super::classifier::{Classification, SkipReason};

#[derive(Default, Clone, Debug)]
pub struct CullStats {
    pub eligible: u32,
    pub set_count: u32,
    pub skipped_no_mesh: u32,
    pub skipped_avatar: u32,
    pub skipped_already_set: u32,
    pub skipped_not_visible: u32,
    pub skipped_hud_or_ui: u32,
    pub end_distance_sum: f32,
}

impl CullStats {
    pub fn record(&mut self, c: &Classification) {
        match c {
            Classification::Eligible => self.eligible = self.eligible.saturating_add(1),
            Classification::Skip(r) => {
                let s = match r {
                    SkipReason::NoMesh => &mut self.skipped_no_mesh,
                    SkipReason::AvatarAncestor => &mut self.skipped_avatar,
                    SkipReason::AlreadyRangeSet => &mut self.skipped_already_set,
                    SkipReason::NotVisible => &mut self.skipped_not_visible,
                    SkipReason::HudOrUi => &mut self.skipped_hud_or_ui,
                };
                *s = s.saturating_add(1);
            }
        }
    }

    pub fn to_summary_string(&self) -> String {
        let avg = if self.set_count > 0 {
            self.end_distance_sum / self.set_count as f32
        } else {
            0.0
        };
        format!(
            "eligible={} set={} avg_end_m={:.1} no_mesh={} avatar={} already_set={} not_visible={} hud={}",
            self.eligible,
            self.set_count,
            avg,
            self.skipped_no_mesh,
            self.skipped_avatar,
            self.skipped_already_set,
            self.skipped_not_visible,
            self.skipped_hud_or_ui,
        )
    }
}

static GLOBAL_STATS: Mutex<CullStats> = Mutex::new(CullStats {
    eligible: 0,
    set_count: 0,
    skipped_no_mesh: 0,
    skipped_avatar: 0,
    skipped_already_set: 0,
    skipped_not_visible: 0,
    skipped_hud_or_ui: 0,
    end_distance_sum: 0.0,
});

pub fn record_global(c: &Classification) {
    if let Ok(mut g) = GLOBAL_STATS.lock() {
        g.record(c);
    }
}

pub fn record_set(end_m: f32) {
    if let Ok(mut g) = GLOBAL_STATS.lock() {
        g.set_count = g.set_count.saturating_add(1);
        g.end_distance_sum += end_m;
    }
}

pub fn drain_global_stats() -> String {
    let snap = match GLOBAL_STATS.lock() {
        Ok(mut g) => {
            let s = g.clone();
            *g = CullStats::default();
            s
        }
        Err(_) => return String::new(),
    };
    snap.to_summary_string()
}
