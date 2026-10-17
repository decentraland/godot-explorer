//! Emote playback: full-body emotes through the locomotion state machine, upper-body (masked)
//! emotes on a OneShot layer above it, and the exactly-once started/finished reporting.

use std::collections::HashMap;

use godot::classes::animation::LoopMode;
use godot::classes::{
    AnimationLibrary, AnimationNodeAnimation, AnimationNodeBlendTree, AnimationNodeOneShot,
    AnimationNodeStateMachine, AnimationNodeStateMachinePlayback, Engine, Input, Time,
};
use godot::prelude::*;

use crate::avatars::emotes;
use crate::avatars::loaders::EmoteLoader;
use crate::fetcher::globals;
use crate::godot_classes::dcl_avatar::DclAvatar;

/// Cooldown against emote spam, which can crash the animation system.
pub const EMOTE_COOLDOWN_SECONDS: f64 = 0.5;
pub const MAX_DEFERRED_RETRIES: u32 = 3;
/// After a teleport an emote is not cancelled by movement for this long.
pub const TELEPORT_GRACE_SECONDS: f64 = 0.5;
/// `AvatarMask.AM_UPPER_BODY` in the internal mask convention (-1 is full body).
pub const MASK_UPPER_BODY: i64 = 0;

const MASKED_ACTIVE_PARAM: &str = "parameters/MaskedOneShot/active";
const MASKED_REQUEST_PARAM: &str = "parameters/MaskedOneShot/request";
const LOCOMOTION_PLAYBACK: &str = "parameters/Locomotion/playback";
const IDLE_ANIM: &str = "idle/Anim";

const ONE_SHOT_FIRE: i32 = 1;
const ONE_SHOT_ABORT: i32 = 2;
const ONE_SHOT_FADE_OUT: i32 = 3;

/// Bones an upper-body emote must not drive: the lower body stays with locomotion. Matched on
/// the bone name, not the track path, so a scene emote with another armature prefix filters too.
const MASKED_EXCLUDED_BONES: [&str; 9] = [
    "Avatar_Hips",
    "Avatar_LeftUpLeg",
    "Avatar_LeftLeg",
    "Avatar_LeftFoot",
    "Avatar_LeftToeBase",
    "Avatar_RightUpLeg",
    "Avatar_RightLeg",
    "Avatar_RightFoot",
    "Avatar_RightToeBase",
];

/// Whether the masked layer must not drive this track: root motion, and the lower body.
pub fn is_excluded_from_mask(track_path: &str) -> bool {
    match track_path.rfind(':') {
        None => !track_path.starts_with("Armature_Prop"),
        Some(colon) => MASKED_EXCLUDED_BONES.contains(&&track_path[colon + 1..]),
    }
}

pub struct EmoteItemData {
    pub urn: String,
    pub default_anim_name: String,
    pub file_hash: String,
    pub armature_prop: Option<Gd<Node3D>>,
    pub from_scene: bool,
    pub looping: bool,
}

/// The animation nodes the emote layer rewrites, taken from the avatar's own copy of the tree.
pub struct EmoteNodes {
    pub single: Gd<AnimationNodeAnimation>,
    pub mix_a: Gd<AnimationNodeAnimation>,
    pub mix_b: Gd<AnimationNodeAnimation>,
    pub masked: Gd<AnimationNodeAnimation>,
    pub one_shot: Gd<AnimationNodeOneShot>,
    pub library: Gd<AnimationLibrary>,
}

#[derive(Default)]
pub struct EmoteState {
    pub loader: EmoteLoader,
    pub nodes: Option<EmoteNodes>,
    pub loaded: HashMap<String, EmoteItemData>,

    pub playing_single: bool,
    pub playing_mixed: bool,
    pub playing_loop: bool,
    /// An upper-body emote plays on the OneShot layer. Exclusive with single/mixed.
    pub playing_masked: bool,

    /// Urn or utility id of the started emote, empty when none: each started emote reports
    /// exactly one terminal state.
    pub current_urn: String,
    pub current_mask: i64,
    /// Numeric SceneId that asked for it, -1 when nobody owns it.
    pub current_scene_id: i32,
    /// A scene-owned masked emote parked because the player left the owning scene.
    pub masked_suspended: bool,

