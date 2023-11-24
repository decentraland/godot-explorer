use std::sync::{Arc, RwLock};

use http::Uri;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
pub enum PortableLocation {
    Urn(String),
    Ens(String),
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpawnResponse {
    pub pid: String,
    pub parent_cid: String,
    pub name: String,
    pub ens: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AvatarForUserData {
    pub body_shape: String,
    pub skin_color: String,
    pub hair_color: String,
    pub eye_color: String,
    pub wearables: Vec<String>,
    pub snapshots: Option<Snapshots>,
}

#[derive(Serialize, Debug)]
pub struct Snapshots {
    pub face256: String,
    pub body: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserData {
    pub display_name: String,
    pub public_key: Option<String>,
    pub has_connected_web3: bool,
    pub user_id: String,
    pub version: u32,
    pub avatar: Option<AvatarForUserData>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetRealmResponse {
    pub base_url: String,
    pub realm_name: String,
    pub network_id: i32,
    pub comms_adapter: String,
    pub is_preview: bool,
}

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
    // Restricted Actions
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
    OpenNftDialog {
        urn: String,
        response: RpcResultSender<Result<(), String>>,
    },
    OpenExternalUrl {
        url: Uri,
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
    // Runtime
    GetRealm {
        response: RpcResultSender<GetRealmResponse>,
    },
    // Portable Experiences
    SpawnPortable {
        location: PortableLocation,
        response: RpcResultSender<Result<SpawnResponse, String>>,
    },
    KillPortable {
        location: PortableLocation,
        response: RpcResultSender<bool>,
    },
    ListPortables {
        response: RpcResultSender<Vec<SpawnResponse>>,
    },
}

#[derive(Debug)]
pub enum LocalCall {
    PlayersGetPlayerData {
        user_id: String,
        response: RpcResultSender<Option<UserData>>,
    },
    PlayersGetPlayersInScene {
        response: RpcResultSender<Vec<String>>,
    },
    PlayersGetConnectedPlayers {
        response: RpcResultSender<Vec<String>>,
    },
}
