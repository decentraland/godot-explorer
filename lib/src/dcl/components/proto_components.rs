use sdk::components::TextWrap;

pub mod sdk {
    #[allow(clippy::all)]
    pub mod components {
        include!(concat!(env!("OUT_DIR"), "/decentraland.sdk.components.rs"));

        pub mod common {
            include!(concat!(
                env!("OUT_DIR"),
                "/decentraland.sdk.components.common.rs"
            ));
        }
    }
}

pub mod common {
    include!(concat!(env!("OUT_DIR"), "/decentraland.common.rs"));

    impl Color4 {
        pub fn black() -> Self {
            Self {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            }
        }
        pub fn white() -> Self {
            Self {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            }
        }
        pub fn to_godot(&self) -> godot::prelude::Color {
            godot::prelude::Color::from_rgba(self.r, self.g, self.b, self.a)
        }

        pub fn to_color_string(&self) -> String {
            format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                (self.r * 255.0) as u8,
                (self.g * 255.0) as u8,
                (self.b * 255.0) as u8,
                (self.a * 255.0) as u8
            )
        }

        pub fn multiply(&mut self, factor: f32) -> Self {
            Self {
                r: self.r * factor,
                g: self.g * factor,
                b: self.b * factor,
                a: self.a * factor,
            }
        }
    }

    impl Color3 {
        pub fn black() -> Self {
            Self {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            }
        }
        pub fn white() -> Self {
            Self {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            }
        }
        pub fn to_godot(&self) -> godot::prelude::Color {
            godot::prelude::Color::from_rgba(self.r, self.g, self.b, 1.0)
        }

        pub fn to_color_string(&self) -> String {
            format!(
                "#{:02x}{:02x}{:02x}",
                (self.r * 255.0) as u8,
                (self.g * 255.0) as u8,
                (self.b * 255.0) as u8
            )
        }

        pub fn multiply(&mut self, factor: f32) -> Self {
            Self {
                r: self.r * factor,
                g: self.g * factor,
                b: self.b * factor,
            }
        }
    }

    impl Vector3 {
        pub fn to_godot(&self) -> godot::prelude::Vector3 {
            godot::prelude::Vector3::new(self.x, self.y, self.z)
        }
    }

    impl Position {
        pub fn to_godot(&self) -> godot::prelude::Vector3 {
            godot::prelude::Vector3::new(self.x, self.y, self.z)
        }
    }

    impl Vector2 {
        pub fn to_godot(&self) -> godot::prelude::Vector2 {
            godot::prelude::Vector2::new(self.x, self.y)
        }
    }

    impl Quaternion {
        pub fn to_godot(&self) -> godot::prelude::Quaternion {
            godot::prelude::Quaternion::new(self.x, self.y, self.z, self.w)
        }
    }
}

pub trait WrapToGodot<T> {
    fn to_godot_or_else(&self, default: T) -> T;
}

impl WrapToGodot<godot::prelude::Color> for Option<common::Color4> {
    fn to_godot_or_else(&self, default: godot::prelude::Color) -> godot::prelude::Color {
        self.as_ref().map(|v| v.to_godot()).unwrap_or(default)
    }
}

impl WrapToGodot<godot::prelude::Color> for Option<common::Color3> {
    fn to_godot_or_else(&self, default: godot::prelude::Color) -> godot::prelude::Color {
        self.as_ref().map(|v| v.to_godot()).unwrap_or(default)
    }
}

