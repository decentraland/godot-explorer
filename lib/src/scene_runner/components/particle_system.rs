use godot::{
    classes::{
        base_material_3d::{
            BillboardMode, BlendMode as GodotBlendMode, CullMode, Flags, ShadingMode,
            TextureFilter, TextureParam, Transparency,
        },
        gpu_particles_3d::EmitFlags,
        image::Format,
        mesh::PrimitiveType,
        particle_process_material::{EmissionShape, Parameter, ParticleFlags},
        ArrayMesh, GpuParticles3D, Gradient, GradientTexture1D, Image, ImageTexture, Material,
        Mesh, ParticleProcessMaterial, QuadMesh, StandardMaterial3D, Texture2D,
    },
    prelude::*,
};

use std::time::Instant;

use rand::Rng;

use crate::{
    content::content_mapping::DclContentMappingAndUrl,
    dcl::{
        components::{
            proto_components::{
                common::{Color4, FloatRange, Texture as DclProtoTexture},
                sdk::components::{pb_particle_system, PbParticleSystem},
            },
            SceneComponentId, SceneEntityId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_global::DclGlobal,
    scene_runner::scene::{Scene, SceneType},
};

/// Hard cap of live particles per emitter, regardless of what the scene requests.
const MAX_AMOUNT_PER_EMITTER: i32 = 5_000;
/// Budget of live particles summed across every emitter of a single scene.
/// TODO: scale down on mobile / low-end (hook into dynamic graphics manager).
const SCENE_PARTICLE_BUDGET: i32 = 50_000;
/// Gravity the `gravity` field multiplies (matches Explorer's physics gravity).
const DCL_GRAVITY: f32 = -9.81;
/// Burst cycle length in seconds. Matches the Unity Explorer reference, whose
/// SDKParticleSystem prefab has a fixed 5s duration (bursts fire "seconds from
/// start of cycle").
const BURST_CYCLE_SECONDS: f32 = 5.0;
/// Local axis cone/box shapes emit along. Unity's convention is local +Z, but
/// DCL->Godot conversion flips Z (see DclTransformAndParent), so it maps to -Z.
const EMISSION_AXIS: Vector3 = Vector3::new(0.0, 0.0, -1.0);

/// Convert a vector from DCL world space to Godot world space (Z is flipped).
fn dcl_world_to_godot(v: &crate::dcl::components::proto_components::common::Vector3) -> Vector3 {
    Vector3::new(v.x, v.y, -v.z)
}

/// Convert a quaternion from DCL space to Godot space (mirrors
/// `DclTransformAndParent::to_godot_transform_3d`). Sanitizes invalid input:
/// scenes can send zero/unnormalized quaternions, and `Basis::from_quaternion`
/// panics on those.
fn dcl_quat_to_godot(x: f32, y: f32, z: f32, w: f32) -> Quaternion {
    let quat = Quaternion::new(x, y, -z, w);
    let len_sq = quat.length_squared();
    if !len_sq.is_finite() || len_sq < 1e-12 {
        return Quaternion::IDENTITY;
    }
    // Always renormalize: glam's is_normalized assert is stricter than float noise.
    quat / len_sq.sqrt()
}

pub struct ParticleSystemItem {
    pub node: Gd<GpuParticles3D>,
    pub material: Gd<StandardMaterial3D>,
    pub last_value: PbParticleSystem,
    pub amount: i32,
    pub waiting_texture: bool,
    /// Content hash (or http URL) the texture fetch was requested with.
    pub texture_hash: Option<String>,
    // --- Burst runtime state ---
    pub bursts: Vec<BurstRuntime>,
    pub started_at: Instant,
    pub last_cycle: u32,
    pub playing: bool,
    pub looping: bool,
    pub lifetime: f32,
    /// Whether the primary user was inside this scene's parcels on the previous
    /// frame (drives the stop-and-clear / resume policy).
    pub user_in_scene: bool,
}

/// Runtime state of one proto `Burst` entry. Mirrors Unity-style burst semantics:
/// fires `count` particles at `time` seconds into the cycle, repeating every
/// `interval` seconds up to `cycles` times (`None` = infinite).
pub struct BurstRuntime {
    pub time: f32,
    pub count: u32,
    pub interval: f32,
    pub probability: f32,
    pub cycles: Option<u32>,
    pub remaining_cycles: Option<u32>,
    pub next_fire: f32,
}

impl BurstRuntime {
    fn from_proto(burst: &pb_particle_system::Burst) -> Self {
        let cycles = burst.cycles.unwrap_or(1);
        let cycles = if cycles <= 0 {
            None
        } else {
            Some(cycles as u32)
        };
        // Unity fires bursts with time in [0, duration]; time == duration fires at
        // the loop boundary (end of cycle), anything beyond never fires.
        let raw_time = burst.time.max(0.0);
        let time = if raw_time > BURST_CYCLE_SECONDS {
            f32::MAX
        } else if raw_time == BURST_CYCLE_SECONDS {
            BURST_CYCLE_SECONDS - 0.001
        } else {
            raw_time
        };
        Self {
            time,
            count: burst.count,
            interval: burst.interval.unwrap_or(0.01).max(0.001),
            probability: burst.probability.unwrap_or(1.0).clamp(0.0, 1.0),
            cycles,
            remaining_cycles: cycles,
            next_fire: time,
        }
    }

    fn reset_for_new_cycle(&mut self) {
        self.remaining_cycles = self.cycles;
        self.next_fire = self.time;
    }
}

pub fn update_particle_system(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    current_parcel_scene_id: &crate::dcl::SceneId,
) {
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let particle_system_component = SceneCrdtStateProtoComponents::get_particle_system(crdt_state);

    // Policy (matches the Unity Explorer): particle systems only run in the scene
    // the primary user is currently in.
    let user_in_scene = if let SceneType::Parcel = scene.scene_type {
        &scene.scene_id == current_parcel_scene_id
    } else {
        true
    };

    if let Some(particle_system_dirty) =
        dirty_lww_components.get(&SceneComponentId::PARTICLE_SYSTEM)
    {
        let mut content_provider = DclGlobal::singleton().bind().get_content_provider();

        for entity in particle_system_dirty {
            let new_value = particle_system_component.get(entity);
            if new_value.is_none() {
                scene.particle_systems.remove(entity);
                continue;
            }

            let new_value = new_value.unwrap();
            let (_godot_entity_node, mut node_3d) = scene.godot_dcl_scene.ensure_node_3d(entity);

            let new_value = new_value.value.clone();
            let existing = node_3d.try_get_node_as::<GpuParticles3D>("ParticleSystem");

            if new_value.is_none() {
                if let Some(mut particle_node) = existing {
                    node_3d.remove_child(&particle_node);
                    particle_node.queue_free();
                }
                scene.particle_systems.remove(entity);
                continue;
            }

            let new_value = new_value.unwrap();

            // Skip if the component didn't actually change
            if let Some(item) = scene.particle_systems.get(entity) {
                if item.last_value == new_value {
                    continue;
                }
            }

            if let Ok(json) = serde_json::to_string(&new_value) {
                tracing::debug!("ParticleSystem apply entity {:?}: {}", entity, json);
            }

            let mut particle_node = if let Some(particle_node) = existing {
                particle_node
            } else {
                let mut new_node = GpuParticles3D::new_alloc();
                new_node.set_name("ParticleSystem");
                node_3d.add_child(&new_node.clone().upcast::<Node>());
                new_node
            };

            let (material, amount) =
                apply_particle_system(scene, *entity, &mut particle_node, &new_value);

            // Kick off texture fetch (async, polled below on following frames).
            // The content provider keys textures by content hash, so resolve the
            // scene-relative path first (http(s) URLs are used as-is).
            let texture_hash = new_value.texture.as_ref().and_then(|texture| {
                let src = texture.src.as_str();
                if src.is_empty() {
                    None
                } else if src.starts_with("http") {
                    crate::content::external_content::register_external_texture(
                        &scene.scene_entity_definition.id,
                        src,
                    );
                    Some(src.to_string())
                } else {
                    match scene.content_mapping.get_hash(src) {
                        Some(hash) => Some(hash.clone()),
                        None => {
                            tracing::warn!(
                                "ParticleSystem texture path not found in content mapping: {src}"
                            );
                            None
                        }
                    }
                }
            });

            let waiting_texture = if let Some(hash) = &texture_hash {
                content_provider.call_deferred(
                    "fetch_texture_by_hash",
                    &[
                        hash.to_godot().to_variant(),
                        DclContentMappingAndUrl::from_ref(scene.content_mapping.clone())
                            .to_variant(),
                    ],
                );
                true
            } else {
                false
            };

            let playing = new_value.active.unwrap_or(true)
                && new_value
                    .playback_state
                    .unwrap_or(pb_particle_system::PlaybackState::PsPlaying as i32)
                    == pb_particle_system::PlaybackState::PsPlaying as i32;
            let bursts = new_value
                .bursts
                .as_ref()
                .map(|b| b.values.iter().map(BurstRuntime::from_proto).collect())
                .unwrap_or_default();

            if !user_in_scene {
                // Created while the user is elsewhere: don't emit until they enter
                // (mirrors Unity, which doesn't even instance the ParticleSystem).
                particle_node.set_emitting(false);
            }

            scene.particle_systems.insert(
                *entity,
                ParticleSystemItem {
                    node: particle_node,
                    material,
                    amount,
                    waiting_texture,
                    texture_hash,
                    bursts,
                    started_at: Instant::now(),
                    last_cycle: 0,
                    playing,
                    looping: new_value.r#loop.unwrap_or(true),
                    lifetime: new_value.lifetime.unwrap_or(5.0).max(0.01),
                    user_in_scene,
                    last_value: new_value,
                },
            );

            scene.dirty_particle_systems = true;
        }
    }

    if scene.dirty_particle_systems {
        poll_particle_textures(scene);
    }

    reconcile_user_presence(scene, user_in_scene);
    drive_bursts(scene);
}

/// Enforce the presence policy (matches Unity's ParticleSystemPlaybackSystem):
/// when the primary user leaves the scene, stop emitting and kill every particle;
/// when they re-enter, restart from scratch — but only for systems whose playback
/// state is PLAYING (paused/stopped systems are left untouched).
///
/// Called every frame from `update_particle_system` (idle scenes tick slowly,
/// every ~120s) and immediately on scene changes from
/// `SceneManager::on_current_parcel_scene_changed`.
pub fn reconcile_user_presence(scene: &mut Scene, user_in_scene: bool) {
    let now = Instant::now();

    for item in scene.particle_systems.values_mut() {
        match (item.user_in_scene, user_in_scene) {
            (true, false) => {
                if item.playing {
                    // restart() clears live particles but force-enables emitting,
                    // so the stop must come after it.
                    item.node.restart();
                    item.node.set_emitting(false);
                }
            }
            (false, true) => {
                if item.playing {
                    // Unity's Play() restarts the system: fresh burst cycle too.
                    item.started_at = now;
                    item.last_cycle = 0;
                    for burst in &mut item.bursts {
                        burst.reset_for_new_cycle();
                    }
                    item.node.set_emitting(true);
                    item.node.restart();
                }
            }
            _ => {}
        }
        item.user_in_scene = user_in_scene;
    }
}

/// Fire pending emission bursts via manual `emit_particle` calls (GPUParticles3D
/// has no native timed-burst support). Runs every frame for every scene emitter.
fn drive_bursts(scene: &mut Scene) {
    let now = Instant::now();

    for item in scene.particle_systems.values_mut() {
        if item.bursts.is_empty() || !item.playing || !item.user_in_scene {
            continue;
        }

        let elapsed = now.duration_since(item.started_at).as_secs_f32();
        let (cycle, cycle_time) = if item.looping {
            (
                (elapsed / BURST_CYCLE_SECONDS) as u32,
                elapsed.rem_euclid(BURST_CYCLE_SECONDS),
            )
        } else {
            (0, elapsed)
        };

        if cycle != item.last_cycle {
            item.last_cycle = cycle;
            for burst in &mut item.bursts {
                burst.reset_for_new_cycle();
            }
        }

        let mut rng = rand::thread_rng();
        let mut pending: Vec<(u32, f32)> = Vec::new(); // (count, probability)
        for burst in &mut item.bursts {
            while burst.next_fire <= cycle_time && burst.remaining_cycles != Some(0) {
                pending.push((burst.count, burst.probability));
                if let Some(remaining) = &mut burst.remaining_cycles {
                    *remaining -= 1;
                }
                burst.next_fire += burst.interval;
            }
        }

        for (count, probability) in pending {
            if rng.gen::<f32>() <= probability {
                emit_burst_particles(&mut item.node, &item.last_value, count, &mut rng);
            }
        }
    }
}

/// Poll pending particle textures and apply them once loaded (mirrors the
/// `dirty_materials` flow in material.rs).
fn poll_particle_textures(scene: &mut Scene) {
    let mut keep_dirty = false;
    let mut content_provider = DclGlobal::singleton().bind().get_content_provider();

    for item in scene.particle_systems.values_mut() {
        if !item.waiting_texture {
            continue;
        }

        let Some(hash) = item.texture_hash.clone() else {
            item.waiting_texture = false;
            continue;
        };

        let mut cp = content_provider.bind_mut();
        if cp.is_resource_from_hash_loaded(hash.to_godot()) {
            if let Some(resource) = cp.get_texture_from_hash(hash.to_godot()) {
                if resource.get_width() > 0 && resource.get_height() > 0 {
                    item.material.set_texture(TextureParam::ALBEDO, &resource);
                } else {
                    tracing::warn!(
                        "Skipping invalid particle texture (0x0) for hash {}, may cause rendering issues",
                        hash
                    );
                }
            }
            item.waiting_texture = false;
        } else {
            keep_dirty = true;
        }
    }

    scene.dirty_particle_systems = keep_dirty;
}

/// Map a `PBParticleSystem` onto a `GpuParticles3D` node. Returns the draw-pass
/// material so the caller can keep it alive and patch the texture in later.
fn apply_particle_system(
    scene: &Scene,
    entity: SceneEntityId,
    node: &mut Gd<GpuParticles3D>,
    value: &PbParticleSystem,
) -> (Gd<StandardMaterial3D>, i32) {
    let lifetime = value.lifetime.unwrap_or(5.0).max(0.01) as f64;
    let rate = value.rate.unwrap_or(10.0).max(0.0);
    let max_particles = value.max_particles.unwrap_or(1000);

    // Godot emits `amount` particles per `lifetime` cycle, so amount = rate * lifetime
    // gives the requested rate. Manually-driven bursts also draw from this pool, so
    // reserve room for the particles they can have alive simultaneously.
    let burst_capacity = value
        .bursts
        .as_ref()
        .map(|bursts| {
            bursts
                .values
                .iter()
                .map(|burst| {
                    let interval = burst.interval.unwrap_or(0.01).max(0.001);
                    let window = (lifetime as f32 - burst.time.max(0.0)).max(0.0);
                    let max_fires = (window / interval) as u32 + 1;
                    let fires = match burst.cycles.unwrap_or(1) {
                        cycles if cycles <= 0 => max_fires,
                        cycles => (cycles as u32).min(max_fires),
                    };
                    burst.count.saturating_mul(fires)
                })
                .sum::<u32>()
        })
        .unwrap_or(0);

    // Clamp by the component cap, the per-emitter hard cap and the per-scene budget.
    let upper_cap = (max_particles as i32).clamp(1, MAX_AMOUNT_PER_EMITTER);
    let continuous = ((rate * lifetime as f32).ceil() as i32).max(0);
    let mut amount = continuous.max(burst_capacity as i32).clamp(1, upper_cap);
    let used_elsewhere: i32 = scene
        .particle_systems
        .iter()
        .filter(|(e, _)| **e != entity)
        .map(|(_, item)| item.amount)
        .sum();
    let remaining_budget = (SCENE_PARTICLE_BUDGET - used_elsewhere).max(0);
    if amount > remaining_budget {
        tracing::warn!(
            "ParticleSystem on entity {:?} clamped from {} to {} particles (scene budget)",
            entity,
            amount,
            remaining_budget
        );
        amount = remaining_budget.max(1);
    }

    node.set_amount(amount);
    node.set_lifetime(lifetime);

    let looping = value.r#loop.unwrap_or(true);
    node.set_one_shot(!looping);
    node.set_pre_process_time(if value.prewarm.unwrap_or(false) && looping {
        lifetime
    } else {
        0.0
    });

    let world_space =
        value.simulation_space == Some(pb_particle_system::SimulationSpace::PssWorld as i32);
    node.set_use_local_coordinates(!world_space);

    // Playback state + active flag
    let active = value.active.unwrap_or(true);
    let playback = value
        .playback_state
        .unwrap_or(pb_particle_system::PlaybackState::PsPlaying as i32);
    match playback {
        x if x == pb_particle_system::PlaybackState::PsPaused as i32 => {
            node.set_emitting(false);
            node.set_speed_scale(0.0);
        }
        x if x == pb_particle_system::PlaybackState::PsStopped as i32 => {
            node.set_speed_scale(1.0);
            node.restart(); // clears existing particles (and re-enables emitting)
            node.set_emitting(false);
        }
        _ => {
            node.set_speed_scale(1.0);
            node.set_emitting(active);
        }
    }

    // --- Process material ---
    let mut process_material = ParticleProcessMaterial::new_gd();
    apply_shape(&mut process_material, value);
    apply_motion(&mut process_material, value);
    apply_size_and_rotation(&mut process_material, value);
    apply_color(&mut process_material, value);
    apply_sprite_sheet(&mut process_material, value, lifetime);

    node.set_process_material(&process_material.upcast::<Material>());

    // --- Draw pass (textured quad) ---
    let material = build_draw_material(value);
    let mesh = build_draw_mesh(value, &material);
    node.set_draw_passes(1);
    node.set_draw_pass_mesh(0, &mesh);

    // Visibility AABB: particles are culled when this box leaves the screen, so
    // derive a generous bound from how far particles can travel in a lifetime.
    let speed_max = float_range(value.initial_velocity_speed.as_ref(), 1.0, 1.0).1;
    let shape_extent = match &value.shape {
        Some(pb_particle_system::Shape::Sphere(s)) => s.radius.unwrap_or(1.0),
        Some(pb_particle_system::Shape::Cone(c)) => c.radius.unwrap_or(1.0),
        Some(pb_particle_system::Shape::Box(b)) => b
            .size
            .as_ref()
            .map(|s| s.x.max(s.y).max(s.z))
            .unwrap_or(1.0),
        _ => 0.0,
    };
    let reach = (speed_max * lifetime as f32 + shape_extent + 2.0).max(4.0);
    node.set_visibility_aabb(Aabb {
        position: Vector3::new(-reach, -reach, -reach),
        size: Vector3::new(reach * 2.0, reach * 2.0, reach * 2.0),
    });

    (material, amount)
}

fn float_range(range: Option<&FloatRange>, default_start: f32, default_end: f32) -> (f32, f32) {
    range
        .map(|r| (r.start, r.end))
        .unwrap_or((default_start, default_end))
}

fn apply_shape(process_material: &mut ParticleProcessMaterial, value: &PbParticleSystem) {
    match &value.shape {
        Some(pb_particle_system::Shape::Sphere(sphere)) => {
            process_material.set_emission_shape(EmissionShape::SPHERE);
            process_material.set_emission_sphere_radius(sphere.radius.unwrap_or(1.0));
            process_material.set_spread(180.0);
        }
        Some(pb_particle_system::Shape::Cone(cone)) => {
            // Godot has no cone emission shape; a flat ring + forward direction with
            // the cone half-angle as spread approximates it. Axis matches Unity's
            // cone (emits along local +Z).
            process_material.set_emission_shape(EmissionShape::RING);
            process_material.set_emission_ring_axis(EMISSION_AXIS);
            process_material.set_emission_ring_radius(cone.radius.unwrap_or(1.0));
            process_material.set_emission_ring_inner_radius(0.0);
            process_material.set_emission_ring_height(0.0);
            process_material.set_direction(EMISSION_AXIS);
            process_material.set_spread(cone.angle.unwrap_or(25.0).clamp(0.0, 90.0));
        }
        Some(pb_particle_system::Shape::Box(b)) => {
            process_material.set_emission_shape(EmissionShape::BOX);
            let size = b.size.clone().unwrap_or(
                crate::dcl::components::proto_components::common::Vector3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
            );
            process_material.set_emission_box_extents(Vector3::new(
                size.x * 0.5,
                size.y * 0.5,
                size.z * 0.5,
            ));
            // Unity's box shape emits along local +Z (not in random directions).
            process_material.set_direction(EMISSION_AXIS);
            process_material.set_spread(0.0);
        }
        // Point (or missing shape): emit from origin in every direction
        _ => {
            process_material.set_emission_shape(EmissionShape::POINT);
            process_material.set_spread(180.0);
        }
    }
}

fn apply_motion(process_material: &mut ParticleProcessMaterial, value: &PbParticleSystem) {
    let (speed_min, speed_max) = float_range(value.initial_velocity_speed.as_ref(), 1.0, 1.0);
    process_material.set_param_min(Parameter::INITIAL_LINEAR_VELOCITY, speed_min);
    process_material.set_param_max(Parameter::INITIAL_LINEAR_VELOCITY, speed_max);

    // Gravity is a multiplier of the Explorer physics gravity; additional_force is
    // a constant acceleration in world space, so both fold into Godot's gravity vector.
    let gravity = value.gravity.unwrap_or(0.0) * DCL_GRAVITY;
    let mut gravity_vec = Vector3::new(0.0, gravity, 0.0);
    if let Some(force) = &value.additional_force {
        // additional_force is in DCL world space; convert to Godot world space.
        gravity_vec += dcl_world_to_godot(force);
    }
    process_material.set_gravity(gravity_vec);

    // Limit velocity over lifetime: Godot 4.6 has a native velocity_limit_curve
    // (hard magnitude clamp), which is exactly dampen = 1 (the proto default).
    if let Some(limit_velocity) = &value.limit_velocity {
        let curve = constant_red_texture(limit_velocity.speed.max(0.0));
        process_material.set_velocity_limit_curve(&curve.upcast::<Texture2D>());

        let dampen = limit_velocity.dampen.unwrap_or(1.0);
        if dampen < 0.99 {
            // A custom particles ShaderMaterial could remove the excess fraction
            // per-frame instead of clamping; not worth a second code path for now.
            tracing::warn!(
                "ParticleSystem.limit_velocity.dampen ({dampen}) approximated as hard clamp"
            );
        }
    }
}

fn apply_size_and_rotation(
    process_material: &mut ParticleProcessMaterial,
    value: &PbParticleSystem,
) {
    let (size_min, size_max) = float_range(value.initial_size.as_ref(), 1.0, 1.0);
    process_material.set_param_min(Parameter::SCALE, size_min);
    process_material.set_param_max(Parameter::SCALE, size_max);

    // size_over_time lerps from start to end over the lifetime; Godot's scale curve
    // multiplies the base scale. Baked to a float texture (CurveTexture would clamp
    // to [0, 1] and break growth over 1x).
    if let Some(size_over_time) = &value.size_over_time {
        let curve_texture = gray_curve_texture(size_over_time.start, size_over_time.end);
        process_material.set_param_texture(Parameter::SCALE, &curve_texture.upcast::<Texture2D>());
    }

    // Rotations: billboard quads can only spin around their view axis, so we reduce
    // the 3D rotation to its Z euler component (documented limitation).
    if let Some(rotation) = &value.initial_rotation {
        let angle = quaternion_z_degrees(rotation.x, rotation.y, rotation.z, rotation.w);
        process_material.set_param_min(Parameter::ANGLE, angle);
        process_material.set_param_max(Parameter::ANGLE, angle);
    }
    if let Some(rotation) = &value.rotation_over_time {
        let angular_velocity = quaternion_z_degrees(rotation.x, rotation.y, rotation.z, rotation.w);
        process_material.set_param_min(Parameter::ANGULAR_VELOCITY, angular_velocity);
        process_material.set_param_max(Parameter::ANGULAR_VELOCITY, angular_velocity);
    }

    process_material.set_particle_flag(
        ParticleFlags::ALIGN_Y_TO_VELOCITY,
        value.face_travel_direction.unwrap_or(false),
    );
}

fn apply_color(process_material: &mut ParticleProcessMaterial, value: &PbParticleSystem) {
    // Each particle's initial color is sampled randomly from this ramp: a two-color
    // gradient gives exactly the "random between start and end" SDK semantic.
    if let Some(initial_color) = &value.initial_color {
        let ramp = gradient_texture(
            color_or_white(initial_color.start.as_ref()),
            color_or_white(initial_color.end.as_ref()),
        );
        process_material.set_color_initial_ramp(&ramp.upcast::<Texture2D>());
    }

    // Color lerped over the particle lifetime.
    if let Some(color_over_time) = &value.color_over_time {
        let ramp = gradient_texture(
            color_or_white(color_over_time.start.as_ref()),
            color_or_white(color_over_time.end.as_ref()),
        );
        process_material.set_color_ramp(&ramp.upcast::<Texture2D>());
    }
}

fn apply_sprite_sheet(
    process_material: &mut ParticleProcessMaterial,
    value: &PbParticleSystem,
    lifetime: f64,
) {
    let Some(sprite_sheet) = &value.sprite_sheet else {
        return;
    };
    // Clamp tiles to a sane atlas size (a scene could send huge values and
    // overflow the multiplication otherwise).
    let tiles_x = sprite_sheet.tiles_x.clamp(1, 64);
    let tiles_y = sprite_sheet.tiles_y.clamp(1, 64);
    let total_frames = (tiles_x * tiles_y) as f32;
    let fps = sprite_sheet.frames_per_second.unwrap_or(30.0);
    // anim_speed is expressed in full-animation cycles per particle lifetime.
    let anim_speed = fps * lifetime as f32 / total_frames;
    process_material.set_param_min(Parameter::ANIM_SPEED, anim_speed);
    process_material.set_param_max(Parameter::ANIM_SPEED, anim_speed);
    process_material.set_param_min(Parameter::ANIM_OFFSET, 0.0);
    process_material.set_param_max(Parameter::ANIM_OFFSET, 1.0);
}

/// 1x1 quad for the draw pass. For non-billboard particles with an initial
/// rotation, the proto gives a single quaternion shared by all particles, so we
/// bake it into the mesh vertices (the process material can only spin billboards
/// around their view axis).
fn build_draw_mesh(value: &PbParticleSystem, material: &Gd<StandardMaterial3D>) -> Gd<Mesh> {
    let billboard = value.billboard.unwrap_or(true);

    if !billboard && value.initial_rotation.is_some() {
        let q = value.initial_rotation.as_ref().unwrap();
        let rotation = Basis::from_quaternion(dcl_quat_to_godot(q.x, q.y, q.z, q.w));

        let corners = [
            Vector2::new(-0.5, -0.5),
            Vector2::new(0.5, -0.5),
            Vector2::new(-0.5, 0.5),
            Vector2::new(0.5, 0.5),
        ];
        let vertices: Vec<Vector3> = corners
            .iter()
            .map(|c| rotation * Vector3::new(c.x, c.y, 0.0))
            .collect();
        let normal = rotation * Vector3::BACK;
        let uvs = [
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
        ];
        let indices = [0i32, 1, 2, 2, 1, 3];

        let mut arrays = VarArray::new();
        arrays.resize(13, &VarArray::new().to_variant());
        arrays.set(
            0,
            &PackedVector3Array::from(vertices.as_slice()).to_variant(),
        );
        arrays.set(1, &PackedVector3Array::from(&[normal; 4]).to_variant());
        arrays.set(4, &PackedVector2Array::from(&uvs).to_variant());
        arrays.set(12, &PackedInt32Array::from(&indices).to_variant());

        let mut array_mesh = ArrayMesh::new_gd();
        array_mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
        array_mesh.surface_set_material(0, &material.clone().upcast::<Material>());
        array_mesh.upcast::<Mesh>()
    } else {
        let mut quad = QuadMesh::new_gd();
        quad.set_size(Vector2::new(1.0, 1.0));
        quad.set_material(&material.clone().upcast::<Material>());
        quad.upcast::<Mesh>()
    }
}

fn build_draw_material(value: &PbParticleSystem) -> Gd<StandardMaterial3D> {
    let mut material = StandardMaterial3D::new_gd();
    material.set_shading_mode(ShadingMode::UNSHADED);
    material.set_transparency(Transparency::ALPHA);
    // Single-sided like Unity's SDKParticleSystemMaterial (_Cull: back). With
    // both faces drawn, every semi-transparent particle contributes twice.
    material.set_cull_mode(CullMode::BACK);
    // Particle color (ramps above) reaches the shader as vertex color.
    material.set_flag(Flags::ALBEDO_FROM_VERTEX_COLOR, true);

    match value.blend_mode {
        Some(x) if x == pb_particle_system::BlendMode::PsbAdd as i32 => {
            material.set_blend_mode(GodotBlendMode::ADD);
        }
        Some(x) if x == pb_particle_system::BlendMode::PsbMultiply as i32 => {
            material.set_blend_mode(GodotBlendMode::MUL);
        }
        _ => {
            material.set_blend_mode(GodotBlendMode::MIX);
        }
    }

    if value.billboard.unwrap_or(true) {
        material.set_billboard_mode(BillboardMode::PARTICLES);
        // The particle-billboard shader path normalizes scale away unless this
        // flag is set; without it every particle renders at raw mesh size.
        material.set_flag(Flags::BILLBOARD_KEEP_SCALE, true);
    } else {
        material.set_billboard_mode(BillboardMode::DISABLED);
    }

    if let Some(texture) = &value.texture {
        apply_texture_filter(&mut material, texture);
    }

    if let Some(sprite_sheet) = &value.sprite_sheet {
        material.set_particles_anim_h_frames(sprite_sheet.tiles_x.clamp(1, 64) as i32);
        material.set_particles_anim_v_frames(sprite_sheet.tiles_y.clamp(1, 64) as i32);
        material.set_particles_anim_loop(true);
    }

    material
}

fn apply_texture_filter(material: &mut StandardMaterial3D, texture: &DclProtoTexture) {
    use crate::dcl::components::proto_components::common::TextureFilterMode;
    let filter = match texture.filter_mode {
        Some(x) if x == TextureFilterMode::TfmPoint as i32 => TextureFilter::NEAREST,
        Some(x) if x == TextureFilterMode::TfmTrilinear as i32 => {
            TextureFilter::LINEAR_WITH_MIPMAPS
        }
        _ => TextureFilter::LINEAR,
    };
    material.set_texture_filter(filter);
}

fn color_or_white(color: Option<&Color4>) -> Color {
    // Proto colors are display-referred (authored like Unity's startColor), but
    // the particle pipeline feeds them to the renderer as-is; converting to
    // linear keeps the on-screen result identical to the Unity Explorer.
    color
        .map(|c| c.to_godot().srgb_to_linear())
        .unwrap_or(Color::WHITE)
}

fn gradient_texture(start: Color, end: Color) -> Gd<GradientTexture1D> {
    let mut gradient = Gradient::new_gd();
    gradient.set_color(0, start);
    gradient.set_color(1, end);
    let mut texture = GradientTexture1D::new_gd();
    texture.set_gradient(&gradient);
    texture
}

/// Manually emit `count` burst particles, sampling spawn position/direction from
/// the emitter shape and speed/color from the component ranges (mirrors the
/// randomization the process material does for natural emission).
fn emit_burst_particles(
    node: &mut Gd<GpuParticles3D>,
    value: &PbParticleSystem,
    count: u32,
    rng: &mut rand::rngs::ThreadRng,
) {
    let (speed_min, speed_max) = float_range(value.initial_velocity_speed.as_ref(), 1.0, 1.0);
    let world_space =
        value.simulation_space == Some(pb_particle_system::SimulationSpace::PssWorld as i32);
    let node_global = if world_space {
        node.get_global_transform()
    } else {
        Transform3D::IDENTITY
    };

    for _ in 0..count.min(MAX_AMOUNT_PER_EMITTER as u32) {
        let (local_pos, local_dir) = sample_shape(value.shape.as_ref(), rng);
        let speed = if (speed_max - speed_min).abs() < f32::EPSILON {
            speed_min
        } else {
            rng.gen_range(speed_min..=speed_max)
        };

        let color = match &value.initial_color {
            Some(range) => {
                let t = rng.gen::<f32>();
                let start = color_or_white(range.start.as_ref());
                let end = color_or_white(range.end.as_ref());
                start.lerp(end, t as f64)
            }
            None => Color::WHITE,
        };

        // In world-space simulation, spawn coordinates are global.
        let xform = node_global
            * Transform3D {
                basis: Basis::IDENTITY,
                origin: local_pos,
            };
        let velocity = node_global.basis * (local_dir * speed);

        node.emit_particle(
            xform,
            velocity,
            color,
            Color::TRANSPARENT_BLACK,
            EmitFlags::POSITION | EmitFlags::VELOCITY | EmitFlags::COLOR,
        );
    }
}

/// Sample a spawn position and initial direction from the emitter shape,
/// matching the process-material mapping in `apply_shape`.
fn sample_shape(
    shape: Option<&pb_particle_system::Shape>,
    rng: &mut rand::rngs::ThreadRng,
) -> (Vector3, Vector3) {
    match shape {
        Some(pb_particle_system::Shape::Sphere(sphere)) => {
            let radius = sphere.radius.unwrap_or(1.0);
            let dir = random_unit_vector(rng);
            // Uniform in the sphere volume (matches EMISSION_SHAPE_SPHERE)
            (
                dir * radius * rng.gen::<f32>().cbrt(),
                random_unit_vector(rng),
            )
        }
        Some(pb_particle_system::Shape::Cone(cone)) => {
            let radius = cone.radius.unwrap_or(1.0);
            let half_angle = cone.angle.unwrap_or(25.0).clamp(0.0, 90.0).to_radians();
            // Uniform on the base disc (ring with inner radius 0), axis +Y
            let r = radius * rng.gen::<f32>().sqrt();
            let theta = rng.gen::<f32>() * std::f32::consts::TAU;
            let pos = Vector3::new(r * theta.cos(), r * theta.sin(), 0.0);
            // Uniform within the cone around the emission axis
            let cos_half = half_angle.cos();
            let cos_theta = rng.gen_range(cos_half..=1.0);
            let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
            let phi = rng.gen::<f32>() * std::f32::consts::TAU;
            let dir = Vector3::new(
                sin_theta * phi.cos(),
                sin_theta * phi.sin(),
                cos_theta * EMISSION_AXIS.z,
            );
            (pos, dir)
        }
        Some(pb_particle_system::Shape::Box(b)) => {
            let size = b
                .size
                .as_ref()
                .map(|s| Vector3::new(s.x, s.y, s.z))
                .unwrap_or(Vector3::ONE);
            let pos = Vector3::new(
                (rng.gen::<f32>() - 0.5) * size.x,
                (rng.gen::<f32>() - 0.5) * size.y,
                (rng.gen::<f32>() - 0.5) * size.z,
            );
            // Unity's box shape emits along local +Z
            (pos, EMISSION_AXIS)
        }
        // Point (or missing shape)
        _ => (Vector3::ZERO, random_unit_vector(rng)),
    }
}

fn random_unit_vector(rng: &mut rand::rngs::ThreadRng) -> Vector3 {
    loop {
        let v = Vector3::new(
            rng.gen_range(-1.0..=1.0),
            rng.gen_range(-1.0..=1.0),
            rng.gen_range(-1.0..=1.0),
        );
        let len_sq = v.length_squared();
        if (1e-6..=1.0).contains(&len_sq) {
            return v / len_sq.sqrt();
        }
    }
}

/// Bake a two-point lerp into a float RGB texture (gray). Godot's CurveTexture
/// bakes to 8-bit and clamps to [0, 1], which can't represent scales above 1.
fn gray_curve_texture(start: f32, end: f32) -> Gd<ImageTexture> {
    const WIDTH: i32 = 8;
    let mut image = Image::create_empty(WIDTH, 1, false, Format::RGBF)
        .expect("failed to allocate scale curve image");
    for x in 0..WIDTH {
        let t = x as f32 / (WIDTH - 1) as f32;
        let value = start + (end - start) * t;
        image.set_pixel(
            x,
            0,
            Color {
                r: value,
                g: value,
                b: value,
                a: 1.0,
            },
        );
    }
    ImageTexture::create_from_image(&image).expect("failed to create scale curve texture")
}

/// 1x1 float texture with a constant red value (hard velocity clamp in m/s).
fn constant_red_texture(value: f32) -> Gd<ImageTexture> {
    let mut image = Image::create_empty(1, 1, false, Format::RF)
        .expect("failed to allocate velocity limit image");
    image.set_pixel(
        0,
        0,
        Color {
            r: value,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        },
    );
    ImageTexture::create_from_image(&image).expect("failed to create velocity limit texture")
}

fn quaternion_z_degrees(x: f32, y: f32, z: f32, w: f32) -> f32 {
    Basis::from_quaternion(dcl_quat_to_godot(x, y, z, w))
        .get_euler()
        .z
        .to_degrees()
}
