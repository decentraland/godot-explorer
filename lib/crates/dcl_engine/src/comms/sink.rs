use std::collections::HashSet;

use ethers_core::types::H160;
use glam::Vec3;

use crate::{comms::profile::UserProfile, dcl::components::proto_components::kernel::comms::rfc4};

/// What the comms layer drives on the avatars; the engine implements it over `AvatarScene`.
pub trait AvatarSink {
    fn add_avatar(&mut self, alias: u32, address: &H160);
    fn remove_avatar(&mut self, alias: u32);
    fn clean(&mut self);
    fn update_profile(&mut self, alias: u32, profile: &UserProfile);
    fn set_profile_fetch_state(&mut self, alias: u32, failures: i32, banned: bool);
    fn set_blocked(&mut self, alias: u32, blocked: bool);
    /// Shown as a label on non-production builds only; the sink decides.
    fn set_version(&mut self, address: &H160, version: &str);
    fn update_movement(&mut self, alias: u32, movement: &rfc4::Movement);
    fn update_position(&mut self, alias: u32, position: &rfc4::Position);
    fn update_movement_compressed(
        &mut self,
        alias: u32,
        position: Vec3,
        rotation_rad: f32,
        timestamp: f32,
        velocity: Vec3,
        is_grounded: bool,
    );
    /// `mask` uses the internal convention: -1 = full body, 0 = upper body.
    fn play_emote(&mut self, alias: u32, incremental_id: u32, urn: &str, mask: i64);
    fn stop_emote(&mut self, alias: u32);
    fn reset_movement_dedup(&mut self, alias: u32);
    fn spawn_voice_channel(
        &mut self,
        alias: u32,
        sample_rate: u32,
        num_channels: u32,
        samples_per_channel: u32,
    );
    fn push_voice_frame(&mut self, alias: u32, samples: &[i16]);
}

/// Livekit video and streamer audio, fanned out to every running scene.
pub trait SceneSink {
    fn video_frame(&mut self, width: u32, height: u32, data: &[u8]);
    fn init_audio(&mut self, sample_rate: u32, num_channels: u32, samples_per_channel: u32);
    fn audio_frame(&mut self, samples: &[i16]);
}

pub trait SocialLists {
    /// `(blocked, muted)`.
    fn sets(&self) -> (HashSet<H160>, HashSet<H160>);
}
