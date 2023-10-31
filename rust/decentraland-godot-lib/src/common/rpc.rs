use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct RpcResultSender<T>(Arc<RwLock<Option<tokio::sync::oneshot::Sender<T>>>>);

impl<T: 'static> RpcResultSender<T> {
    pub fn new(sender: tokio::sync::oneshot::Sender<T>) -> Self {
        Self(Arc::new(RwLock::new(Some(sender))))
    }

    pub fn send(&self, result: T) {
        if let Ok(mut guard) = self.0.write() {
            if let Some(response) = guard.take() {
                let _ = response.send(result);
            }
        }
    }

    pub fn take(&self) -> tokio::sync::oneshot::Sender<T> {
        self.0
            .write()
            .ok()
            .and_then(|mut guard| guard.take())
            .take()
            .unwrap()
    }
}

impl<T: 'static> From<tokio::sync::oneshot::Sender<T>> for RpcResultSender<T> {
    fn from(value: tokio::sync::oneshot::Sender<T>) -> Self {
        RpcResultSender::new(value)
    }
}

#[derive(Debug)]
pub enum RpcCall {
    ChangeRealm {
        to: String,
        message: Option<String>,
        response: RpcResultSender<Result<(), String>>,
    },
    MovePlayerTo {
        position_target: [f32; 3],
        camera_target: Option<[f32; 3]>,
        response: RpcResultSender<Result<(), String>>,
    },
    TeleportTo {
        world_coordinates: [i32; 2],
        response: RpcResultSender<Result<(), String>>,
    },
    TriggerEmote {
        emote_id: String,
        response: RpcResultSender<Result<(), String>>,
    },
    TriggerSceneEmote {
        emote_src: String,
        looping: bool,
        response: RpcResultSender<Result<(), String>>,
    },
}

pub type RpcCalls = Vec<RpcCall>;
