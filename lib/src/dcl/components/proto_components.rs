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

    /// Preview-server hot-reload messages (`WsSceneMessage`), decoded by
    /// `DclPreviewMessage` for the preview WebSocket.
    #[allow(clippy::all)]
    pub mod development {
        include!(concat!(env!("OUT_DIR"), "/decentraland.sdk.development.rs"));
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

#[allow(clippy::all)]
pub mod pulse {
    include!(concat!(env!("OUT_DIR"), "/decentraland.pulse.rs"));
    // Quantize/dequantize accessors generated by build_quant.rs from the
    // (decentraland.common.quantized[_power]) field options — see options.proto.
    include!(concat!(env!("OUT_DIR"), "/pulse_quant.rs"));
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

// Exercises the build_quant.rs-generated accessors against the grids the Pulse
// server bakes from the same .proto options — a drift here means wrong world
// positions on the wire with no compile error anywhere else.
#[cfg(test)]
mod pulse_quant_tests {
    use super::pulse;

    fn assert_approx(actual: f32, expected: f32, eps: f32) {
        assert!(
            (actual - expected).abs() <= eps,
            "expected ~{expected}, got {actual} (eps {eps})"
        );
    }

    #[test]
    fn position_grids_match_server_scheme() {
        // position_x/z: 8 bits over [0, 16] → step 16/255; encoded 128 → ≈ 8.031.
        assert_approx(pulse::PlayerState::position_x_step(), 16.0 / 255.0, 1e-6);
        let state = pulse::PlayerState {
            position_x: 128,
            ..Default::default()
        };
        assert_approx(state.position_x_dequantized(), 8.031, 0.001);
        assert_eq!(pulse::PlayerState::position_x_quantized(8.031), 128);

        // position_y: 13 bits over [0, 200] → step ≈ 0.0244.
        assert_approx(pulse::PlayerState::position_y_step(), 200.0 / 8191.0, 1e-6);
    }

    #[test]
    fn linear_roundtrip_error_bounded_by_half_step() {
        for value in [0.0f32, 0.03, 1.0, 7.99, 8.0, 15.97, 16.0] {
            let encoded = pulse::PlayerState::position_x_quantized(value);
            let decoded = pulse::PlayerState {
                position_x: encoded,
                ..Default::default()
            }
            .position_x_dequantized();
            assert_approx(
                decoded,
                value,
                pulse::PlayerState::position_x_step() / 2.0 + 1e-6,
            );
        }
    }

    #[test]
    fn rotation_seven_bits_full_circle() {
        assert_eq!(pulse::PlayerState::rotation_y_quantized(0.0), 0);
        assert_eq!(pulse::PlayerState::rotation_y_quantized(360.0), 127);
        let encoded = pulse::PlayerState::rotation_y_quantized(90.0);
        let decoded = pulse::PlayerState {
            rotation_y: encoded,
            ..Default::default()
        }
        .rotation_y_dequantized();
        assert_approx(decoded, 90.0, 360.0 / 127.0 / 2.0 + 1e-4);
    }

    #[test]
    fn power_law_velocity_zero_is_exact_and_sign_rides_the_lsb() {
        // Exact zero: a stopped peer must decode to exactly 0.0, not the linear
        // quantizer's ±half-step residual (the whole point of quantized_power).
        assert_eq!(pulse::PlayerState::velocity_x_quantized(0.0), 0);
        let stopped = pulse::PlayerState::default();
        assert_eq!(stopped.velocity_x_dequantized(), 0.0);

        // Sign in the LSB: same magnitude bits, opposite sign bit.
        let pos = pulse::PlayerState::velocity_x_quantized(1.0);
        let neg = pulse::PlayerState::velocity_x_quantized(-1.0);
        assert_eq!(pos & 1, 0);
        assert_eq!(neg & 1, 1);
        assert_eq!(pos >> 1, neg >> 1);

        // pow=2 concentrates resolution at low speeds: 1 m/s round-trips tightly...
        let decoded = pulse::PlayerState {
            velocity_x: pos,
            ..Default::default()
        }
        .velocity_x_dequantized();
        assert_approx(decoded, 1.0, 0.05);
        // ...and the extremes clamp to the symmetric ±50 range.
        let max_enc = pulse::PlayerState::velocity_x_quantized(50.0);
        let max_dec = pulse::PlayerState {
            velocity_x: max_enc,
            ..Default::default()
        }
        .velocity_x_dequantized();
        assert_approx(max_dec, 50.0, 1e-3);
        assert_eq!(
            pulse::PlayerState::velocity_x_quantized(999.0),
            max_enc,
            "out-of-range clamps to max"
        );
    }

    #[test]
    fn delta_optional_fields_dequantize_to_option() {
        let delta = pulse::PlayerStateDeltaTier0 {
            position_x: Some(128),
            ..Default::default()
        };
        assert_approx(delta.position_x_dequantized().unwrap(), 8.031, 0.001);
        assert_eq!(delta.position_y_dequantized(), None);
        assert_eq!(delta.velocity_x_dequantized(), None);
    }

    #[test]
    fn point_at_covers_signed_world_span() {
        // 17 bits over [-3000, 3000] → step ≈ 0.0458 m.
        assert_approx(
            pulse::PlayerState::point_at_x_step(),
            6000.0 / 131071.0,
            1e-6,
        );
        assert_eq!(pulse::PlayerState::point_at_x_quantized(-3000.0), 0);
        let origin = pulse::PlayerState::point_at_x_quantized(0.0);
        let decoded = pulse::PlayerState {
            point_at_x: Some(origin),
            ..Default::default()
        }
        .point_at_x_dequantized()
        .unwrap();
        assert_approx(
            decoded,
            0.0,
            pulse::PlayerState::point_at_x_step() / 2.0 + 1e-4,
        );
    }

    #[test]
    fn envelopes_roundtrip_through_prost() {
        use prost::Message;

        let msg = pulse::ClientMessage {
            message: Some(pulse::client_message::Message::Teleport(
                pulse::TeleportRequest {
                    parcel_index: 54858,
                    position_x: pulse::TeleportRequest::position_x_quantized(1.0),
                    position_y: pulse::TeleportRequest::position_y_quantized(2.0),
                    position_z: pulse::TeleportRequest::position_z_quantized(3.0),
                    realm: "main".into(),
                },
            )),
        };
        let bytes = msg.encode_to_vec();
        let decoded = pulse::ClientMessage::decode(bytes.as_slice()).unwrap();
        assert_eq!(msg, decoded);
    }
}
