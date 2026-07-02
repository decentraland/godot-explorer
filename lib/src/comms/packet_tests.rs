//! Encode/decode round-trip tests for the RFC4 comms wire format.
//!
//! Every comms packet crosses the wire as a `prost`-encoded `rfc4::Packet`
//! (`packet.encode(buf)` on send in `ws_room.rs`/`livekit.rs`, `Packet::decode`
//! on receive). These tests guard against silent protobuf drift — e.g. a field
//! number changing or a `oneof` arm being dropped — by asserting each packet we
//! build survives a full encode→decode cycle unchanged.

use crate::dcl::components::proto_components::kernel::comms::rfc4;
use prost::Message;

fn round_trip(packet: rfc4::Packet) -> rfc4::Packet {
    let mut buf = Vec::new();
    packet.encode(&mut buf).expect("encode");
    rfc4::Packet::decode(buf.as_slice()).expect("decode")
}

fn pkt(msg: rfc4::packet::Message) -> rfc4::Packet {
    rfc4::Packet {
        message: Some(msg),
        protocol_version: 100,
    }
}

#[test]
fn position_round_trip() {
    let original = pkt(rfc4::packet::Message::Position(rfc4::Position {
        index: 7,
        position_x: 1.5,
        position_y: 2.5,
        position_z: -3.5,
        rotation_x: 0.0,
        rotation_y: std::f32::consts::FRAC_1_SQRT_2,
        rotation_z: 0.0,
        rotation_w: std::f32::consts::FRAC_1_SQRT_2,
    }));
    assert_eq!(round_trip(original.clone()), original);
}

#[test]
fn movement_round_trip() {
    let original = pkt(rfc4::packet::Message::Movement(rfc4::Movement {
        timestamp: 12.34,
        position_x: 10.0,
        position_y: 1.0,
        position_z: -20.0,
        velocity_x: 1.0,
        velocity_y: 0.0,
        velocity_z: -2.0,
        rotation_y: 270.0,
        movement_blend_value: 1.0,
        slide_blend_value: 0.0,
        is_grounded: true,
        is_jumping: false,
        jump_count: 1,
        is_long_jump: false,
        is_long_fall: false,
        is_falling: false,
        is_stunned: false,
        glide_state: 0,
        is_instant: false,
        is_emoting: false,
        head_ik_yaw_enabled: false,
        head_ik_pitch_enabled: false,
        head_yaw: 0.0,
        head_pitch: 0.0,
        point_at_x: 0.0,
        point_at_y: 0.0,
        point_at_z: 0.0,
        is_pointing_at: false,
    }));
    assert_eq!(round_trip(original.clone()), original);
}

#[test]
fn chat_round_trip() {
    let original = pkt(rfc4::packet::Message::Chat(rfc4::Chat {
        message: "héllo 🌐 world".to_string(),
        timestamp: 1234.5,
    }));
    assert_eq!(round_trip(original.clone()), original);
}

#[test]
fn player_emote_round_trip() {
    // The exact shape that the id==0 dedup bug was about.
    for &(id, urn) in &[
        (0u32, "urn:decentraland:off-chain:base-emotes:wave"),
        (9, "urn:decentraland:matic:collections-v2:0xabc:0:1"),
    ] {
        let original = pkt(rfc4::packet::Message::PlayerEmote(rfc4::PlayerEmote {
            incremental_id: id,
            urn: urn.to_string(),
            timestamp: 42.0,
        }));
        assert_eq!(round_trip(original.clone()), original);
    }
}

#[test]
fn profile_version_and_response_round_trip() {
    let version = pkt(rfc4::packet::Message::ProfileVersion(
        rfc4::AnnounceProfileVersion { profile_version: 5 },
    ));
    assert_eq!(round_trip(version.clone()), version);

    let response = pkt(rfc4::packet::Message::ProfileResponse(
        rfc4::ProfileResponse {
            serialized_profile: "{\"name\":\"guest\"}".to_string(),
            base_url: "https://peer.decentraland.org/content".to_string(),
        },
    ));
    assert_eq!(round_trip(response.clone()), response);
}

#[test]
fn scene_message_round_trip() {
    let original = pkt(rfc4::packet::Message::Scene(rfc4::Scene {
        scene_id: "bafkrei-test".to_string(),
        data: vec![1, 2, 3, 4, 250, 0, 255],
    }));
    assert_eq!(round_trip(original.clone()), original);
}

#[test]
fn protocol_version_is_preserved() {
    let original = rfc4::Packet {
        message: Some(rfc4::packet::Message::Chat(rfc4::Chat {
            message: "hi".to_string(),
            timestamp: 1.0,
        })),
        protocol_version: 100,
    };
    assert_eq!(round_trip(original.clone()).protocol_version, 100);
}
