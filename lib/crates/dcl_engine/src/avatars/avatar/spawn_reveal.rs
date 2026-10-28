//! #1554: the spawn/teleport hologram — a ghost body under the hologram shader while wearables
//! load, then one step into the real meshes. Re-triggering mid-reveal restarts the sweep instead
//! of queueing.

use std::sync::atomic::{AtomicBool, Ordering};

use godot::classes::{
    MeshInstance3D, Node3D, PackedScene, Shader, ShaderMaterial, Skeleton3D, Time,
};
use godot::prelude::*;

use crate::fetcher::globals;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::dcl_global::DclGlobal;

const HOLOGRAM_SHADER: &str = "res://assets/shaders/ghost_hologram.gdshader";
const GHOST_BODY_FEMALE: &str = "res://assets/avatar/ghost_body_female.glb";
const GHOST_BODY_MALE: &str = "res://assets/avatar/ghost_body_male.glb";

const GHOST_IN_DURATION: f32 = 0.8;
const REVEAL_HEIGHT: f32 = 2.1;
// After the sweep, hold a random beat so a crowd does not swap in lockstep.
const MIN_HOLD_MIN: f32 = 0.25;
const MIN_HOLD_MAX: f32 = 0.5;
/// Extra grace for the real meshes after the sweep.
const READY_TIMEOUT: f32 = 5.0;
/// Stop trying to start a reveal this long after the avatar was born.
const GIVE_UP_MS: u64 = 10_000;
const WARM_SECONDS: f64 = 0.5;

/// A load in progress hides spawn reveals: they belong to visible gameplay. Process-wide, so an
/// avatar spawning mid-load (the burst after a teleport) starts from the truth.
static WORLD_IS_LOADING: AtomicBool = AtomicBool::new(false);
static LOADING_SIGNALS_CONNECTED: AtomicBool = AtomicBool::new(false);
/// The hologram pipeline compiles once per process: warming it per avatar would spawn a pair of
/// skinned ghost bodies per avatar in a spawn burst.
static PIPELINE_WARMED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Phase {
    #[default]
    Idle,
    GhostIn,
    Hold,
}

pub struct SpawnReveal {
    /// A preview (backpack, renderer) shows a profile that never spawned, so it turns this off.
    pub enabled: bool,
    /// The trigger gate, not the reveal: whether one should ever start for this avatar.
    started: bool,
    /// Restarted when the profile arrives: a slow profile must not spend the give-up budget.
    pub born_ms: u64,
    phase: Phase,
    elapsed: f32,
    min_hold: f32,
    material: Option<Gd<ShaderMaterial>>,
    ghost: Option<Gd<Node3D>>,
    ghost_skeleton: Option<Gd<Skeleton3D>>,
    ghost_meshes: Vec<Gd<MeshInstance3D>>,
    /// Bone pairing resolved once per reveal: our bone and the matching ghost bone.
    bone_pairs: Vec<(i32, i32)>,
}

impl Default for SpawnReveal {
    fn default() -> Self {
        Self {
            enabled: true,
            started: false,
            born_ms: Time::singleton().get_ticks_msec(),
            phase: Phase::Idle,
            elapsed: 0.0,
            min_hold: 0.0,
            material: None,
            ghost: None,
            ghost_skeleton: None,
            ghost_meshes: Vec::new(),
            bone_pairs: Vec::new(),
        }
    }
}

impl SpawnReveal {
    pub fn is_idle(&self) -> bool {
        self.phase == Phase::Idle
    }
}

pub fn world_is_loading() -> bool {
    WORLD_IS_LOADING.load(Ordering::Relaxed)
}

fn find_skeleton_in(node: &Gd<Node>) -> Option<Gd<Skeleton3D>> {
    if let Ok(skeleton) = node.clone().try_cast::<Skeleton3D>() {
        return Some(skeleton);
    }
    node.get_children()
        .iter_shared()
        .find_map(|child| find_skeleton_in(&child))
}

fn hologram_material() -> Option<Gd<ShaderMaterial>> {
    let shader = try_load::<Shader>(HOLOGRAM_SHADER).ok()?;
    let mut material = ShaderMaterial::new_gd();
    material.set_shader(&shader);
    Some(material)
}

fn mesh_children(skeleton: &Gd<Skeleton3D>) -> Vec<Gd<MeshInstance3D>> {
    skeleton
        .get_children()
        .iter_shared()
        .filter_map(|c| c.try_cast::<MeshInstance3D>().ok())
        .collect()
}