    pub masked_fired_frame: i64,
    pub masked_filter_paths: Vec<NodePath>,

    pub is_modifying: bool,
    pub queued_urn: String,
    pub last_emote_time: f64,
    pub grace_until: f64,
    pub is_loading: bool,
    /// Bumped by `stop_emote`: a load that finished after a stop must not start playing.
    pub stop_generation: u64,
    pub prop_armature_names: Vec<String>,
    pub deferred_retry_count: u32,
}

impl EmoteState {
    pub fn new() -> Self {
        Self {
            current_mask: -1,
            current_scene_id: -1,
            masked_fired_frame: -1,
            ..Default::default()
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing_single || self.playing_mixed || self.playing_masked
    }
}

fn now_seconds() -> f64 {
    Time::singleton().get_ticks_msec() as f64 / 1000.0
}

fn animation_node(
    node: Option<Gd<godot::classes::AnimationNode>>,
) -> Option<Gd<AnimationNodeAnimation>> {
    node?.try_cast().ok()
}

impl DclAvatar {
    /// Takes the avatar's own copy of the animation tree and points every emote node at the
    /// idle clip, so the tree always evaluates something that exists.
    pub(crate) fn setup_emotes(&mut self) {
        let Some(nodes) = self.nodes.as_ref() else {
            return;
        };
        let mut tree = nodes.animation_tree.clone();
        let mut player = nodes.animation_player.clone();

        // "Local to scene" does not reach the independent nodes (godot#82421): copy the tree.
        let Some(root) = tree.get_tree_root() else {
            godot_error!("avatar: AnimationTree has no root");
            return;
        };
        let root = root.duplicate_resource_ex().deep_internal().done();
        tree.set_tree_root(&root);
        let Ok(root) = root.try_cast::<AnimationNodeBlendTree>() else {
            godot_error!("avatar: AnimationTree root is not a blend tree");
            return;
        };

        let locomotion = root
            .get_node("Locomotion")
            .and_then(|n| n.try_cast::<AnimationNodeStateMachine>().ok());
        let (Some(locomotion), Some(masked), Some(one_shot)) = (
            locomotion,
            animation_node(root.get_node("EmoteMasked")),
            root.get_node("MaskedOneShot")
                .and_then(|n| n.try_cast::<AnimationNodeOneShot>().ok()),
        ) else {
            godot_error!("avatar: the emote nodes are missing from the animation tree");
            return;
        };
        let single = animation_node(locomotion.get_node("Emote"));
        let mix = locomotion
            .get_node("Emote_Mix")
            .and_then(|n| n.try_cast::<AnimationNodeBlendTree>().ok());
        let (Some(mut single), Some(mix)) = (single, mix) else {
            godot_error!("avatar: Emote / Emote_Mix are missing from the locomotion machine");
            return;
        };
        let (Some(mut mix_a), Some(mut mix_b)) = (
            animation_node(mix.get_node("A")),
            animation_node(mix.get_node("B")),
        ) else {
            godot_error!("avatar: Emote_Mix has no A / B");
            return;
        };
        let mut masked = masked;
        for node in [&mut single, &mut mix_a, &mut mix_b, &mut masked] {
            node.set_animation(IDLE_ANIM);
        }

        // The idle clip is edited at runtime (props visibility): make it unshared.
        if let Some(mut idle_library) = player
            .has_animation_library("idle")
            .then(|| player.get_animation_library("idle"))
            .flatten()
        {
            idle_library = idle_library.duplicate_resource();
            if let Some(idle) = idle_library.get_animation("Anim") {
                let idle = idle.duplicate_resource();
                idle_library.remove_animation("Anim");
                idle_library.add_animation("Anim", &idle);
            }
            player.remove_animation_library("idle");
            player.add_animation_library("idle", &idle_library);
        }

        // Asking for a missing library logs an engine error, so check first.
        let existing = player
            .has_animation_library("emotes")
            .then(|| player.get_animation_library("emotes"))
            .flatten();
        let library = existing.unwrap_or_else(|| {
            let library = AnimationLibrary::new_gd();
            player.add_animation_library("emotes", &library);
            library
        });

        self.emote.nodes = Some(EmoteNodes {
            single,
            mix_a,
            mix_b,
            masked,
            one_shot,
            library,
        });
    }

