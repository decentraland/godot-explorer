use godot::prelude::{GodotString, PackedStringArray, ToVariant, Variant, VariantArray};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct AvatarColor3 {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AvatarSnapshots {
    pub face256: String,
    pub body: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AvatarEmote {
    pub slot: u32,
    pub urn: String,
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct AvatarColor {
    pub color: AvatarColor3,
}

impl From<&AvatarColor> for godot::prelude::Color {
    fn from(val: &AvatarColor) -> Self {
        godot::prelude::Color::from_rgb(val.color.r, val.color.g, val.color.b)
    }
}

#[derive(Serialize, Deserialize, Clone)]
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

#[derive(Serialize, Deserialize, Clone)]
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
                    \"face256\":\"QmSqZ2npVD4RLdqe17FzGCFcN29RfvmqmEd2FcQUctxaKk\",
                    \"body\":\"QmSav1o6QK37Jj1yhbmhYk9MJc6c2H5DWbWzPVsg9JLYfF\"
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
            has_claimed_name: Default::default(),
            has_connected_web3: Default::default(),
            avatar,
        }
    }
}

impl SerializedProfile {
    pub fn to_godot_array(&self, base_url: &str) -> [Variant; 8] {
        let name: GodotString = GodotString::from(self.name.as_str());

        let body_shape: GodotString = self
            .avatar
            .body_shape
            .as_ref()
            .unwrap_or(&"default".into())
            .into();

        let eyes: godot::prelude::Color = self
            .avatar
            .eyes
            .as_ref()
            .unwrap_or(&AvatarColor {
                color: AvatarColor3 {
                    r: 0.2,
                    g: 0.2,
                    b: 0.5,
                },
            })
            .into();

        let hair: godot::prelude::Color = self
            .avatar
            .hair
            .as_ref()
            .unwrap_or(&AvatarColor {
                color: AvatarColor3 {
                    r: 0.2,
                    g: 0.2,
                    b: 0.5,
                },
            })
            .into();

        let skin: godot::prelude::Color = self
            .avatar
            .skin
            .as_ref()
            .unwrap_or(&AvatarColor {
                color: AvatarColor3 {
                    r: 0.2,
                    g: 0.2,
                    b: 0.5,
                },
            })
            .into();

        let wearables = self
            .avatar
            .wearables
            .iter()
            .map(GodotString::from)
            .collect::<PackedStringArray>();

        let emotes = self
            .avatar
            .emotes
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .map(|v| {
                let mut arr = VariantArray::new();
                arr.push(GodotString::from(v.urn.as_str()).to_variant());
                arr.push(v.slot.to_variant());
                arr.to_variant()
            })
            .collect::<VariantArray>();

        let base_url = GodotString::from(base_url).to_variant();
        let name = name.to_variant();
        let body_shape = body_shape.to_variant();
        let eyes = eyes.to_variant();
        let hair = hair.to_variant();
        let skin = skin.to_variant();
        let wearables = wearables.to_variant();
        let emotes = emotes.to_variant();

        [
            base_url, name, body_shape, eyes, hair, skin, wearables, emotes,
        ]
    }
}

#[derive(Serialize, Deserialize, Clone)]
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
