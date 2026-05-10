use std::sync::Mutex;

#[derive(Default, Clone, Debug)]
pub struct CheapPbrStats {
    pub diffuse_lambert: u32,
    pub specular_disabled: u32,
    pub skipped_avatar: u32,
    pub skipped_hud: u32,
    pub skipped_already: u32,
    pub skipped_not_base_mat: u32,
}

impl CheapPbrStats {
    pub fn to_summary_string(&self) -> String {
        format!(
            "lambert={} spec_off={} avatar={} hud={} already={} not_base={}",
            self.diffuse_lambert,
            self.specular_disabled,
            self.skipped_avatar,
            self.skipped_hud,
            self.skipped_already,
            self.skipped_not_base_mat,
        )
    }
}

static GLOBAL: Mutex<CheapPbrStats> = Mutex::new(CheapPbrStats {
    diffuse_lambert: 0,
    specular_disabled: 0,
    skipped_avatar: 0,
    skipped_hud: 0,
    skipped_already: 0,
    skipped_not_base_mat: 0,
});

pub fn record_lambert() {
    if let Ok(mut g) = GLOBAL.lock() {
        g.diffuse_lambert = g.diffuse_lambert.saturating_add(1);
    }
}
pub fn record_specular_disabled() {
    if let Ok(mut g) = GLOBAL.lock() {
        g.specular_disabled = g.specular_disabled.saturating_add(1);
    }
}
pub fn record_skipped_avatar() {
    if let Ok(mut g) = GLOBAL.lock() {
        g.skipped_avatar = g.skipped_avatar.saturating_add(1);
    }
}
pub fn record_skipped_hud() {
    if let Ok(mut g) = GLOBAL.lock() {
        g.skipped_hud = g.skipped_hud.saturating_add(1);
    }
}
pub fn record_skipped_already() {
    if let Ok(mut g) = GLOBAL.lock() {
        g.skipped_already = g.skipped_already.saturating_add(1);
    }
}
pub fn record_skipped_not_base_mat() {
    if let Ok(mut g) = GLOBAL.lock() {
        g.skipped_not_base_mat = g.skipped_not_base_mat.saturating_add(1);
    }
}

pub fn drain_global_stats() -> String {
    let snap = match GLOBAL.lock() {
        Ok(mut g) => {
            let s = g.clone();
            *g = CheapPbrStats::default();
            s
        }
        Err(_) => return String::new(),
    };
    snap.to_summary_string()
}