    pub(crate) fn reset_emote_nodes_to_idle(&mut self) {
        if let Some(nodes) = self.emote.nodes.as_mut() {
            for node in [
                &mut nodes.single,
                &mut nodes.mix_a,
                &mut nodes.mix_b,
                &mut nodes.masked,
            ] {
                node.set_animation(IDLE_ANIM);
            }
        }
    }

    fn tree_set(&self, param: &str, value: Variant) {
        if let Some(nodes) = self.nodes.as_ref() {
            nodes.animation_tree.clone().set(param, &value);
        }
    }

    fn locomotion_playback(&self) -> Option<Gd<AnimationNodeStateMachinePlayback>> {
        self.nodes
            .as_ref()?
            .animation_tree
            .get(LOCOMOTION_PLAYBACK)
            .try_to()
            .ok()
    }

    /// Stops whatever plays and reports it interrupted.
    pub fn stop_emote(&mut self) {
        self.emit_emote_finished(true);
        if self.emote.playing_masked {
            self.tree_set(MASKED_REQUEST_PARAM, ONE_SHOT_FADE_OUT.to_variant());
            self.emote.playing_masked = false;
            self.hide_all_props();
        }
        self.emote.masked_suspended = false;
        self.emote.playing_single = false;
        self.emote.playing_mixed = false;
        self.emote.playing_loop = false;
        self.emote.stop_generation += 1;
    }

    /// The emote still mid-playback (urn, mask), cleared so nothing reports it twice. Used by the
    /// teardown paths right before the node is freed: a deferred emit on a dying node is dropped,
    /// and the scene would wait forever for a terminal state.
    pub fn take_unfinished_emote(&mut self) -> Option<(String, i64)> {
        self.clear_pending_network_emote();
        if self.emote.current_urn.is_empty() {
            return None;
        }
        let info = (
            std::mem::take(&mut self.emote.current_urn),
            self.emote.current_mask,
        );
        self.emote.current_mask = -1;
        self.emote.current_scene_id = -1;
        self.emote.masked_suspended = false;
        Some(info)
    }

    /// Reports the end of the current emote, once per started emote. Deferred like
    /// `emote_triggered`, so a supersede reads stop-then-start.
    pub(crate) fn emit_emote_finished(&mut self, interrupted: bool) {
        if self.emote.current_urn.is_empty() {
            return;
        }
        let urn = std::mem::take(&mut self.emote.current_urn);
        let mask = self.emote.current_mask;
        self.emote.current_mask = -1;
        self.emote.current_scene_id = -1;
        self.emote.masked_suspended = false;
        self.run_deferred(move |this| {
            this.signals()
                .emote_finished()
                .emit(&urn, interrupted, mask)
        });
    }

    /// Plays an emote by id or urn that is already loaded.
    /// `mask`: -1 full body, 0 upper body. `owner_scene_id`: -1 when nobody owns it.
    pub fn play_emote(&mut self, id: &str, mask: i64, owner_scene_id: i32) {
        if id.is_empty() {
            return;
        }
        if self.emote.is_modifying {
            self.emote.queued_urn = id.to_string();
            return;
        }
        if let Some(nodes) = self.nodes.as_mut() {
            if !nodes.animation_tree.is_active() {
                nodes.animation_tree.set_active(true);
            }
        }

        let mut mask = mask;
        let triggered = if !id.starts_with("urn") {
            if emotes::is_emote_utility(id) {
                mask = -1;
                self.play_utility_emote(id)
            } else if emotes::is_emote_default(id) {
                self.play_loaded_emote(&emotes::base_emote_urn(id), mask, owner_scene_id)
            } else {
                godot_error!("Unknown emote: {id}");
                false
            }
        } else {
            self.play_loaded_emote(id, mask, owner_scene_id)
        };
        if !triggered {
            return;
        }

        // A still-playing emote is superseded: report its interruption first.
        self.emit_emote_finished(true);
        self.emote.current_urn = id.to_string();
        self.emote.current_mask = mask;
        self.emote.current_scene_id = owner_scene_id;
        self.emote.masked_suspended = false;
        if self.is_local_player {
            self.track_emote(id);
        }
        let looping = self.emote.playing_loop;
        let id = id.to_string();
        self.run_deferred(move |this| this.signals().emote_triggered().emit(&id, looping, mask));
    }

