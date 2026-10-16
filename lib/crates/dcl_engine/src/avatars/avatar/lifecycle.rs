//! The avatar node's life: wiring the scene's nodes in `ready`, the per-frame update, input,
//! modifier areas, voice and trigger detection.

use godot::classes::base_material_3d::BillboardMode;
use godot::classes::input::MouseMode;
use godot::classes::{
    Area3D, AudioStreamGenerator, AudioStreamPlayer, AudioStreamPlayer3D, CollisionShape3D, Input,
    InputEvent, Node3D, Skeleton3D, Sprite3D, StaticBody3D, SubViewport, Timer,
};
use godot::prelude::*;

use super::{AvatarNodes, EmoteAudio};
use crate::avatars::exclude_ids::is_excluded;
use crate::avatars::impostor::{LodState, DISTANCE_CHECK_PERIOD_FRAMES};
use crate::avatars::impostor_capture;
use crate::avatars::mesh_assembler::{
    apply_toon_material, apply_toon_material_recursive, MeshAssembler,
};
use crate::avatars::modifier_detector::AvatarModifierAreaDetector;
use crate::avatars::nameplate::NicknameUi;
use crate::fetcher::globals;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::dcl_avatar_modifier_area_3d::DclAvatarModifierArea3D;
use crate::godot_classes::dcl_global::DclGlobal;

// Collision layers, mirroring `decentraland.sdk.components.ColliderLayer`. CL_PLAYER is on every
// avatar; CL_MAIN_PLAYER is added for the local player so scenes can tell it from the rest.
const CL_PLAYER: u32 = 4;
const CL_MAIN_PLAYER: u32 = 8;

// AvatarModifierType.
const MODIFIER_HIDE_AVATAR: i32 = 0;
const MODIFIER_DISABLE_PASSPORT: i32 = 1;
const MODIFIER_HIDE_NAME_TAG: i32 = 2;

fn child<T: GodotClass + Inherits<Node>>(root: &Gd<DclAvatar>, path: &str) -> Option<Gd<T>> {
    root.get_node_or_null(path)?.try_cast().ok()
}

impl DclAvatar {
    /// The nodes of `avatar.tscn`. None when the scene lacks them (a bare avatar in a test).
    fn collect_nodes(&self) -> Option<AvatarNodes> {
        let me = self.to_gd();
        let skeleton: Gd<Skeleton3D> = child(&me, "Armature/Skeleton3D")?;
        let audio_emote = match me.get_node_or_null("AudioPlayer_Emote") {
            Some(n) => EmoteAudio::Spatial(n.try_cast::<AudioStreamPlayer3D>().ok()?),
            None => return None,
        };
        let nickname_quad: Gd<Sprite3D> = child(
            &me,
            "Armature/Skeleton3D/BoneAttachment3D_Name/NicknameQuad",
        )?;
        Some(AvatarNodes {
            animation_tree: child(&me, "AnimationTree")?,
            animation_player: child(&me, "AnimationPlayer")?,
            skeleton,
            audio_emote,
            audio_double_jump: child(&me, "AudioPlayer_DoubleJump")?,
            nickname_ui: child::<NicknameUi>(
                &me,
                "Armature/Skeleton3D/BoneAttachment3D_Name/NicknameQuad/NicknameViewport/NicknameUI",
            )?,
            nickname_viewport: child::<SubViewport>(
                &me,
                "Armature/Skeleton3D/BoneAttachment3D_Name/NicknameQuad/NicknameViewport",
            ),
            nickname_quad,
            timer_hide_mic: child::<Timer>(&me, "Timer_HideMic")?,
            click_area: child::<Area3D>(&me, "ClickArea")?,
            trigger_detector: child::<StaticBody3D>(&me, "TriggerDetector"),
            glider_prop: child::<Node3D>(&me, "GliderProp")?,
            modifier_detector: child::<AvatarModifierAreaDetector>(&me, "avatar_modifier_area_detector")?,
        })
    }

