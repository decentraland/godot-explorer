//! Level of detail: the distance and pool bands, the impostor slot, and how each tier drives
//! the animation tree, the meshes and the particles.

use std::sync::atomic::{AtomicBool, Ordering};

use godot::classes::animation_mixer::AnimationCallbackModeProcess;
use godot::classes::node::ProcessMode;
use godot::classes::{
    Area3D, CollisionShape3D, Engine, GpuParticles3D, MeshInstance3D, Time,
    VisibleOnScreenNotifier3D,
};
use godot::prelude::*;

use crate::avatars::impostor::{
    LodState, DISTANCE_CHECK_PERIOD_FRAMES, DISTANCE_FAR, MID_ANIM_ADVANCE_EVERY_N_FRAMES,
    MID_RANGE_NEAR, TINT_FULL_DISTANCE,
};
use crate::fetcher::globals;
use crate::godot_classes::dcl_avatar::DclAvatar;

/// Covers the avatar including arms-out emote poses: erring large only animates an avatar that
/// may be off screen, while erring small freezes a drawn one.
const SCREEN_NOTIFIER_AABB: Aabb = Aabb {
    position: Vector3::new(-1.0, -0.3, -1.0),
    size: Vector3::new(2.0, 2.8, 2.0),
};

/// Global toggle for the move/jump/land dust particles, set from the graphics profile.
static PARTICLES_ENABLED: AtomicBool = AtomicBool::new(true);

pub struct LodRuntime {
    pub state: LodState,
    pub impostor_layer: i32,
    pub phase: u64,
    pub mesh_visibility_captured: bool,
    /// Written by the coordinator: demotes the distance LOD by pool rank.
    pub rank_cap: LodState,
    /// Written by the coordinator: beyond the real impostor layers, such avatars borrow
    /// another slot's texture and render fully tinted.
    pub use_overflow: bool,
    pub layer_is_overflow: bool,
    /// Written by the coordinator: fully outside the frustum, so the slot is released.
    pub off_frustum: bool,
    /// Godot's exact draw state (the notifier), not the coordinator's lagging sphere test.
    pub on_screen: bool,
    pub screen_notifier: Option<Gd<VisibleOnScreenNotifier3D>>,
    /// The tree was paused because the avatar left the screen.
    pub anim_frozen_off_screen: bool,
    pub anim_freeze_start_ms: u64,
    /// MID tier: the tree advances every Nth frame so bones update at ~20 fps.
    pub throttle_acc: f64,
    pub throttle_counter: u32,
    pub throttle_active: bool,
}

impl Default for LodRuntime {
    fn default() -> Self {
        Self {
            state: LodState::Full,
            impostor_layer: -1,
            phase: 0,
            mesh_visibility_captured: false,
            rank_cap: LodState::Full,
            use_overflow: false,
            layer_is_overflow: false,
            off_frustum: false,
            on_screen: true,
            screen_notifier: None,
            anim_frozen_off_screen: false,
            anim_freeze_start_ms: 0,
            throttle_acc: 0.0,
            throttle_counter: 0,
            throttle_active: false,
        }
    }
}

/// How the animation tree is driven: `(active, throttled)`.
///   off screen        -> frozen, the cull hides it and the CPU is saved
///   on screen FULL    -> active every frame
///   on screen MID     -> active, throttled
///   on screen FAR     -> inactive, the impostor is drawn
pub fn resolve_anim_drive(on_screen: bool, state: LodState) -> (bool, bool) {
    if !on_screen {
        return (false, false);
    }
    match state {
        LodState::Full => (true, false),
        LodState::Mid => (true, true),
        LodState::Far => (false, false),
    }
}

/// Locomotion conditions: only grounded avatars walk, jog or run.
pub fn locomotion_conditions(
    walk: bool,
    jog: bool,
    run: bool,
    grounded: bool,
) -> (bool, bool, bool) {
    (walk && grounded, jog && grounded, run && grounded)
}