    fn play_utility_emote(&mut self, utility_id: &str) -> bool {
        let anim_name = format!("default_actions/{utility_id}");
        let Some(player) = self.nodes.as_ref().map(|n| n.animation_player.clone()) else {
            return false;
        };
        if !player.has_animation(anim_name.as_str()) {
            godot_error!(
                "Utility emote {utility_id} not found from player '{}'",
                self.avatar_name
            );
            return false;
        }
        self.abort_masked_emote();
        if let Some(nodes) = self.emote.nodes.as_mut() {
            nodes.single.set_animation(anim_name.as_str());
        }
        if let Some(mut playback) = self.locomotion_playback() {
            if playback.get_current_node() == "Emote" {
                playback.start_ex("Emote").reset(true).done();
            } else {
                playback.travel("Emote");
            }
        }
        self.emote.playing_single = true;
        self.emote.playing_mixed = false;
        self.emote.playing_loop = false;
        true
    }

    /// Plays the clip on the masked OneShot layer; locomotion keeps the excluded tracks and the
    /// emote keeps playing while the player walks, runs or jumps.
    fn play_masked_emote(&mut self, anim_path: &str) -> bool {
        if self.emote.playing_single || self.emote.playing_mixed {
            self.emote.playing_single = false;
            self.emote.playing_mixed = false;
            self.tree_set("parameters/Locomotion/conditions/emote", false.to_variant());
            self.tree_set("parameters/Locomotion/conditions/nemote", true.to_variant());
        }
        // The new clip's own visibility track shows its prop again.
        self.hide_all_props();

        let Some(animation) = self
            .nodes
            .as_ref()
            .and_then(|n| n.animation_player.get_animation(anim_path))
        else {
            return false;
        };
        let looping = self.emote.playing_loop;
        let Some(nodes) = self.emote.nodes.as_mut() else {
            return false;
        };

        // Filter the layer to exactly this clip's tracks minus lower body and root motion.
        for path in self.emote.masked_filter_paths.drain(..) {
            nodes.one_shot.set_filter_path(&path, false);
        }
        let mut excluded = 0;
        for track in 0..animation.get_track_count() {
            let path = animation.track_get_path(track);
            if is_excluded_from_mask(&path.to_string()) {
                excluded += 1;
                continue;
            }
            nodes.one_shot.set_filter_path(&path, true);
            self.emote.masked_filter_paths.push(path);
        }
        if excluded == 0 {
            godot_warn!("Masked emote '{anim_path}' matched no lower-body track; the mask may have no effect");
        }

        nodes.masked.set_animation(anim_path);
        // Looping inside the node: re-firing the OneShot per loop shows a blend seam. A custom
        // timeline is required for `loop_mode` and defaults to 1s, so take the clip length.
        nodes.masked.set_use_custom_timeline(looping);
        if looping {
            nodes
                .masked
                .set_timeline_length(animation.get_length() as f64);
            nodes.masked.set_loop_mode(LoopMode::LINEAR);
        } else {
            nodes.masked.set_loop_mode(LoopMode::NONE);
        }
        self.tree_set(MASKED_REQUEST_PARAM, ONE_SHOT_FIRE.to_variant());
        self.emote.masked_fired_frame = Engine::singleton().get_process_frames() as i64;
        self.emote.playing_masked = true;
        true
    }

    /// The local player crossed a scene boundary: a masked emote only drives the skeleton while
    /// the player is inside the scene that asked for it. Deliberately silent: no triggered or
    /// finished signal, so scenes see no EmoteState traffic when the player crosses a boundary.
    /// Only looping emotes come back; a non-looping one already delivered its playback.
    pub fn on_current_scene_changed_impl(&mut self, new_scene_id: i32) {
        if self.emote.current_urn.is_empty() || self.emote.current_scene_id == -1 {
            return;
        }
        let inside = self.emote.current_scene_id == new_scene_id;
        if self.emote.playing_masked && !inside {
            if !self.emote.playing_loop {
                self.stop_emote();
                return;
            }
            self.tree_set(MASKED_REQUEST_PARAM, ONE_SHOT_FADE_OUT.to_variant());
            self.emote.playing_masked = false;
            self.emote.masked_suspended = true;
            self.hide_all_props();
        } else if self.emote.masked_suspended && inside {
            self.resume_masked_emote();
        }
    }

