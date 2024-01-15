use std::{collections::HashMap, sync::Arc};

use tokio::sync::{Notify, RwLock};

#[derive(Clone, Debug)]
pub enum ContentState {
    RequestOwner,
    Busy(Arc<Notify>),
    Released(Result<(), String>),
}

pub struct ContentNotificator {
    files: RwLock<HashMap<String, ContentState>>,
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

    pub async fn get(&self, key: &String) -> Option<ContentState> {
        let files = self.files.read().await;
        files.get(key).cloned()
    }

    pub async fn get_or_create_notify(&self, key: &String) -> ContentState {
        {
            let files = self.files.read().await;
            if let Some(content_state) = files.get(key) {
                return content_state.clone();
            }
        }

        let mut files = self.files.write().await;
        let content_state = ContentState::Busy(Arc::new(Notify::new()));
        files.insert(key.clone(), content_state.clone());
        ContentState::RequestOwner
    }

    pub async fn resolve(&self, key: &String, result: Result<(), String>) {
        let mut files = self.files.write().await;
        if let Some(notify) = files.insert(key.clone(), ContentState::Released(result)) {
            if let ContentState::Busy(notify) = notify {
                notify.notify_waiters();
            }
        }
        files.remove(key);
    }
}