/// The tier a camera distance maps to: `(state, dither_alpha, fade_alpha, tint)`.
/// Mesh always, billboard only past the safety valve: crowding is the pool's job.
pub fn bands_for_distance(dist: f32) -> (LodState, f32, f32, f32) {
    if dist < MID_RANGE_NEAR {
        (LodState::Full, 1.0, 0.0, 0.0)
    } else if dist < DISTANCE_FAR {
        (LodState::Mid, 1.0, 0.0, 0.0)
    } else {
        let span = TINT_FULL_DISTANCE - DISTANCE_FAR;
        let tint = ((dist - DISTANCE_FAR) / span).clamp(0.0, 1.0);
        (LodState::Far, 0.0, 1.0, tint)
    }
}

/// Applies the pool cap on top of the distance band.
pub fn with_rank_cap(band: (LodState, f32, f32, f32), cap: LodState) -> (LodState, f32, f32, f32) {
    if cap <= band.0 {
        return band;
    }
    if cap == LodState::Far {
        (cap, 0.0, 1.0, band.3)
    } else {
        (cap, 1.0, 0.0, band.3)
    }
}

pub fn particles_enabled() -> bool {
    PARTICLES_ENABLED.load(Ordering::Relaxed)
}

/// Sets the particles flag and applies it to every avatar in the tree.
pub fn apply_particles_enabled(enabled: bool) {
    PARTICLES_ENABLED.store(enabled, Ordering::Relaxed);
    let Some(tree) = Engine::singleton()
        .get_main_loop()
        .and_then(|l| l.try_cast::<godot::classes::SceneTree>().ok())
    else {
        return;
    };
    let Some(root) = tree.get_root() else {
        return;
    };
    for node in root
        .find_children_ex("*")
        .type_("DclAvatar")
        .recursive(true)
        .owned(false)
        .done()
        .iter_shared()
    {
        if let Ok(mut avatar) = node.try_cast::<DclAvatar>() {
            let show = enabled && avatar.bind().lod.state == LodState::Full;
            avatar.bind_mut().set_particles_visible(show);
        }
    }
}