    fn resume_masked_emote(&mut self) {
        self.emote.masked_suspended = false;
        let urn = self.emote.current_urn.clone();
        let Some(anim_name) = self
            .emote
            .loaded
            .get(&urn)
            .map(|e| e.default_anim_name.clone())
        else {
            self.stop_emote();
            return;
        };
        let anim_path = format!("emotes/{anim_name}");
        let has_animation = self
            .nodes
            .as_ref()
            .is_some_and(|n| n.animation_player.has_animation(anim_path.as_str()));
        if !has_animation {
            self.stop_emote();
            return;
        }
        if let Some(nodes) = self.nodes.as_mut() {
            if !nodes.animation_tree.is_active() {
                nodes.animation_tree.set_active(true);
            }
        }
        if !self.play_masked_emote(&anim_path) {
            self.stop_emote();
            return;
        }
        // The suspend dropped `playing_masked`, so comms took the stop edge and cleared its
        // pending urn: announce the emote again or remote viewers never see the arms come back.
        // Scenes still see nothing: their EmoteState rides the signals.
        if self.is_local_player {
            globals::comms()
                .bind_mut()
                .send_emote(urn.as_str().into(), self.emote.current_mask);
        }
    }

    /// Ends the current emote for a scene's `stopEmote`. A masked emote owned by another scene
    /// is left alone (a portable experience must not end the parcel scene's emote); returns
    /// whether it stopped something.
    pub fn stop_emote_from_scene(&mut self, scene_id: i32) -> bool {
        if (self.emote.playing_masked || self.emote.masked_suspended)
            && self.emote.current_scene_id != scene_id
        {
            return false;
        }
        self.stop_emote();
        self.clear_pending_network_emote();
        true
    }

    /// The owning scene is torn down: drop its emote for good, reporting the terminal state.
    pub fn clear_emote_owned_by_scene(&mut self, scene_id: i32) {
        if self.emote.current_urn.is_empty() || self.emote.current_scene_id != scene_id {
            return;
        }
        self.stop_emote();
    }

    /// Stops the masked layer without reporting; callers report the interruption when due.
    pub(crate) fn abort_masked_for_freeze(&mut self) {
        if self.emote.playing_masked {
            self.tree_set(MASKED_REQUEST_PARAM, ONE_SHOT_ABORT.to_variant());
            self.emote.playing_masked = false;
        }
    }

    pub(crate) fn reset_rest_pose_for_freeze(&mut self) {
        self.reset_skeleton_to_rest_pose();
    }

    fn abort_masked_emote(&mut self) {
        if !self.emote.playing_masked {
            return;
        }
        self.tree_set(MASKED_REQUEST_PARAM, ONE_SHOT_ABORT.to_variant());
        self.emote.playing_masked = false;
        self.hide_all_props();
    }

