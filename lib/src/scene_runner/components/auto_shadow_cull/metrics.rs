use std::sync::Mutex;

use super::{Classification, SkipReason};

#[derive(Default, Clone, Debug)]
pub struct CullStats {
    pub culled: u32,
    pub disabled: u32,
    pub skipped_no_mesh: u32,
    pub skipped_not_visible: u32,
    pub skipped_avatar: u32,
    pub skipped_hud: u32,
    pub skipped_already_off: u32,
    pub skipped_large_enough: u32,
}

impl CullStats {
    pub fn record(&mut self, c: &Classification) {
        match c {
            Classification::Cull => self.culled = self.culled.saturating_add(1),
            Classification::Skip(r) => {
                let s = match r {
                    SkipReason::NoMesh => &mut self.skipped_no_mesh,
                    SkipReason::NotVisible => &mut self.skipped_not_visible,
                    SkipReason::AvatarAncestor => &mut self.skipped_avatar,
                    SkipReason::HudOrUi => &mut self.skipped_hud,
                    SkipReason::AlreadyOff => &mut self.skipped_already_off,
                    SkipReason::LargeEnough => &mut self.skipped_large_enough,
                };
                *s = s.saturating_add(1);
            }
        }
    }

    pub fn to_summary_string(&self) -> String {
        format!(
            "culled={} disabled={} no_mesh={} not_visible={} avatar={} hud={} already_off={} large={}",
            self.culled,
            self.disabled,
            self.skipped_no_mesh,
            self.skipped_not_visible,
            self.skipped_avatar,
            self.skipped_hud,
            self.skipped_already_off,
            self.skipped_large_enough,
        )
    }
}

static GLOBAL: Mutex<CullStats> = Mutex::new(CullStats {
    culled: 0,
    disabled: 0,
    skipped_no_mesh: 0,
    skipped_not_visible: 0,
    skipped_avatar: 0,
    skipped_hud: 0,
    skipped_already_off: 0,
    skipped_large_enough: 0,
});

pub fn record_global(c: &Classification) {
    if let Ok(mut g) = GLOBAL.lock() {
        g.record(c);
    }
}

pub fn record_disabled() {
    if let Ok(mut g) = GLOBAL.lock() {
        g.disabled = g.disabled.saturating_add(1);
    }
}

pub fn drain_global_stats() -> String {
    let snap = match GLOBAL.lock() {
        Ok(mut g) => {
            let s = g.clone();
            *g = CullStats::default();
            s
        }
        Err(_) => return String::new(),
    };
    snap.to_summary_string()
}