impl DclAvatar {
    pub(crate) fn mesh_children(&self) -> Vec<Gd<MeshInstance3D>> {
        self.nodes
            .as_ref()
            .map(|n| {
                n.skeleton
                    .get_children()
                    .iter_shared()
                    .filter_map(|c| c.try_cast::<MeshInstance3D>().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn capture_mesh_visibility(&mut self) {
        if self.lod.mesh_visibility_captured || self.nodes.is_none() {
            return;
        }
        for mut mesh in self.mesh_children() {
            let visible = mesh.is_visible();
            mesh.set_meta("lod_visible", &visible.to_variant());
        }
        self.lod.mesh_visibility_captured = true;
    }

    pub(crate) fn set_meshes_visible(&mut self, visible: bool) {
        self.capture_mesh_visibility();
        for mut mesh in self.mesh_children() {
            let original = if mesh.has_meta("lod_visible") {
                mesh.get_meta("lod_visible").booleanize()
            } else {
                true
            };
            mesh.set_visible(visible && original);
        }
    }

    fn set_animation_active(&mut self, active: bool) {
        if let Some(nodes) = self.nodes.as_mut() {
            nodes.animation_tree.set_active(active);
        }
    }

    fn set_animation_speed(&mut self, speed: f32) {
        if let Some(nodes) = self.nodes.as_mut() {
            nodes.animation_player.set_speed_scale(speed);
        }
    }

    /// Routes the throttle through here so the callback mode and the flag never drift apart.
    fn set_animation_throttle(&mut self, throttled: bool) {
        self.lod.throttle_active = throttled;
        self.lod.throttle_acc = 0.0;
        self.lod.throttle_counter = 0;
        if let Some(nodes) = self.nodes.as_mut() {
            nodes
                .animation_tree
                .set_callback_mode_process(if throttled {
                    AnimationCallbackModeProcess::MANUAL
                } else {
                    AnimationCallbackModeProcess::IDLE
                });
        }
    }

    /// Freezes the tree while the avatar is not drawn and restores it on return, advancing by
    /// the paused wall-clock time once so the emote phase matches. The local player never freezes.
    pub(crate) fn apply_screen_freeze(&mut self) {
        if self.nodes.is_none() || self.is_local_player {
            return;
        }
        if !self.lod.on_screen {
            if !self.lod.anim_frozen_off_screen {
                self.set_animation_active(false);
                self.set_animation_throttle(false);
                self.lod.anim_freeze_start_ms = Time::singleton().get_ticks_msec();
                self.lod.anim_frozen_off_screen = true;
            }
        } else if self.lod.anim_frozen_off_screen {
            self.lod.anim_frozen_off_screen = false;
            let elapsed = (Time::singleton().get_ticks_msec() - self.lod.anim_freeze_start_ms)
                as f64
                / 1000.0;
            let (active, throttle) = resolve_anim_drive(self.lod.on_screen, self.lod.state);
            self.set_animation_active(active);
            self.set_animation_throttle(throttle);
            if active && elapsed > 0.0 {
                if let Some(nodes) = self.nodes.as_mut() {
                    nodes.animation_tree.advance(elapsed);
                }
            }
        }
    }

    /// Keeps a drawn avatar's tree active, never undoing an off-screen freeze.
    pub(crate) fn ensure_anim_active(&mut self) {
        if self.lod.anim_frozen_off_screen {
            return;
        }
        if let Some(nodes) = self.nodes.as_mut() {
            if !nodes.animation_tree.is_active() {
                nodes.animation_tree.set_active(true);
            }
        }
    }

    pub(crate) fn set_particles_visible(&mut self, visible: bool) {
        let visible = visible && PARTICLES_ENABLED.load(Ordering::Relaxed);
        for name in [
            "GPUParticles3D_Move",
            "GPUParticles3D_Jump",
            "GPUParticles3D_Land",
        ] {
            let Some(node) = self.base().get_node_or_null(name) else {
                continue;
            };
            let Ok(mut node) = node.try_cast::<Node3D>() else {
                continue;
            };
            node.set_visible(visible);
            if let Ok(mut particles) = node.clone().try_cast::<GpuParticles3D>() {
                particles.set_emitting(visible);
            }
            node.set_process_mode(if visible {
                ProcessMode::INHERIT
            } else {
                ProcessMode::DISABLED
            });
        }
    }

    pub(crate) fn set_click_active(&mut self, active: bool) {
        let Some(nodes) = self
            .nodes
            .as_mut()
            .filter(|n| n.click_area.is_instance_valid())
        else {
            return;
        };
        if let Some(shape) = nodes.click_area.get_node_or_null("CollisionShape3D") {
            if let Ok(mut shape) = shape.try_cast::<CollisionShape3D>() {
                shape.set_disabled(!active);
            }
        }
        let area: &mut Gd<Area3D> = &mut nodes.click_area;
        area.set_monitoring(active);
        area.set_monitorable(active);
    }

    pub(crate) fn set_dither_alpha(&mut self, value: f32) {
        let transparency = (1.0 - value).clamp(0.0, 1.0);
        for mut mesh in self.mesh_children() {
            mesh.set_transparency(transparency);
        }
    }

    /// Hides the impostor render until the next LOD pass: the MultiMesh lives on the avatar
    /// scene, so hiding the node does not reach it.
    pub(crate) fn hide_impostor_render(&mut self) {
        if self.lod.impostor_layer >= 0 {
            let id = self.base().instance_id().to_i64();
            globals::avatars_scene()
                .bind_mut()
                .set_impostor_state(id, 0.0, 0.0, 0.0);
        }
    }

    /// Key of the disk-backed impostor texture cache: the address for cross-session stability,
    /// plus body, wearables and colors so a new outfit captures fresh instead of pulling stale
    /// pixels. NPCs share an entry when they look the same.
    pub fn get_impostor_cache_key(&self) -> String {
        use sha2::{Digest, Sha256};
        let data = self.avatar_data.bind();
        let mut parts: Vec<String> = vec![
            self.avatar_id.to_string().to_lowercase(),
            data.get_body_shape().to_string(),
        ];
        let mut wearables: Vec<String> = data
            .get_wearables()
            .as_slice()
            .iter()
            .map(|w| w.to_string())
            .collect();
        wearables.sort();
        parts.extend(wearables);
        parts.push(format!("{:?}", data.get_skin_color()));
        parts.push(format!("{:?}", data.get_eyes_color()));
        parts.push(format!("{:?}", data.get_hair_color()));
        let digest = Sha256::digest(parts.join("|").as_bytes());
        digest.iter().map(|b| format!("{b:02x}")).collect()
    }

    pub(crate) fn maybe_update_lod(&mut self) {
        if !self.avatar_ready || self.is_local_player || self.hidden {
            return;
        }
        if Engine::singleton().get_frames_drawn() as u64 % DISTANCE_CHECK_PERIOD_FRAMES
            != self.lod.phase
        {
            return;
        }
        self.update_lod();
    }

    fn update_lod(&mut self) {
        // The impostor system disabled globally forces FULL, so the benchmark's OFF phase
        // measures full-mesh rendering with no impostor work.
        if !globals::engine()
            .bind()
            .setting("avatar_impostors_enabled".into())
            .booleanize()
        {
            if self.lod.state != LodState::Full {
                self.apply_lod_state(LodState::Full, 1.0, 0.0, 0.0, 0.0);
            }
            return;
        }
        // is_visible_in_tree, not the node's own flag: a scene NPC hidden by its parent entity
        // keeps visible=true, and its impostor lives on the avatar scene and ignores that.
        if !self.base().is_visible_in_tree() {
            self.hide_impostor_render();
            return;
        }
        let Some(camera) = self.base().get_viewport().and_then(|v| v.get_camera_3d()) else {
            return;
        };
        let dist = camera
            .get_global_position()
            .distance_to(self.base().get_global_position());
        let band = with_rank_cap(bands_for_distance(dist), self.lod.rank_cap);
        self.apply_lod_state(band.0, band.1, band.2, band.3, dist);
        // Reconcile against the notifier in case a screen signal was missed.
        self.apply_screen_freeze();
    }

    pub(crate) fn apply_lod_state(
        &mut self,
        state: LodState,
        dither_alpha: f32,
        fade_alpha: f32,
        tint: f32,
        distance: f32,
    ) {
        let changed = state != self.lod.state;
        let previous = self.lod.state;
        self.lod.state = state;
        self.set_dither_alpha(dither_alpha);
        let id = self.base().instance_id().to_i64();
        let mut scene = globals::avatars_scene();

        if self.lod.off_frustum {
            // The slot returns to the pool for an avatar that is on screen; re-entry rehydrates
            // from the disk cache.
            if self.lod.impostor_layer >= 0 {
                scene.bind_mut().clear_impostor(id);
                self.lod.impostor_layer = -1;
                self.lod.layer_is_overflow = false;
            }
        } else if state == LodState::Far {
            self.ensure_impostor_layer(distance);
            if self.lod.impostor_layer >= 0 {
                scene
                    .bind_mut()
                    .set_impostor_state(id, fade_alpha, tint, distance);
            }
        } else if self.lod.impostor_layer >= 0 {
            // Keep the slot across LOD oscillation: re-allocating would recapture every time
            // the camera swings between FAR and MID.
            scene.bind_mut().set_impostor_state(id, 0.0, 0.0, distance);
        }
        if changed {
            self.on_lod_state_changed(state, previous);
        }
    }

    fn ensure_impostor_layer(&mut self, distance: f32) {
        let id = self.base().instance_id().to_i64();
        let key = GString::from(&self.get_impostor_cache_key());
        let avatar = self.to_gd();
        let mut scene = globals::avatars_scene();
        // The scene dispatches: a fresh slot, or a render-mode toggle in place that keeps the
        // layer warm (no clear-then-realloc, so a brief off-frustum turn does not recapture).
        self.lod.impostor_layer = scene.bind_mut().request_impostor_layer(
            id,
            avatar,
            distance,
            self.lod.use_overflow,
            key,
        );
        if self.lod.impostor_layer < 0 {
            return;
        }
        self.lod.layer_is_overflow = self.lod.use_overflow;
        // Capture only when the slot owns a real layer that is not filled yet.
        if scene.bind().impostor_needs_capture(id) {
            crate::avatars::impostor_capture::request_capture(self.to_gd());
        }
    }

    /// Drops the impostor slot; used by the avatar scene when the avatar is removed.
    pub fn release_impostor(&mut self) {
        if self.lod.impostor_layer < 0 {
            return;
        }
        let id = self.base().instance_id().to_i64();
        globals::avatars_scene().bind_mut().clear_impostor(id);
        self.lod.impostor_layer = -1;
    }

    /// The outfit changed: the cached impostor texture is stale.
    pub(crate) fn refresh_impostor_texture(&mut self) {
        if self.lod.impostor_layer >= 0 && !self.lod.layer_is_overflow {
            let id = self.base().instance_id().to_i64();
            let key = GString::from(&self.get_impostor_cache_key());
            globals::avatars_scene()
                .bind_mut()
                .invalidate_impostor_texture(id, key);
            crate::avatars::impostor_capture::request_capture(self.to_gd());
        }
    }

    fn on_lod_state_changed(&mut self, state: LodState, _previous: LodState) {
        // At FAR the tree stops advancing, so a running emote would never reach its end and the
        // scene would wait forever for a terminal EmoteState: report it interrupted now.
        if state == LodState::Far && self.emote.is_playing() {
            self.stop_emote();
        }
        let full = state == LodState::Full;
        self.set_meshes_visible(state != LodState::Far);
        self.set_particles_visible(full);
        self.set_click_active(full);
        self.set_animation_speed(1.0);
        // Driven by the real screen state, so an off-screen avatar that changes LOD stays frozen.
        let (active, throttle) = resolve_anim_drive(self.lod.on_screen, state);
        self.set_animation_active(active);
        self.set_animation_throttle(throttle);
        self.apply_nickname_visibility();
    }

    /// Drives the freeze off Godot's own on-screen detection, not the coordinator's
    /// approximate, 6-frame-lagged flag: the freeze can never leave a visible avatar on a
    /// stale pose.
    pub(crate) fn setup_screen_notifier(&mut self) {
        let mut notifier = VisibleOnScreenNotifier3D::new_alloc();
        notifier.set_aabb(SCREEN_NOTIFIER_AABB);
        self.base_mut().add_child(&notifier);
        let on_enter = self.base().callable("on_avatar_screen_entered");
        let on_exit = self.base().callable("on_avatar_screen_exited");
        notifier.connect("screen_entered", &on_enter);
        notifier.connect("screen_exited", &on_exit);
        self.lod.screen_notifier = Some(notifier);
    }

    pub(crate) fn tick_animation_throttle(&mut self, delta: f64) {
        if !self.lod.throttle_active {
            return;
        }
        let Some(nodes) = self.nodes.as_mut() else {
            return;
        };
        self.lod.throttle_acc += delta;
        self.lod.throttle_counter += 1;
        if self.lod.throttle_counter >= MID_ANIM_ADVANCE_EVERY_N_FRAMES {
            nodes.animation_tree.advance(self.lod.throttle_acc);
            self.lod.throttle_acc = 0.0;
            self.lod.throttle_counter = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anim_drive_follows_screen_and_tier() {
        assert_eq!(resolve_anim_drive(false, LodState::Full), (false, false));
        assert_eq!(resolve_anim_drive(true, LodState::Full), (true, false));
        assert_eq!(resolve_anim_drive(true, LodState::Mid), (true, true));
        assert_eq!(resolve_anim_drive(true, LodState::Far), (false, false));
    }

    #[test]
    fn only_grounded_avatars_walk() {
        assert_eq!(
            locomotion_conditions(true, false, true, true),
            (true, false, true)
        );
        assert_eq!(
            locomotion_conditions(true, true, true, false),
            (false, false, false)
        );
    }

    #[test]
    fn distance_bands() {
        assert_eq!(bands_for_distance(5.0).0, LodState::Full);
        assert_eq!(bands_for_distance(40.0).0, LodState::Mid);
        let (state, dither, fade, tint) = bands_for_distance(95.0);
        assert_eq!((state, dither, fade), (LodState::Far, 0.0, 1.0));
        assert!(tint > 0.0 && tint < 1.0);
        assert_eq!(bands_for_distance(500.0).3, 1.0);
    }

    #[test]
    fn pool_cap_only_demotes() {
        let near = bands_for_distance(5.0);
        assert_eq!(with_rank_cap(near, LodState::Full), near);
        assert_eq!(with_rank_cap(near, LodState::Mid).0, LodState::Mid);
        let far_cap = with_rank_cap(near, LodState::Far);
        assert_eq!((far_cap.0, far_cap.1, far_cap.2), (LodState::Far, 0.0, 1.0));
        let far = bands_for_distance(100.0);
        assert_eq!(with_rank_cap(far, LodState::Mid), far);
    }
}
