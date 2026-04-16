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

/// Deserialize a component's binary data to JSON. Used by the runtime scene
/// logger when a debugged scene receives a CRDT message.
///
/// Transform (id=1) uses a custom binary format and is handled inline; every
/// other id is delegated to the generated `deserialize_proto_component_to_json`
/// (see `build.rs`), so the dispatch table stays in sync with the .proto sources
/// automatically.
pub fn deserialize_component_to_json(component_id: u32, data: &[u8]) -> Option<serde_json::Value> {
    match component_id {
        1 => deserialize_transform(data),
        _ => deserialize_proto_component_to_json(component_id, data),
    }
}

include!(concat!(env!("OUT_DIR"), "/deserialize_component.gen.rs"));

/// Deserialize Transform component (custom binary format, not proto).
/// Format: translation(Vec3) + rotation(Quat) + scale(Vec3) + parent(EntityId)
/// = 12 + 16 + 12 + 4 = 44 bytes
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

    // serde_json refuses NaN/Infinity floats; reject the whole transform rather
    // than emitting an invalid log entry that would later fail to serialize.
    let floats = [tx, ty, tz, rx, ry, rz, rw, sx, sy, sz];
    if floats.iter().any(|v| !v.is_finite()) {
        return None;
    }

    Some(serde_json::json!({
        "position": { "x": tx, "y": ty, "z": tz },
        "rotation": { "x": rx, "y": ry, "z": rz, "w": rw },
        "scale": { "x": sx, "y": sy, "z": sz },
        "parent": parent
    }))
}
