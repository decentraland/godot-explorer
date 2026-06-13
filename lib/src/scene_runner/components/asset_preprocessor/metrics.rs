use std::sync::Mutex;

#[derive(Default, Clone, Debug)]
pub struct PreprocStats {
    pub meshes_stripped: u32,
    pub stripped_bytes_total: u64,
    pub occluders_added: u32,
    pub impostors_baked: u32,
}

impl PreprocStats {
    pub fn to_summary_string(&self) -> String {
        format!(
            "stripped={} bytes_saved={} occluders={} impostors={}",
            self.meshes_stripped,
            self.stripped_bytes_total,
            self.occluders_added,
            self.impostors_baked,
        )
    }
}

static GLOBAL: Mutex<PreprocStats> = Mutex::new(PreprocStats {
    meshes_stripped: 0,
    stripped_bytes_total: 0,
    occluders_added: 0,
    impostors_baked: 0,
});

pub fn record_stripped(bytes: u64) {
    if let Ok(mut g) = GLOBAL.lock() {
        g.meshes_stripped = g.meshes_stripped.saturating_add(1);
        g.stripped_bytes_total = g.stripped_bytes_total.saturating_add(bytes);
    }
}

pub fn record_occluder() {
    if let Ok(mut g) = GLOBAL.lock() {
        g.occluders_added = g.occluders_added.saturating_add(1);
    }
}

pub fn record_impostors(count: u32) {
    if count == 0 {
        return;
    }
    if let Ok(mut g) = GLOBAL.lock() {
        g.impostors_baked = g.impostors_baked.saturating_add(count);
    }
}

pub fn drain_global_stats() -> String {
    let snap = match GLOBAL.lock() {
        Ok(mut g) => {
            let s = g.clone();
            *g = PreprocStats::default();
            s
        }
        Err(_) => return String::new(),
    };
    snap.to_summary_string()
}