impl DclAvatar {
    /// Gates reveals to visible gameplay (not behind loading screens) and warms the hologram
    /// pipeline, once per process: Godot draws a solid placeholder until the shader compiles.
    pub(crate) fn spawn_reveal_ready(&mut self) {
        if !LOADING_SIGNALS_CONNECTED.swap(true, Ordering::Relaxed) {
            let global = DclGlobal::singleton();
            global
                .signals()
                .loading_started()
                .connect(|| WORLD_IS_LOADING.store(true, Ordering::Relaxed));
            global
                .signals()
                .loading_finished()
                .connect(|| WORLD_IS_LOADING.store(false, Ordering::Relaxed));
        }
        if PIPELINE_WARMED.swap(true, Ordering::Relaxed) {
            return;
        }
        let Some(material) = hologram_material() else {
            return;
        };
        let mut warmers: Vec<Gd<Node3D>> = Vec::new();
        for path in [GHOST_BODY_FEMALE, GHOST_BODY_MALE] {
            let Some(mut inst) = try_load::<PackedScene>(path)
                .ok()
                .and_then(|scene| scene.instantiate())
                .and_then(|node| node.try_cast::<Node3D>().ok())
            else {
                continue;
            };
            inst.set_scale(Vector3::new(0.001, 0.001, 0.001));
            self.base_mut().add_child(&inst);
            if let Some(skeleton) = find_skeleton_in(&inst.clone().upcast()) {
                for mut mesh in mesh_children(&skeleton) {
                    mesh.set_material_override(&material);
                }
            }
            warmers.push(inst);
        }
        let Some(mut tree) = self.base().get_tree_or_null() else {
            return;
        };
        tree.create_timer(WARM_SECONDS)
            .signals()
            .timeout()
            .connect(move || {
                for inst in &warmers {
                    if inst.is_instance_valid() {
                        inst.clone().queue_free();
                    }
                }
            });
    }

    /// Avatars ghost in on first appearance: local spawn, teleports and remotes appearing. FULL
    /// LOD only (a FAR avatar is a billboard), and never behind the loading screen.
    pub(crate) fn spawn_reveal_trigger(&mut self) {
        if self.spawn_reveal.started {
            return;
        }
        let can_start = self.spawn_reveal.enabled
            && self.nametag.profile_ready
            && !self.avatar_name.is_empty()
            && self.lod.state == crate::avatars::impostor::LodState::Full
            && self.is_grounded
            && !world_is_loading();
        if can_start {
            self.spawn_reveal.started = true;
            self.spawn_reveal_start();
        } else if Time::singleton().get_ticks_msec() - self.spawn_reveal.born_ms > GIVE_UP_MS {
            // The profile never resolved (scene NPCs, fetch failures): never stay hidden.
            self.spawn_reveal.started = true;
            self.set_skeleton_visible(true);
        }
    }

    fn set_skeleton_visible(&mut self, visible: bool) {
        if let Some(nodes) = self.nodes.as_mut() {
            nodes.skeleton.set_visible(visible);
        }
    }

    fn spawn_reveal_start(&mut self) {
        let Some(our_skeleton) = self.nodes.as_ref().map(|n| n.skeleton.clone()) else {
            return;
        };
        if self.spawn_reveal.material.is_none() {
            self.spawn_reveal.material = hologram_material();
        }
        let Some(mut material) = self.spawn_reveal.material.clone() else {
            return;
        };
        // The name is resolved (trigger gate): tint by the profile's name color.
        let nc = DclAvatar::get_nickname_color(self.avatar_name.clone());
        material.set_shader_parameter(
            "fresnel_color",
            &Vector4::new(nc.r, nc.g, nc.b, 0.97).to_variant(),
        );
        self.spawn_reveal_end();
        // The ghost is the NAKED BASE BODY of the avatar's body type, the bundled GLB instanced
        // whole with its own armature (reparenting a mesh onto our skeleton does not engage
        // skinning). Bone poses copy from our skeleton.
        let body_type = self
            .avatar_data
            .bind()
            .get_body_shape()
            .to_string()
            .to_lowercase();
        let path = if body_type.contains("female") {
            GHOST_BODY_FEMALE
        } else {
            GHOST_BODY_MALE
        };
        let Some(mut ghost) = try_load::<PackedScene>(path)
            .ok()
            .and_then(|scene| scene.instantiate())
            .and_then(|node| node.try_cast::<Node3D>().ok())
        else {
            return;
        };
        ghost.set_name("__ghost");
        // The ghost GLB faces +Z, our rig faces -Z.
        ghost.set_rotation(Vector3::new(0.0, std::f32::consts::PI, 0.0));
        self.base_mut().add_child(&ghost);
        let ghost_skeleton = find_skeleton_in(&ghost.clone().upcast());
        let reveal = &mut self.spawn_reveal;
        reveal.ghost_meshes.clear();
        reveal.bone_pairs.clear();
        if let Some(mut ghost_skeleton) = ghost_skeleton.clone() {
            for mut mesh in mesh_children(&ghost_skeleton) {
                mesh.set_material_override(&material);
                reveal.ghost_meshes.push(mesh);
            }
            // Driven off OUR skeleton's rest proportions (same rig).
            ghost_skeleton.set_scale(Vector3::ONE);
            for ghost_bone in 0..ghost_skeleton.get_bone_count() {
                let ours = our_skeleton.find_bone(&ghost_skeleton.get_bone_name(ghost_bone));
                if ours >= 0 {
                    reveal.bone_pairs.push((ours, ghost_bone));
                }
            }
        }
        reveal.ghost = Some(ghost);
        reveal.ghost_skeleton = ghost_skeleton;
        reveal.phase = Phase::GhostIn;
        reveal.elapsed = 0.0;
        reveal.min_hold =
            godot::global::randf_range(MIN_HOLD_MIN as f64, MIN_HOLD_MAX as f64) as f32;
        self.set_skeleton_visible(false);
    }

