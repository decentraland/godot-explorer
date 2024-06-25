use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;

pub struct DownloadState {
    pub current_size: AtomicU64,
    pub start_time: Instant,
    pub done: bool,
}

impl DownloadState {
    pub fn new() -> Self {
        Self {
            current_size: AtomicU64::new(0),
            start_time: Instant::now(),
            done: false,
        }
    }

    pub fn update_progress(&self, current_size: u64) {
        self.current_size.store(current_size, Ordering::SeqCst);
    }

    pub fn mark_done(&mut self) {
        self.done = true;
    }

    pub fn get_speed(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.current_size.load(Ordering::SeqCst) as f64 / elapsed
        } else {
            0.0
        }
    }
}

pub struct DownloadStateInfo {
    pub current_size: u64,
    pub speed: f64,
    pub done: bool,
}

impl DownloadStateInfo {
    pub fn from_download_state(state: &DownloadState) -> Self {
        Self {
            current_size: state.current_size.load(Ordering::SeqCst),
            speed: state.get_speed(),
            done: state.done,
        }
    }
}

pub struct ResourceDownloadTracking {
    downloads: Arc<RwLock<HashMap<String, Arc<RwLock<DownloadState>>>>>,
}

impl ResourceDownloadTracking {
    pub fn new() -> Self {
        Self {
            downloads: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start(&self, file_hash: String) {
        let state = Arc::new(RwLock::new(DownloadState::new()));
        let mut downloads = self.downloads.write().await;
        downloads.insert(file_hash, state);
    }

    pub async fn report_progress(&self, file_hash: &str, current_size: u64) {
        let downloads = self.downloads.read().await;
        if let Some(state) = downloads.get(file_hash) {
            let state = state.read().await;
            state.update_progress(current_size);
        }
    }

    pub async fn end(&self, file_hash: &str) {
        let downloads = self.downloads.read().await;
        if let Some(state) = downloads.get(file_hash) {
            let mut state = state.write().await;
            state.mark_done();
        }
    }

    pub fn consume_downloads_state(&self) -> HashMap<String, DownloadStateInfo> {
        let mut downloads = self.downloads.blocking_write();
        let mut downloads_to_return = HashMap::new();

        downloads.retain(|file_hash, state| {
            let state_info = {
                let state = state.blocking_read();
                DownloadStateInfo::from_download_state(&state)
            };

            if state_info.done {
                downloads_to_return.insert(file_hash.clone(), state_info);
                false
            } else {
                downloads_to_return.insert(file_hash.clone(), state_info);
                true
            }
        });

        downloads_to_return
    }
}
