//! The avatar node. `DclAvatar` (godot_classes/dcl_avatar.rs) holds the state; the behaviors
//! are `impl DclAvatar` blocks split by concern in the submodules here.

pub mod anchors;
pub mod assembly;
pub mod build;
pub mod emote;
pub mod emote_load;
pub mod glider;
pub mod gpu_ready_gate;
pub mod ik;
pub mod lifecycle;
pub mod locomotion;
pub mod lod;
pub mod nametag;
pub mod script_api;
pub mod spawn_reveal;

use godot::classes::{
    AnimationPlayer, AnimationTree, Area3D, AudioStream, AudioStreamPlayer, AudioStreamPlayer3D,
    Skeleton3D, Sprite3D, StaticBody3D, SubViewport, Timer,
};
use godot::prelude::*;

use crate::avatars::modifier_detector::AvatarModifierAreaDetector;
use crate::avatars::nameplate::NicknameUi;

/// The emote sound player: 3D, or plain when the avatar plays non-spatial audio.
pub enum EmoteAudio {
    Spatial(Gd<AudioStreamPlayer3D>),
    Plain(Gd<AudioStreamPlayer>),
}

impl EmoteAudio {
    pub fn stop(&mut self) {
        match self {
            Self::Spatial(player) => player.stop(),
            Self::Plain(player) => player.stop(),
        }
    }

    pub fn play(&mut self, stream: Gd<AudioStream>) {
        match self {
            Self::Spatial(player) => {
                player.set_stream(&stream);
                player.play();
            }
            Self::Plain(player) => {
                player.set_stream(&stream);
                player.play();
            }
        }
    }
}

/// Nodes of `avatar.tscn` the behaviors drive, resolved once in `ready`.
pub struct AvatarNodes {
    pub animation_tree: Gd<AnimationTree>,
    pub animation_player: Gd<AnimationPlayer>,
    pub skeleton: Gd<Skeleton3D>,
    pub audio_emote: EmoteAudio,
    pub audio_double_jump: Gd<AudioStreamPlayer3D>,
    pub nickname_ui: Gd<NicknameUi>,
    pub nickname_quad: Gd<Sprite3D>,
    /// Freed by `NameplateLayer::attach`: the plate lives in the shared layer instead.
    pub nickname_viewport: Option<Gd<SubViewport>>,
    pub timer_hide_mic: Gd<Timer>,
    pub click_area: Gd<Area3D>,
    /// Freed for scene NPCs, which have no trigger detection.
    pub trigger_detector: Option<Gd<StaticBody3D>>,
    pub glider_prop: Gd<Node3D>,
    pub modifier_detector: Gd<AvatarModifierAreaDetector>,
}

/// A network emote that arrived while the avatar was still loading (Pulse replays the peer's
/// last emote at join): playing it earlier would resolve against the default body shape and be
/// wiped by the rebuild, so it is replayed when the avatar is ready.
pub struct PendingEmote {
    pub urn: String,
    pub mask: i64,
    pub scene_id: i32,
}