    pub(crate) fn life_ready(&mut self) {
        let Some(nodes) = self.collect_nodes() else {
            return;
        };
        let skeleton = nodes.skeleton.clone();
        self.nodes = Some(nodes);
        self.mesh_assembler = Some(MeshAssembler::new(skeleton.clone()));

        if let Some(nodes) = self.nodes.as_mut() {
            apply_toon_material_recursive(&nodes.glider_prop.clone().upcast());
            // One evaluation so an avatar that spawns off screen or FAR (tree frozen at once)
            // still gets a pose instead of the bind pose.
            nodes.animation_tree.set_active(true);
            nodes.animation_tree.advance(0.0);
            nodes
                .nickname_quad
                .set_billboard_mode(BillboardMode::ENABLED);
        }

        // Scene NPCs live outside the comms list and have no physics colliders: nameplate
        // occlusion finds them through this group.
        if self.is_avatar_shape {
            self.base_mut().add_to_group("avatar_shapes");
        }

        // Personal-space dissolve for any avatar near the world camera (#1814).
        let mut fade = crate::avatars::proximity_fade::AvatarProximityFade::new_alloc();
        fade.set_name("AvatarProximityFade");
        self.base_mut().add_child(&fade);

        self.setup_emotes();
        let on_skeleton = self.base().callable("on_skeleton_updated");
        let mut skeleton_node = skeleton;
        skeleton_node.connect("skeleton_updated", &on_skeleton);
        self.recompute_nametag_clearance();
        self.recompute_nametag_posed_top();

        // The detector (a child, ready first) may have scanned overlaps before this: evaluate
        // again so spawning inside an area is not lost.
        self.try_show();

        if self.non_3d_audio {
            self.replace_emote_audio_with_plain();
        }

        if let Some(nodes) = self.nodes.as_mut() {
            nodes.nickname_ui.bind_mut().set_mic_enabled(false);
        }
        let mut global = DclGlobal::singleton();
        let on_chat = self.base().callable("on_global_chat_message");
        global.connect("on_chat_message", &on_chat);
        self.nameplate_attach();
        self.apply_nickname_visibility();

        self.lod.phase = self.unique_id as u64 % DISTANCE_CHECK_PERIOD_FRAMES;
        impostor_capture::register(self.to_gd());
        self.setup_screen_notifier();

        // Metadata for raycast detection, same as DCL entities.
        let id = self.avatar_id.clone();
        if let Some(nodes) = self.nodes.as_mut() {
            nodes.click_area.set_meta("is_avatar", &true.to_variant());
            nodes.click_area.set_meta("avatar_id", &id.to_variant());
        }
    }

    /// The 3D emote player carries the volume in the scene: the plain replacement repeats it
    /// (Unity's prefab is at 0.5) or emotes play at full volume.
    fn replace_emote_audio_with_plain(&mut self) {
        let Some(nodes) = self.nodes.as_mut() else {
            return;
        };
        let EmoteAudio::Spatial(old) = &nodes.audio_emote else {
            return;
        };
        let name = old.get_name();
        let mut old = old.clone();
        self.base_mut().remove_child(&old);
        old.queue_free();
        let mut plain = AudioStreamPlayer::new_alloc();
        plain.set_bus("AvatarAndEmotes");
        plain.set_volume_db(0.5_f32.log10() * 20.0);
        self.base_mut().add_child(&plain);
        plain.set_name(&name.to_string());
        if let Some(nodes) = self.nodes.as_mut() {
            nodes.audio_emote = EmoteAudio::Plain(plain);
        }
    }

    pub(crate) fn life_exit_tree(&mut self) {
        impostor_capture::unregister(&self.to_gd());
        // The plate lives in the shared layer, so it is not freed with the subtree, and must not
        // be freed here either: a transient reparent also exits the tree and `ready` runs once.
        // Hide it; the real free is `NOTIFICATION_PREDELETE`.
        self.nameplate_park();
    }

    pub(crate) fn life_predelete(&mut self) {
        self.nameplate_detach();
    }

    /// Per frame, after the movement lerp: nameplate, LOD, animation drive and the AnimationTree
    /// conditions.
    pub(crate) fn life_process(&mut self, delta: f64) {
        self.advance_movement(delta);
        if self.nodes.is_none() {
            return;
        }
        self.nameplate_update();
        self.maybe_update_lod();
        self.tick_animation_throttle(delta);
        if self.lod.state == LodState::Far {
            return;
        }
        // No body to animate for a wearable-only NPC.
        if self.is_avatar_shape && self.avatar_data.bind().get_show_only_wearables() {
            if let Some(nodes) = self.nodes.as_mut() {
                nodes.animation_tree.set_active(false);
            }
            return;
        }
        // Never undo an off-screen freeze: that reactivation left frozen avatars stuck.
        self.ensure_anim_active();

        // `is_grounded` suppresses the all-false window at the jump apex (the rise/fall deadband)
        // so idle does not leak in mid-air.
        let idle =
            self.is_grounded && !self.jog && !self.walk && !self.run && !self.rise && !self.fall;
        self.emote_process(idle);

        // Masked (upper-body) emotes keep playing while moving, so idle cannot gate them:
        // otherwise Pulse would broadcast EmoteStop for a walking emote.
        let emoting = self.emote.is_playing() && (idle || self.emote.playing_masked);
        if self.is_local_player {
            globals::comms().bind_mut().set_emoting(emoting);
        }
        self.drive_animation_tree(idle, delta);
    }