impl sdk::components::common::TextAlignMode {
    pub fn to_godot(
        &self,
    ) -> (
        godot::global::HorizontalAlignment,
        godot::global::VerticalAlignment,
    ) {
        match self {
            sdk::components::common::TextAlignMode::TamTopLeft => (
                godot::global::HorizontalAlignment::LEFT,
                godot::global::VerticalAlignment::TOP,
            ),
            sdk::components::common::TextAlignMode::TamTopCenter => (
                godot::global::HorizontalAlignment::CENTER,
                godot::global::VerticalAlignment::TOP,
            ),
            sdk::components::common::TextAlignMode::TamTopRight => (
                godot::global::HorizontalAlignment::RIGHT,
                godot::global::VerticalAlignment::TOP,
            ),
            sdk::components::common::TextAlignMode::TamMiddleLeft => (
                godot::global::HorizontalAlignment::LEFT,
                godot::global::VerticalAlignment::CENTER,
            ),
            sdk::components::common::TextAlignMode::TamMiddleCenter => (
                godot::global::HorizontalAlignment::CENTER,
                godot::global::VerticalAlignment::CENTER,
            ),
            sdk::components::common::TextAlignMode::TamMiddleRight => (
                godot::global::HorizontalAlignment::RIGHT,
                godot::global::VerticalAlignment::CENTER,
            ),
            sdk::components::common::TextAlignMode::TamBottomLeft => (
                godot::global::HorizontalAlignment::LEFT,
                godot::global::VerticalAlignment::BOTTOM,
            ),
            sdk::components::common::TextAlignMode::TamBottomCenter => (
                godot::global::HorizontalAlignment::CENTER,
                godot::global::VerticalAlignment::BOTTOM,
            ),
            sdk::components::common::TextAlignMode::TamBottomRight => (
                godot::global::HorizontalAlignment::RIGHT,
                godot::global::VerticalAlignment::BOTTOM,
            ),
        }
    }
}

impl sdk::components::PbAnimationState {
    pub fn playing_backward(&self) -> bool {
        self.speed() < 0.0
    }
}

impl sdk::components::PbUiText {
    pub fn text_wrap_compat(&self) -> TextWrap {
        if self.text_wrap.is_none() {
            return TextWrap::TwNoWrap;
        }

        self.text_wrap()
    }
}

pub mod kernel {
    #[allow(clippy::all)]
    pub mod comms {
        pub mod rfc5 {
            include!(concat!(
                env!("OUT_DIR"),
                "/decentraland.kernel.comms.rfc5.rs"
            ));
        }
        pub mod rfc4 {
            include!(concat!(
                env!("OUT_DIR"),
                "/decentraland.kernel.comms.rfc4.rs"
            ));
        }
        pub mod v3 {
            include!(concat!(env!("OUT_DIR"), "/decentraland.kernel.comms.v3.rs"));
        }
    }
}

pub mod social_service {
    // Include the error types from the social_service package
    include!(concat!(env!("OUT_DIR"), "/decentraland.social_service.rs"));

    #[allow(clippy::all)]
    pub mod v2 {
        include!(concat!(
            env!("OUT_DIR"),
            "/decentraland.social_service.v2.rs"
        ));
    }
}