    /// Clean instant swap: ghost out, real meshes on (one switch).
    pub(crate) fn spawn_reveal_end(&mut self) {
        let reveal = &mut self.spawn_reveal;
        if let Some(mut ghost) = reveal.ghost.take() {
            if ghost.is_instance_valid() {
                ghost.queue_free();
            }
        }
        reveal.ghost_skeleton = None;
        reveal.ghost_meshes.clear();
        reveal.bone_pairs.clear();
        reveal.phase = Phase::Idle;
        self.set_skeleton_visible(true);
        // A capture asked for while the ghost was up was skipped, so ask again now. Never
        // invalidate here: captures mid-reveal are blocked, so there is no texture to throw away.
        let id = self.base().instance_id().to_i64();
        if globals::avatars_scene().bind().impostor_needs_capture(id) {
            crate::avatars::impostor_capture::request_capture(self.to_gd());
        }
    }

    /// Runs every frame, even when the LOD and animation paths bail early: a frozen reveal is a
    /// permanently hidden avatar.
    pub(crate) fn spawn_reveal_tick(&mut self, delta: f64) {
        if self.spawn_reveal.phase == Phase::Idle {
            return;
        }
        let Some(our_skeleton) = self.nodes.as_ref().map(|n| n.skeleton.clone()) else {
            return;
        };
        let y = self.base().get_global_position().y;
        let avatar_ready = self.avatar_ready;
        let reveal = &mut self.spawn_reveal;
        reveal.elapsed += delta as f32;
        let Some(mut material) = reveal.material.clone() else {
            return;
        };
        // Keep the hologram override (the assembler stomps overrides) and mirror our pose onto
        // the ghost so it animates with us.
        for mesh in reveal.ghost_meshes.iter_mut() {
            if mesh.is_instance_valid() {
                mesh.set_material_override(&material);
            }
        }
        if let Some(ghost_skeleton) = reveal
            .ghost_skeleton
            .as_mut()
            .filter(|s| s.is_instance_valid())
        {
            for &(ours, theirs) in &reveal.bone_pairs {
                ghost_skeleton
                    .set_bone_pose_position(theirs, our_skeleton.get_bone_pose_position(ours));
                ghost_skeleton
                    .set_bone_pose_rotation(theirs, our_skeleton.get_bone_pose_rotation(ours));
            }
        }
        let mut finished = false;
        match reveal.phase {
            Phase::GhostIn => {
                let progress = (reveal.elapsed / GHOST_IN_DURATION).clamp(0.0, 1.0);
                material.set_shader_parameter(
                    "reveal_y",
                    &(y - 0.05 + progress * REVEAL_HEIGHT).to_variant(),
                );
                if progress >= 1.0 {
                    reveal.phase = Phase::Hold;
                }
            }
            Phase::Hold => {
                material.set_shader_parameter("reveal_y", &(y + REVEAL_HEIGHT).to_variant());
                let ready = avatar_ready || reveal.elapsed >= GHOST_IN_DURATION + READY_TIMEOUT;
                let held = reveal.elapsed >= GHOST_IN_DURATION + reveal.min_hold;
                finished = ready && held;
            }
            Phase::Idle => {}
        }
        if finished {
            self.spawn_reveal_end();
        }
    }
}