    fn drive_animation_tree(&mut self, idle: bool, delta: f64) {
        let (single, mixed) = (self.emote.playing_single, self.emote.playing_mixed);
        let (walk, jog, run) = crate::avatars::avatar::lod::locomotion_conditions(
            self.walk,
            self.jog,
            self.run,
            self.is_grounded,
        );

        // Rising edge of the double jump; the first observation is suppressed so state from
        // before we started watching does not play its sound.
        let mut jump_edge = self.jump_count > self.glider.last_jump_count && self.jump_count >= 2;
        if self.glider.jump_sync_pending {
            jump_edge = false;
            self.glider.jump_sync_pending = false;
        }
        self.glider.last_jump_count = self.jump_count;
        let gliding = self.glide_state == 1 || self.glide_state == 2;

        let moving = self.walk || self.jog || self.run;
        let target = if moving { 1.0 } else { 0.0 };
        let step = delta as f32 * 4.0;
        let blend = self.glider.forward_blend;
        self.glider.forward_blend = if (target - blend).abs() <= step {
            target
        } else {
            blend + (target - blend).signum() * step
        };
        let forward_blend = self.glider.forward_blend;

        let (rise, fall, land, grounded) = (self.rise, self.fall, self.land, self.is_grounded);
        let Some(nodes) = self.nodes.as_mut() else {
            return;
        };
        let tree = &mut nodes.animation_tree;
        let mut set = |param: &str, value: Variant| tree.set(param, &value);
        set("parameters/Locomotion/conditions/idle", idle.to_variant());
        set(
            "parameters/Locomotion/conditions/emote",
            single.to_variant(),
        );
        set(
            "parameters/Locomotion/conditions/nemote",
            (!single).to_variant(),
        );
        set("parameters/Locomotion/conditions/emix", mixed.to_variant());
        set(
            "parameters/Locomotion/conditions/nemix",
            (!mixed).to_variant(),
        );
        set("parameters/Locomotion/conditions/run", run.to_variant());
        set("parameters/Locomotion/conditions/jog", jog.to_variant());
        set("parameters/Locomotion/conditions/walk", walk.to_variant());
        set("parameters/Locomotion/conditions/rise", rise.to_variant());
        set("parameters/Locomotion/conditions/fall", fall.to_variant());
        set("parameters/Locomotion/conditions/land", land.to_variant());
        // `nfall` reads is_grounded, not `land`: `land` is a short pulse locally and was
        // overridden to is_grounded for remotes. is_grounded has the same shape on both sides.
        set(
            "parameters/Locomotion/conditions/nfall",
            grounded.to_variant(),
        );
        set(
            "parameters/Locomotion/conditions/double_jump",
            jump_edge.to_variant(),
        );
        set(
            "parameters/Locomotion/conditions/gliding",
            gliding.to_variant(),
        );
        set(
            "parameters/Locomotion/conditions/ngliding",
            (!gliding).to_variant(),
        );
        set(
            "parameters/Locomotion/Gliding_Idle/Blend2/blend_amount",
            forward_blend.to_variant(),
        );
        if jump_edge {
            self.play_double_jump();
        }
        self.update_glider_prop();
    }

    pub(crate) fn life_physics_process(&mut self) {
        // The occlusion raycast must run here: the direct space state crashes from an idle frame.
        self.nameplate_update_occlusion();
    }

    pub(crate) fn life_input(&mut self, event: Gd<InputEvent>) {
        if !event.is_action_pressed("ia_pointer") {
            return;
        }
        // Only the selected avatar, when it is not blocked or hidden.
        let selected = DclGlobal::singleton().bind_mut().get_selected_avatar();
        let me = self.base().instance_id();
        if selected.map(|s| s.instance_id()) != Some(me)
            || self.avatar_id.is_empty()
            || self.hidden
            || self.passport_disabled
        {
            return;
        }
        if Input::singleton().get_mouse_mode() != MouseMode::CAPTURED {
            return;
        }
        if let Some(mut explorer) = globals::explorer() {
            if explorer.is_instance_valid()
                && explorer.call("is_session_hide_main_hud", &[]).booleanize()
                && explorer
                    .call("is_session_hide_view_profile", &[])
                    .booleanize()
            {
                return;
            }
        }
        let avatar = self.to_gd().to_variant();
        DclGlobal::singleton().call_deferred(
            "emit_signal",
            &["open_profile_by_avatar".to_variant(), avatar],
        );
    }

    /// Re-evaluates the modifier areas the avatar stands in.
    pub fn try_show(&mut self) {
        let Some(detector) = self.nodes.as_ref().map(|n| n.modifier_detector.clone()) else {
            return;
        };
        let areas = detector.bind().areas_in_scene(self.current_parcel_scene_id);
        self.apply_modifier_areas(&areas);
    }