/// Deserialize a component's binary data to JSON.
///
/// Only available when the `scene_logging` feature is enabled.
#[cfg(feature = "scene_logging")]
pub fn deserialize_component_to_json(component_id: u32, data: &[u8]) -> Option<serde_json::Value> {
    use prost::Message;
    use sdk::components::*;

    // Helper macro to decode and serialize a component
    macro_rules! decode_component {
        ($type:ty) => {{
            <$type>::decode(data)
                .ok()
                .and_then(|v| serde_json::to_value(v).ok())
        }};
    }

    match component_id {
        // Transform is a custom binary format, not proto
        1 => deserialize_transform(data),
        1017 => decode_component!(PbMaterial),
        1018 => decode_component!(PbMeshRenderer),
        1019 => decode_component!(PbMeshCollider),
        1020 => decode_component!(PbAudioSource),
        1021 => decode_component!(PbAudioStream),
        1030 => decode_component!(PbTextShape),
        1040 => decode_component!(PbNftShape),
        1041 => decode_component!(PbGltfContainer),
        1042 => decode_component!(PbAnimator),
        1043 => decode_component!(PbVideoPlayer),
        1044 => decode_component!(PbVideoEvent),
        1048 => decode_component!(PbEngineInfo),
        1049 => decode_component!(PbGltfContainerLoadingState),
        1050 => decode_component!(PbUiTransform),
        1052 => decode_component!(PbUiText),
        1053 => decode_component!(PbUiBackground),
        1054 => decode_component!(PbUiCanvasInformation),
        1060 => decode_component!(PbTriggerArea),
        1061 => decode_component!(PbTriggerAreaResult),
        1062 => decode_component!(PbPointerEvents),
        1063 => decode_component!(PbPointerEventsResult),
        1067 => decode_component!(PbRaycast),
        1068 => decode_component!(PbRaycastResult),
        1070 => decode_component!(PbAvatarModifierArea),
        1071 => decode_component!(PbCameraModeArea),
        1072 => decode_component!(PbCameraMode),
        1073 => decode_component!(PbAvatarAttach),
        1074 => decode_component!(PbPointerLock),
        1075 => decode_component!(PbMainCamera),
        1076 => decode_component!(PbVirtualCamera),
        1078 => decode_component!(PbInputModifier),
        1079 => decode_component!(PbLightSource),
        1080 => decode_component!(PbAvatarShape),
        1081 => decode_component!(PbVisibilityComponent),
        1087 => decode_component!(PbAvatarBase),
        1088 => decode_component!(PbAvatarEmoteCommand),
        1089 => decode_component!(PbPlayerIdentityData),
        1090 => decode_component!(PbBillboard),
        1091 => decode_component!(PbAvatarEquippedData),
        1093 => decode_component!(PbUiInput),
        1094 => decode_component!(PbUiDropdown),
        1095 => decode_component!(PbUiInputResult),
        1096 => decode_component!(PbUiDropdownResult),
        1097 => decode_component!(PbMapPin),
        1099 => decode_component!(PbGltfNodeModifiers),
        1102 => decode_component!(PbTween),
        1103 => decode_component!(PbTweenState),
        1104 => decode_component!(PbTweenSequence),
        1105 => decode_component!(PbAudioEvent),
        1106 => decode_component!(PbRealmInfo),
        1200 => decode_component!(PbGltfNode),
        1201 => decode_component!(PbGltfNodeState),
        1202 => decode_component!(PbUiScrollResult),
        1203 => decode_component!(PbUiCanvas),
        1206 => decode_component!(PbGlobalLight),
        1207 => decode_component!(PbTextureCamera),
        1208 => decode_component!(PbCameraLayers),
        1209 => decode_component!(PbPrimaryPointerInfo),
        1210 => decode_component!(PbSkyboxTime),
        1211 => decode_component!(PbCameraLayer),
        _ => None,
    }
}

/// Deserialize Transform component (custom binary format, not proto).
/// Format: translation(Vec3) + rotation(Quat) + scale(Vec3) + parent(EntityId)
/// = 12 + 16 + 12 + 4 = 44 bytes
#[cfg(feature = "scene_logging")]
fn deserialize_transform(data: &[u8]) -> Option<serde_json::Value> {
    if data.len() < 44 {
        return None;
    }

    // Read translation (3 floats)
    let tx = f32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let ty = f32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let tz = f32::from_le_bytes([data[8], data[9], data[10], data[11]]);

    // Read rotation (4 floats - quaternion)
    let rx = f32::from_le_bytes([data[12], data[13], data[14], data[15]]);
    let ry = f32::from_le_bytes([data[16], data[17], data[18], data[19]]);
    let rz = f32::from_le_bytes([data[20], data[21], data[22], data[23]]);
    let rw = f32::from_le_bytes([data[24], data[25], data[26], data[27]]);

    // Read scale (3 floats)
    let sx = f32::from_le_bytes([data[28], data[29], data[30], data[31]]);
    let sy = f32::from_le_bytes([data[32], data[33], data[34], data[35]]);
    let sz = f32::from_le_bytes([data[36], data[37], data[38], data[39]]);

    // Read parent entity ID (u16 number + u16 version = 4 bytes)
    let parent_number = u16::from_le_bytes([data[40], data[41]]);
    let parent_version = u16::from_le_bytes([data[42], data[43]]);
    let parent = ((parent_version as u32) << 16) | (parent_number as u32);

    Some(serde_json::json!({
        "position": { "x": tx, "y": ty, "z": tz },
        "rotation": { "x": rx, "y": ry, "z": rz, "w": rw },
        "scale": { "x": sx, "y": sy, "z": sz },
        "parent": parent
    }))
}
