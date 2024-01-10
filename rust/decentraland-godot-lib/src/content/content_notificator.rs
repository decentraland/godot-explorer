use std::{collections::HashMap, sync::Arc};

use tokio::sync::{Notify, RwLock};

pub struct ContentNotificator {
    files: RwLock<HashMap<String, Arc<Notify>>>,
}

impl Default for ContentNotificator {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentNotificator {
    pub fn new() -> Self {
        Self {
            files: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get_or_create_notify(&self, key: String) -> (bool, Arc<Notify>) {
        {
            let files = self.files.read().await;
            if let Some(notify) = files.get(&key) {
                return (false, notify.clone());
            }
        }

        let mut files = self.files.write().await;
        let notify = Arc::new(Notify::new());
        files.insert(key, notify.clone());
        drop(files);

        (true, notify)
    }

    pub async fn remove_notify(&mut self, key: String) {
        let mut files = self.files.write().await;
        files.remove(&key);
    }
}
