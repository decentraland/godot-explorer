use godot::prelude::*;
use serde::{Deserialize, Serialize};

use crate::dcl::components::proto_components::{
    common::Color3,
    sdk::components::{PbAvatarBase, PbAvatarEquippedData, PbPlayerIdentityData},
};

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub struct AvatarColor3 {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AvatarSnapshots {
    pub face256: String,
    pub body: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AvatarEmote {
    pub slot: u32,
    pub urn: String,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub struct AvatarColor {
    pub color: AvatarColor3,
}

impl From<&AvatarColor> for godot::prelude::Color {
    fn from(val: &AvatarColor) -> Self {
        godot::prelude::Color::from_rgb(val.color.r, val.color.g, val.color.b)
    }
}

impl From<&AvatarColor> for Color3 {
    fn from(val: &AvatarColor) -> Self {
        Color3 {
            r: val.color.r,
            g: val.color.g,
            b: val.color.b,
        }
    }
}

impl From<&godot::prelude::Color> for AvatarColor {
    fn from(val: &godot::prelude::Color) -> Self {
        AvatarColor {
            color: AvatarColor3 {
                r: val.r,
                g: val.g,
                b: val.b,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AvatarWireFormat {
    pub name: Option<String>,
    #[serde(rename = "bodyShape")]
    pub body_shape: Option<String>,
    pub eyes: Option<AvatarColor>,
    pub hair: Option<AvatarColor>,
    pub skin: Option<AvatarColor>,
    pub wearables: Vec<String>,
    pub emotes: Option<Vec<AvatarEmote>>,
    pub snapshots: Option<AvatarSnapshots>,
}

#[derive(Deserialize)]
pub struct LambdaProfiles {
    pub avatars: Vec<SerializedProfile>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SerializedProfile {
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    pub name: String,
    pub description: String,
    pub version: i64,
    #[serde(rename = "ethAddress")]
    pub eth_address: String,
    #[serde(rename = "tutorialStep")]
    pub tutorial_step: u32,
    pub email: Option<String>,
    pub blocked: Option<Vec<String>>,
    pub muted: Option<Vec<String>>,
    pub interests: Option<Vec<String>>,
    #[serde(rename = "hasClaimedName")]
    pub has_claimed_name: Option<bool>,
    #[serde(rename = "hasConnectedWeb3")]
    pub has_connected_web3: Option<bool>,
    pub avatar: AvatarWireFormat,
}

impl Default for SerializedProfile {
    fn default() -> Self {
        let avatar = serde_json::from_str("
            {
                \"bodyShape\":\"urn:decentraland:off-chain:base-avatars:BaseFemale\",
                \"wearables\":[
                    \"urn:decentraland:off-chain:base-avatars:f_sweater\",
                    \"urn:decentraland:off-chain:base-avatars:f_jeans\",
                    \"urn:decentraland:off-chain:base-avatars:bun_shoes\",
                    \"urn:decentraland:off-chain:base-avatars:standard_hair\",
                    \"urn:decentraland:off-chain:base-avatars:f_eyes_01\",
                    \"urn:decentraland:off-chain:base-avatars:f_eyebrows_00\",
                    \"urn:decentraland:off-chain:base-avatars:f_mouth_00\"
                ],
                \"snapshots\": {
                    \"body\": \"bafkreigxesh5owgy4vreca65nh33zqw7br6haokkltmzg3mn22g5whcfbq\",
                    \"face256\": \"bafkreibykc3l7ai5z5zik7ypxlqetgtmiepr42al6jcn4yovdgezycwa2y\"
                },
                \"eyes\":{
                    \"color\":{\"r\":0.3,\"g\":0.2235294133424759,\"b\":0.99,\"a\":1}
                },
                \"hair\":{
                    \"color\":{\"r\":0.5960784554481506,\"g\":0.37254902720451355,\"b\":0.21568627655506134,\"a\":1}
                },
                \"skin\":{
                    \"color\":{\"r\":0.4901960790157318,\"g\":0.364705890417099,\"b\":0.27843138575553894,\"a\":1}
                }
            }
        ").unwrap();

        Self {
            user_id: Default::default(),
            name: "Godot User".to_string(),
            description: Default::default(),
            version: 1,
            eth_address: "0x0000000000000000000000000000000000000000".to_owned(),
            tutorial_step: Default::default(),
            email: Default::default(),
            blocked: Default::default(),
            muted: Default::default(),
            interests: Default::default(),
            has_claimed_name: Some(false),
            has_connected_web3: Some(false),
            avatar,
        }
    }
}

impl SerializedProfile {
    pub fn to_pb_avatar_base(&self) -> PbAvatarBase {
        PbAvatarBase {
            skin_color: self.avatar.skin.map(|c| Color3::from(&c)),
            eyes_color: self.avatar.eyes.map(|c| Color3::from(&c)),
            hair_color: self.avatar.hair.map(|c| Color3::from(&c)),
            body_shape_urn: self
                .avatar
                .body_shape
                .as_deref()
                .map(ToString::to_string)
                .unwrap_or("urn:decentraland:off-chain:base-avatars:BaseFemale".to_owned()),
            name: self.avatar.name.as_deref().unwrap_or("???").to_owned(),
        }
    }
    pub fn to_pb_player_identity_data(&self) -> PbPlayerIdentityData {
        PbPlayerIdentityData {
            address: self.user_id.clone().unwrap_or("unknown".to_owned()),
            is_guest: !self.has_connected_web3.as_ref().unwrap_or(&false),
        }
    }
    pub fn to_pb_avatar_equipped_data(&self) -> PbAvatarEquippedData {
        PbAvatarEquippedData {
            wearable_urns: self.avatar.wearables.to_vec(),
            emotes_urns: self
                .avatar
                .emotes
                .as_ref()
                .unwrap_or(&Vec::default())
                .iter()
                .map(|emote| emote.urn.clone())
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UserProfile {
    pub version: u32,
    pub content: SerializedProfile,
    pub base_url: String,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            base_url: "https://peer.decentraland.zone/content/contents/".to_owned(),
            version: 1,
            content: SerializedProfile::default(),
        }
    }
}

impl UserProfile {
    pub fn from_godot_dictionary(dictionary: &Dictionary) -> Self {
        let value = godot::engine::Json::stringify(dictionary.to_variant());
        serde_json::from_str(value.to_string().as_str()).unwrap_or_default()
    }

    pub fn to_godot_dictionary(&self) -> Dictionary {
        let value = serde_json::to_string(self).unwrap_or_default();
        let value = godot::engine::Json::parse_string(value.into());
        value.to::<Dictionary>()
    }
}