    fn play_loaded_emote(&mut self, urn: &str, mask: i64, owner_scene_id: i32) -> bool {
        let Some(item) = self.emote.loaded.get(urn) else {
            godot_error!("Emote {urn} not found from player '{}'", self.avatar_name);
            return false;
        };
        if item.default_anim_name.is_empty() {
            godot_error!("Emote {urn} has no animation");
            return false;
        }
        let anim_path = format!("emotes/{}", item.default_anim_name);
        let (from_scene, scene_loop, item_urn) = (item.from_scene, item.looping, item.urn.clone());

        self.emote.playing_loop = if from_scene {
            scene_loop
        } else {
            let wearable = globals::content_provider()
                .bind_mut()
                .get_wearable(item_urn.as_str().into());
            match wearable {
                Some(wearable) => wearable.bind().get_emote_loop(),
                None => return false,
            }
        };

        let known = self
            .nodes
            .as_ref()
            .is_some_and(|n| n.animation_player.has_animation(anim_path.as_str()));
        if !known {
            godot_error!("Animation not found in player: {anim_path}");
            return false;
        }
        if mask == MASK_UPPER_BODY {
            return self.play_masked_emote(&anim_path);
        }

        // A full-body emote supersedes a masked one still on the OneShot layer.
        self.abort_masked_emote();
        self.hide_all_props();
        self.reset_skeleton_to_rest_pose();

        self.emote.playing_single = true;
        self.emote.playing_mixed = false;
        if let Some(nodes) = self.emote.nodes.as_mut() {
            nodes.single.set_animation(anim_path.as_str());
        }
        if let Some(nodes) = self.nodes.as_mut() {
            if !nodes.animation_tree.is_active() {
                nodes.animation_tree.set_active(true);
            }
        }

        let Some(mut playback) = self.locomotion_playback() else {
            return true;
        };
        // A state machine that has not started needs a frame: start it and retry deferred.
        if playback.get_current_node().is_empty() {
            playback.start_ex("Idle").reset(true).done();
            let avatar = self.to_gd();
            let urn = urn.to_string();
            crate::engine::local_task::spawn_local(async move {
                crate::engine::local_task::next_frame().await;
                if avatar.is_instance_valid() {
                    avatar
                        .clone()
                        .bind_mut()
                        .deferred_play_emote(&urn, mask, owner_scene_id);
                }
            });
            return true;
        }

        // The transition requires these conditions: _process sets them too, but too late for an
        // immediate travel.
        self.tree_set("parameters/Locomotion/conditions/emote", true.to_variant());
        self.tree_set(
            "parameters/Locomotion/conditions/nemote",
            false.to_variant(),
        );
        if playback.get_current_node() == "Emote" {
            playback.start_ex("Emote").reset(true).done();
        } else {
            playback.travel("Emote");
        }
        self.emote.deferred_retry_count = 0;
        true
    }

    pub(crate) fn deferred_play_emote(&mut self, urn: &str, mask: i64, owner_scene_id: i32) {
        self.emote.deferred_retry_count += 1;
        if self.emote.deferred_retry_count > MAX_DEFERRED_RETRIES {
            self.emote.deferred_retry_count = 0;
            self.force_play_emote(urn);
            return;
        }
        self.play_emote(urn, mask, owner_scene_id);
    }

    /// Plays straight through the state machine when it will not initialize (an avatar hidden
    /// in the backpack).
    fn force_play_emote(&mut self, urn: &str) {
        let Some(item) = self.emote.loaded.get(urn) else {
            return;
        };
        if item.default_anim_name.is_empty() {
            return;
        }
        let anim_path = format!("emotes/{}", item.default_anim_name);
        let known = self
            .nodes
            .as_ref()
            .is_some_and(|n| n.animation_player.has_animation(anim_path.as_str()));
        if !known {
            return;
        }
        self.hide_all_props();
        self.reset_skeleton_to_rest_pose();
        if let Some(nodes) = self.emote.nodes.as_mut() {
            nodes.single.set_animation(anim_path.as_str());
        }
        self.tree_set("parameters/Locomotion/conditions/emote", true.to_variant());
        self.tree_set(
            "parameters/Locomotion/conditions/nemote",
            false.to_variant(),
        );
        if let Some(mut playback) = self.locomotion_playback() {
            playback.start_ex("Emote").reset(true).done();
        }
        self.emote.playing_single = true;
        self.emote.playing_mixed = false;
        self.emote.playing_loop = false;
    }

    pub(crate) fn hide_all_props(&mut self) {
        let names = self.emote.prop_armature_names.clone();
        for name in names {
            if let Some(mut prop) = self
                .base()
                .try_get_node_as::<godot::classes::Node3D>(name.as_str())
            {
                prop.hide();
            }
        }
        // Props added before tracking.
        for child in self.base().get_children().iter_shared() {
            if child.get_name().to_string().starts_with("Armature_Prop") {
                if let Ok(mut node) = child.try_cast::<Node3D>() {
                    node.hide();
                }
            }
        }
    }

    fn reset_skeleton_to_rest_pose(&mut self) {
        if let Some(nodes) = self.nodes.as_mut() {
            for bone in 0..nodes.skeleton.get_bone_count() {
                nodes.skeleton.reset_bone_pose(bone);
            }
        }
    }

    /// Whether any emote is playing.
    pub fn is_emote_playing(&self) -> bool {
        self.emote.is_playing()
    }

    /// Blocks emote cancellation by movement for a moment after a teleport or `movePlayerTo`.
    pub fn set_teleport_grace(&mut self) {
        self.emote.grace_until = now_seconds() + TELEPORT_GRACE_SECONDS;
    }

