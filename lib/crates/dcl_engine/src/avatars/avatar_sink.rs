use std::collections::HashSet;

use ethers_core::types::H160;
use glam::Vec3;
use godot::prelude::*;

use crate::{
    avatars::avatar_scene::AvatarScene,
    comms::{
        profile::UserProfile,
        sink::{AvatarSink, SceneSink, SocialLists},
    },
    dcl::components::proto_components::kernel::comms::rfc4,
    godot_classes::{dcl_global::DclGlobal, dcl_social_blacklist::DclSocialBlacklist},
};

fn to_godot_vec3(v: Vec3) -> Vector3 {
    Vector3::new(v.x, v.y, v.z)
}

fn voice_frame(samples: &[i16]) -> PackedVector2Array {
    PackedVector2Array::from_iter(samples.iter().map(|c| {
        let val = (*c as f32) / (i16::MAX as f32);
        Vector2 { x: val, y: val }
    }))
}

impl AvatarSink for Gd<AvatarScene> {
    fn add_avatar(&mut self, alias: u32, address: &H160) {
        self.bind_mut()
            .add_avatar(alias, GString::from(&format!("{:#x}", address)));
    }

    fn remove_avatar(&mut self, alias: u32) {
        self.bind_mut().remove_avatar(alias);
    }

    fn clean(&mut self) {
        self.bind_mut().clean();
    }

    fn update_profile(&mut self, alias: u32, profile: &UserProfile) {
        self.bind_mut().update_avatar_by_alias(alias, profile);
    }

    fn set_profile_fetch_state(&mut self, alias: u32, failures: i32, banned: bool) {
        self.bind_mut()
            .set_avatar_profile_fetch_state(alias, failures, banned);
    }

    fn set_blocked(&mut self, alias: u32, blocked: bool) {
        self.bind_mut().set_avatar_blocked(alias, blocked);
    }

    fn set_version(&mut self, address: &H160, version: &str) {
        if DclGlobal::is_production() {
            return;
        }
        self.bind_mut().set_avatar_version_by_address(
            GString::from(&format!("{:#x}", address)),
            GString::from(version),
        );
    }

    fn update_movement(&mut self, alias: u32, movement: &rfc4::Movement) {
        self.bind_mut()
            .update_avatar_transform_with_movement(alias, movement);
    }

    fn update_position(&mut self, alias: u32, position: &rfc4::Position) {
        self.bind_mut()
            .update_avatar_transform_with_rfc4_position(alias, position);
    }

    fn update_movement_compressed(
        &mut self,
        alias: u32,
        position: Vec3,
        rotation_rad: f32,
        timestamp: f32,
        velocity: Vec3,
        is_grounded: bool,
    ) {
        self.bind_mut()
            .update_avatar_transform_with_movement_compressed(
                alias,
                to_godot_vec3(position),
                rotation_rad,
                timestamp,
                to_godot_vec3(velocity),
                is_grounded,
            );
    }

    fn play_emote(&mut self, alias: u32, incremental_id: u32, urn: &str, mask: i64) {
        self.bind_mut()
            .play_emote(alias, incremental_id, &urn.to_string(), mask);
    }

    fn stop_emote(&mut self, alias: u32) {
        self.bind_mut().stop_emote(alias);
    }

    fn reset_movement_dedup(&mut self, alias: u32) {
        self.bind_mut().reset_movement_dedup(alias);
    }

    fn spawn_voice_channel(
        &mut self,
        alias: u32,
        sample_rate: u32,
        num_channels: u32,
        samples_per_channel: u32,
    ) {
        self.bind_mut()
            .spawn_voice_channel(alias, sample_rate, num_channels, samples_per_channel);
    }

    fn push_voice_frame(&mut self, alias: u32, samples: &[i16]) {
        self.bind_mut()
            .push_voice_frame(alias, voice_frame(samples));
    }
}

/// Fans livekit media out to every scene the global scene runner holds.
pub struct GlobalSceneSink;

impl SceneSink for GlobalSceneSink {
    fn video_frame(&mut self, width: u32, height: u32, data: &[u8]) {
        let mut scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
        for (_, scene) in scene_runner.bind_mut().get_all_scenes_mut().iter_mut() {
            scene.process_livekit_video_frame(width, height, data);
        }
    }

    fn init_audio(&mut self, sample_rate: u32, num_channels: u32, samples_per_channel: u32) {
        let mut scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
        for (_, scene) in scene_runner.bind_mut().get_all_scenes_mut().iter_mut() {
            scene.init_livekit_audio(sample_rate, num_channels, samples_per_channel);
        }
    }

    fn audio_frame(&mut self, samples: &[i16]) {
        let frame = voice_frame(samples);
        let mut scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
        for (_, scene) in scene_runner.bind_mut().get_all_scenes_mut().iter_mut() {
            scene.process_livekit_audio_frame(frame.clone());
        }
    }
}

impl SocialLists for Gd<DclSocialBlacklist> {
    fn sets(&self) -> (HashSet<H160>, HashSet<H160>) {
        let bind = self.bind();
        (bind.get_blocked_set().clone(), bind.get_muted_set().clone())
    }
}
