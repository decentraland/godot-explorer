use std::sync::{Arc, RwLock};

use ethers_core::types::H160;
use godot::builtin::Vector2;
use http::Uri;
use serde::Serialize;

use crate::auth::decentraland_auth_server::CreateRequest;

use super::common::{SceneTestPlan, SceneTestResult};

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
pub struct ContentMapping {
    pub file: String,
    pub hash: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetSceneInformationResponse {
    pub urn: String,
    pub content: Vec<ContentMapping>,
    pub metadata_json: String,
    pub base_url: String,
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
                if response.send(result).is_err() {
                    tracing::error!("Failed to send rpc response");
                }
            }
        }
    }

    pub fn take(&self) -> tokio::sync::oneshot::Sender<T> {
        self.0
            .write()
            .ok()
            .and_then(|mut guard| guard.take())
            .unwrap()
    }
}

impl<T: 'static> From<tokio::sync::oneshot::Sender<T>> for RpcResultSender<T> {
    fn from(value: tokio::sync::oneshot::Sender<T>) -> Self {
        RpcResultSender::new(value)
    }
}

/// Specifies the recipient of a network message
#[derive(Debug, Clone, Copy)]
pub enum NetworkMessageRecipient {
    /// Broadcast to all peers
    All,
    /// Send to a specific peer by their Ethereum address
    Peer(H160),
    /// Send to the authoritative multiplayer server
    AuthServer,
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
        avatar_target: Option<[f32; 3]>,
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
    },
    TriggerSceneEmote {
        emote_src: String,
        looping: bool,
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
    SceneTestPlan {
        body: SceneTestPlan,
    },
    SceneTestResult {
        body: SceneTestResult,
    },
    SendAsync {
        body: CreateRequest,
        response: RpcResultSender<Result<serde_json::Value, String>>,
    },
    SendCommsMessage {
        body: Vec<u8>,
        recipient: NetworkMessageRecipient,
    },
    GetTextureSize {
        src: String,
        response: RpcResultSender<Result<Vector2, String>>,
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