    /// Per-frame emote state: movement cancels a full-body emote, and the natural end of a
    /// clip is detected. `idle` is true when the avatar is not moving.
    pub(crate) fn emote_process(&mut self, idle: bool) {
        if self.emote.playing_masked {
            // `active` may not reflect a FIRE request until the tree processed a frame. A
            // looping masked emote loops inside the node: only stop/supersede ends it.
            let frame = Engine::singleton().get_process_frames() as i64;
            if !self.emote.playing_loop && frame > self.emote.masked_fired_frame {
                let active = self
                    .nodes
                    .as_ref()
                    .map(|n| n.animation_tree.get(MASKED_ACTIVE_PARAM).booleanize())
                    .unwrap_or(false);
                if !active {
                    self.emit_emote_finished(false);
                    self.emote.playing_masked = false;
                    self.hide_all_props();
                }
            }
        }

        if !(self.emote.playing_single || self.emote.playing_mixed) {
            return;
        }
        let input = Input::singleton();
        let direction = input.get_vector("ia_left", "ia_right", "ia_forward", "ia_backward");
        let player_input = direction.length() > 0.01 || input.is_action_pressed("ia_jump");
        let in_grace = now_seconds() < self.emote.grace_until && !player_input;

        if !idle && !in_grace {
            self.emit_emote_finished(true);
            self.emote.playing_single = false;
            self.emote.playing_mixed = false;
            self.hide_all_props();
        } else if idle {
            let Some(mut playback) = self.locomotion_playback() else {
                return;
            };
            let current = playback.get_current_node();
            if current == "Emote" || current == "Emote_Mix" {
                let position = playback.get_current_play_position();
                let still_playing = position < playback.get_current_length();
                if position > 0.0 && !still_playing {
                    if self.emote.playing_loop {
                        playback.start_ex(&current).reset(true).done();
                    } else {
                        self.emit_emote_finished(false);
                        self.emote.playing_single = false;
                        self.emote.playing_mixed = false;
                        self.hide_all_props();
                    }
                }
            }
        }
    }

    fn track_emote(&self, id: &str) {
        let is_base = emotes::is_emote_default(id) || emotes::is_base_emote_urn(id);
        let source = if id.contains("scene-emote") {
            "scene"
        } else {
            "user"
        };
        let screen = if source == "scene" {
            "SCENE"
        } else {
            "EMOTE_WHEEL"
        };
        let payload = serde_json::json!({"emote_urn": id, "is_base": is_base, "source": source});
        globals::metrics().bind_mut().track_click_button(
            "USED EMOTE".to_string(),
            screen.to_string(),
            payload.to_string(),
        );
    }

    /// Pauses the tree on the idle pose for stills (previews, renderers).
    pub fn freeze_on_idle(&mut self) {
        self.emit_emote_finished(true);
        // A OneShot left fired would resume the emote with stale filters on reactivation.
        if self.emote.playing_masked {
            self.tree_set(MASKED_REQUEST_PARAM, ONE_SHOT_ABORT.to_variant());
            self.emote.playing_masked = false;
        }
        let Some(nodes) = self.nodes.as_mut() else {
            return;
        };
        nodes
            .animation_tree
            .set_process_mode(godot::classes::node::ProcessMode::DISABLED);
        nodes.animation_player.stop();
        nodes
            .animation_player
            .play_ex()
            .name("Idle")
            .custom_blend(-1.0)
            .custom_speed(0.0)
            .done();
        for child in self.base().get_children().iter_shared() {
            if child.get_name().to_string().starts_with("Armature_Prop") {
                if let Ok(mut node) = child.try_cast::<Node3D>() {
                    node.hide();
                }
            }
        }
    }

    pub(crate) fn clear_pending_network_emote(&mut self) {
        self.pending_emote = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_body_and_root_motion_are_excluded_from_the_mask() {
        assert!(is_excluded_from_mask("Armature/Skeleton3D:Avatar_Hips"));
        assert!(is_excluded_from_mask("Other/Rig:Avatar_LeftFoot"));
        assert!(is_excluded_from_mask("Armature"));
        assert!(!is_excluded_from_mask("Armature/Skeleton3D:Avatar_Head"));
        assert!(!is_excluded_from_mask("Armature_Prop"));
    }
}