    /// Resets the modifier state, then applies the union of every overlapping area's modifiers
    /// (#2665: only the latest area counted, so entering a second one wiped an active
    /// HideNameTags from an area the avatar never left).
    pub(crate) fn apply_modifier_areas(&mut self, areas: &[Gd<DclAvatarModifierArea3D>]) {
        if !self.hidden {
            self.base_mut().show();
            self.set_click_area_enabled(true);
        }
        self.passport_disabled = false;
        self.nametag_hidden = false;
        let id = self.avatar_id.to_string();
        for area in areas {
            let area = area.bind();
            let excluded: Vec<String> = area
                .get_exclude_ids()
                .iter_shared()
                .map(|s| s.to_string())
                .collect();
            if is_excluded(&id, &excluded) {
                continue;
            }
            for modifier in area.get_avatar_modifiers().iter_shared() {
                match modifier {
                    MODIFIER_HIDE_AVATAR => {
                        self.base_mut().hide();
                        self.hide_impostor_render();
                        self.set_click_area_enabled(false);
                    }
                    MODIFIER_DISABLE_PASSPORT => self.passport_disabled = true,
                    MODIFIER_HIDE_NAME_TAG => self.nametag_hidden = true,
                    _ => {}
                }
            }
        }
        self.apply_nickname_visibility();
    }

    pub fn apply_hidden(&mut self, value: bool) {
        self.hidden = value;
        if value {
            self.base_mut().hide();
            self.hide_impostor_render();
            // Blocked or hidden avatars cannot be interacted with.
            self.set_click_area_enabled(false);
        } else {
            self.try_show();
            self.set_click_area_enabled(true);
        }
        // The plate is reparented into the shared layer, so hide() above does not reach it.
        self.apply_nickname_visibility();
    }

    fn set_click_area_enabled(&mut self, enabled: bool) {
        let Some(nodes) = self
            .nodes
            .as_ref()
            .filter(|n| n.click_area.is_instance_valid())
        else {
            return;
        };
        if let Some(shape) = nodes.click_area.get_node_or_null("CollisionShape3D") {
            if let Ok(mut shape) = shape.try_cast::<CollisionShape3D>() {
                shape.set_disabled(!enabled);
            }
        }
    }

    /// Detection for the local player and remote avatars; scene NPCs have none. `entity_id` is
    /// SceneEntityId.PLAYER for the local player, the assigned entity for remote avatars.
    pub fn setup_trigger_detection(&mut self, entity_id: i32) {
        self.dcl_entity_id = entity_id;
        let is_local = self.is_local_player;
        let Some(trigger) = self
            .nodes
            .as_mut()
            .and_then(|n| n.trigger_detector.as_mut())
        else {
            return;
        };
        // So trigger_area.rs can identify this avatar.
        trigger.set_meta("dcl_entity_id", &entity_id.to_variant());
        if is_local {
            trigger.set_collision_layer(CL_PLAYER | CL_MAIN_PLAYER);
        }
        if let Some(shape) = trigger.get_node_or_null("CollisionShape3D") {
            if let Ok(mut shape) = shape.try_cast::<CollisionShape3D>() {
                shape.set_disabled(false);
            }
        }
    }

    pub fn remove_trigger_detection(&mut self) {
        if let Some(nodes) = self.nodes.as_mut() {
            if let Some(mut trigger) = nodes.trigger_detector.take() {
                trigger.queue_free();
            }
        }
    }

    pub fn spawn_voice_channel(&mut self, sample_rate: f32) {
        let mut player = AudioStreamPlayer::new_alloc();
        player.set_bus("VoiceChat");
        let mut generator = AudioStreamGenerator::new_gd();
        generator.set_mix_rate(sample_rate);
        player.set_stream(&generator);
        self.base_mut().add_child(&player);
        player.play();
        self.voice_player = Some(player);
    }

    pub fn push_voice_frame(&mut self, frame: PackedVector2Array) {
        let Some(player) = self.voice_player.as_mut() else {
            return;
        };
        if !player.is_playing() {
            player.play();
        }
        if let Some(playback) = player.get_stream_playback() {
            if let Ok(mut playback) =
                playback.try_cast::<godot::classes::AudioStreamGeneratorPlayback>()
            {
                playback.push_buffer(&frame);
            }
        }
        let Some(nodes) = self.nodes.as_mut() else {
            return;
        };
        // The mic icon hides again when the frames stop.
        let was_on = nodes.nickname_ui.bind().get_mic_enabled();
        if !was_on {
            nodes.nickname_ui.bind_mut().set_mic_enabled(true);
            nodes.timer_hide_mic.start();
        }
    }

    pub(crate) fn hide_mic(&mut self) {
        if let Some(nodes) = self.nodes.as_mut() {
            nodes.nickname_ui.bind_mut().set_mic_enabled(false);
        }
    }

    pub fn apply_toon_material_to(&self, node: Gd<Node>) {
        apply_toon_material(&node);
    }
}
