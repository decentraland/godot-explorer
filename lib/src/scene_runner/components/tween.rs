use std::time::Duration;

use godot::{
    builtin::Basis,
    classes::{BaseMaterial3D, MeshInstance3D, Node3D},
    prelude::*,
};

use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::{
                pb_tween::Mode, EasingFunction, PbTween, PbTweenState, TextureMovementType,
                TweenStateStatus,
            },
            transform_and_parent::DclTransformAndParent,
            SceneComponentId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, InsertIfNotExists, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_gltf_container::DclGltfContainer,
    scene_runner::scene::{Scene, TextureAnimation},
};

pub struct Tween {
    pub data: PbTween,
    pub ease_fn: fn(f32) -> f32,
    pub start_time: std::time::Instant,
    pub paused_time: Option<std::time::Instant>,
    pub playing: Option<bool>,
    /// Last update time for delta time calculation (used by continuous modes)
    pub last_update: std::time::Instant,
}

impl Tween {
    fn get_progress(&self, elapsed_time: Duration) -> f32 {
        elapsed_time.as_millis() as f32 / self.data.duration // 0 to 1...
    }
}

fn get_ease_fn(ease_type: EasingFunction) -> fn(f32) -> f32 {
    match ease_type {
        EasingFunction::EfLinear => simple_easing::linear,
        EasingFunction::EfEaseinquad => simple_easing::quad_in,
        EasingFunction::EfEaseoutquad => simple_easing::quad_out,
        EasingFunction::EfEasequad => simple_easing::quad_in_out,
        EasingFunction::EfEaseinsine => simple_easing::sine_in,
        EasingFunction::EfEaseoutsine => simple_easing::sine_out,
        EasingFunction::EfEasesine => simple_easing::sine_in_out,
        EasingFunction::EfEaseinexpo => simple_easing::expo_in,
        EasingFunction::EfEaseoutexpo => simple_easing::expo_out,
        EasingFunction::EfEaseexpo => simple_easing::expo_in_out,
        EasingFunction::EfEaseinelastic => simple_easing::elastic_in,
        EasingFunction::EfEaseoutelastic => simple_easing::elastic_out,
        EasingFunction::EfEaseelastic => simple_easing::elastic_in_out,
        EasingFunction::EfEaseinbounce => simple_easing::bounce_in,
        EasingFunction::EfEaseoutbounce => simple_easing::bounce_out,
        EasingFunction::EfEasebounce => simple_easing::bounce_in_out,
        EasingFunction::EfEaseincubic => simple_easing::cubic_in,
        EasingFunction::EfEaseoutcubic => simple_easing::cubic_out,
        EasingFunction::EfEasecubic => simple_easing::cubic_in_out,
        EasingFunction::EfEaseinquart => simple_easing::quart_in,
        EasingFunction::EfEaseoutquart => simple_easing::quart_out,
        EasingFunction::EfEasequart => simple_easing::quart_in_out,
        EasingFunction::EfEaseinquint => simple_easing::quint_in,
        EasingFunction::EfEaseoutquint => simple_easing::quint_out,
        EasingFunction::EfEasequint => simple_easing::quint_in_out,
        EasingFunction::EfEaseincirc => simple_easing::circ_in,
        EasingFunction::EfEaseoutcirc => simple_easing::circ_out,
        EasingFunction::EfEasecirc => simple_easing::circ_in_out,
        EasingFunction::EfEaseinback => simple_easing::back_in,
        EasingFunction::EfEaseoutback => simple_easing::back_out,
        EasingFunction::EfEaseback => simple_easing::back_in_out,
    }
}

/// Apply UV offset and scale to a single MeshInstance3D
fn apply_uv_to_mesh(mesh: &mut Gd<MeshInstance3D>, tex_anim: &TextureAnimation) {
    let surface_count = mesh.get_surface_override_material_count();
    for surface_idx in 0..surface_count {
        // Get the active material (override or mesh resource material)
        if let Some(material) = mesh.get_active_material(surface_idx) {
            if let Ok(mut base_material) = material.try_cast::<BaseMaterial3D>() {
                base_material.set_uv1_offset(Vector3::new(
                    tex_anim.uv_offset.x,
                    tex_anim.uv_offset.y,
                    0.0,
                ));
                base_material.set_uv1_scale(Vector3::new(
                    tex_anim.uv_scale.x,
                    tex_anim.uv_scale.y,
                    1.0,
                ));
            }
        }
    }
}

/// Recursively find all MeshInstance3D nodes in a tree and apply UV animation
fn apply_uv_to_tree(node: &Gd<Node3D>, tex_anim: &TextureAnimation) {
    // Check if this node is a MeshInstance3D
    if let Ok(mut mesh) = node.clone().try_cast::<MeshInstance3D>() {
        apply_uv_to_mesh(&mut mesh, tex_anim);
    }

    // Recursively check children
    for i in 0..node.get_child_count() {
        if let Some(child) = node.get_child(i) {
            if let Ok(child_3d) = child.try_cast::<Node3D>() {
                apply_uv_to_tree(&child_3d, tex_anim);
            }
        }
    }
}

/// Apply UV animation to all meshes of an entity (both DCL primitives and GLTF models)
fn apply_texture_animation_to_entity(
    scene: &Scene,
    entity: &crate::dcl::components::SceneEntityId,
    tex_anim: &TextureAnimation,
) {
    let Some(godot_entity_node) = scene.godot_dcl_scene.get_godot_entity_node(entity) else {
        return;
    };

    let Some(base_3d) = &godot_entity_node.base_3d else {
        return;
    };

    // Try DCL primitive (MeshRenderer child)
    if let Some(mut mesh_renderer) = base_3d.try_get_node_as::<MeshInstance3D>("MeshRenderer") {
        apply_uv_to_mesh(&mut mesh_renderer, tex_anim);
    }

    // Try GLTF container
    if let Some(gltf_container) = base_3d.try_get_node_as::<DclGltfContainer>("GltfContainer") {
        if let Some(gltf_root) = gltf_container.bind().get_gltf_resource() {
            apply_uv_to_tree(&gltf_root, tex_anim);
        }
    }
}

pub fn update_tween(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let tween_component = SceneCrdtStateProtoComponents::get_tween(crdt_state);

    let now = std::time::Instant::now();

    let mut tweens_to_delete = Vec::new();
    let mut texture_animations_to_apply = Vec::new();

    if let Some(tween_dirty) = dirty_lww_components.get(&SceneComponentId::TWEEN) {
        for entity in tween_dirty {
            let new_value = tween_component.get(entity);

            let Some(new_value) = new_value else {
                continue; // no value, continue
            };

            let new_value = new_value.value.clone();

            let existing = scene.tweens.get_mut(entity);

            if new_value.is_none() {
                // tween gets deleted
                if existing.is_some() {
                    tweens_to_delete.push(entity);
                }
            } else if let Some(new_value) = new_value {
                let offset_time_ms = new_value.duration * new_value.current_time();
                let offset_time = std::time::Duration::from_millis(offset_time_ms as u64);

                if let Some(existing_tween) = existing {
                    // update tween

                    if existing_tween.data.playing != new_value.playing
                        && new_value.playing != Some(false)
                    {
                        if let Some(paused_time) = existing_tween.paused_time {
                            // resume tween
                            existing_tween.start_time += now - paused_time;
                            existing_tween.paused_time = None;
                            existing_tween.playing = Some(true);
                        }
                    }

                    // reset tween when the mode changes, or we have a new current time
                    let reset_tween = existing_tween.data.mode != new_value.mode
                        || new_value.current_time.is_some();
                    if reset_tween {
                        existing_tween.start_time = now - offset_time;
                    }

                    // copy new tween values
                    existing_tween.data.current_time = new_value.current_time;
                    existing_tween.data.mode = new_value.mode;
                    existing_tween.data.duration = new_value.duration;
                    existing_tween.data.easing_function = new_value.easing_function;
                    existing_tween.data.playing = new_value.playing;
                } else {
                    // new tween
                    let paused_time = if new_value.playing == Some(false) {
                        Some(now)
                    } else {
                        None
                    };

                    scene.tweens.insert(
                        *entity,
                        Tween {
                            ease_fn: get_ease_fn(
                                EasingFunction::from_i32(new_value.easing_function).unwrap(),
                            ),
                            data: new_value,
                            start_time: now - offset_time,
                            paused_time,
                            playing: None,
                            last_update: now,
                        },
                    );
                };
            }
        }
    }

    for entity in tweens_to_delete {
        scene.tweens.remove(entity);
        // Also clean up any texture animation state
        scene.texture_animations.remove(entity);

        // update tween state
        SceneCrdtStateProtoComponents::get_tween_state_mut(crdt_state).put(*entity, None);
    }

    for (entity, tween) in &mut scene.tweens {
        if tween.playing == Some(false) {
            continue;
        }

        // Check if this is a continuous mode
        let is_continuous = matches!(
            &tween.data.mode,
            Some(Mode::RotateContinuous(_))
                | Some(Mode::MoveContinuous(_))
                | Some(Mode::TextureMoveContinuous(_))
        );

        let mut current_tween_state: TweenStateStatus = TweenStateStatus::TsActive;
        let delta_time = now.duration_since(tween.last_update).as_secs_f32();

        // For standard tweens, calculate progress based on duration
        let progress = if is_continuous {
            // Continuous tweens don't use progress in the traditional sense
            // We use delta_time instead for incremental updates
            0.0 // Progress not meaningful for continuous modes
        } else {
            let elapsed_time = now - tween.start_time;
            let duration = std::time::Duration::from_millis(tween.data.duration as u64);

            if elapsed_time >= duration {
                tween.playing = Some(false);
                current_tween_state = TweenStateStatus::TsCompleted;
                1.0 // finished
            } else {
                tween.get_progress(elapsed_time)
            }
        };

        tween.playing = tween.data.playing;
        if tween.playing == Some(false) {
            // get paused
            current_tween_state = TweenStateStatus::TsPaused;
            tween.paused_time = Some(now);
        }

        // update tween state
        SceneCrdtStateProtoComponents::get_tween_state_mut(crdt_state).put(
            *entity,
            Some(PbTweenState {
                current_time: progress,
                state: current_tween_state as i32,
            }),
        );

        // if we paused the tween, we skip the transform update
        if tween.playing == Some(false) {
            continue;
        }

        // Update last_update for next frame's delta time calculation
        tween.last_update = now;

        // get entity transform from crdt state
        let mut transform: DclTransformAndParent = crdt_state
            .get_transform_mut()
            .get(entity)
            .and_then(|transform| transform.value.clone())
            .unwrap_or_default();

        // calculate new transform with the tween
        let ease_value = (tween.ease_fn)(progress);
        let new_transform = match &tween.data.mode {
            Some(Mode::Move(data)) => {
                let start = data.start.clone().unwrap().to_godot();
                let end = data.end.clone().unwrap().to_godot();

                if data.face_direction == Some(true) {
                    let direction = (end - start).normalized();
                    let basis = if direction.is_zero_approx() {
                        Basis::IDENTITY
                    } else {
                        let v_x = godot::builtin::Vector3::UP.cross(direction);
                        let v_x = if v_x.is_zero_approx() {
                            // same workaround as bevy-explorer
                            // when the direction is colinear to the up vector, we use the forward vector as up+
                            godot::builtin::Vector3::FORWARD.cross(direction)
                        } else {
                            v_x
                        }
                        .normalized();
                        let v_y = direction.cross(v_x);

                        let mut basis = Basis::IDENTITY;
                        basis.set_col_a(v_x);
                        basis.set_col_b(v_y);
                        basis.set_col_c(direction);
                        basis
                    };

                    transform.rotation = basis.get_quaternion();
                }

                transform.translation = start + ((end - start) * ease_value);
                transform
            }
            Some(Mode::Rotate(data)) => {
                let start = data.start.clone().unwrap().to_godot();
                let end = data.end.clone().unwrap().to_godot();
                transform.rotation = start + ((end - start) * ease_value);
                transform
            }
            Some(Mode::Scale(data)) => {
                let start = data.start.clone().unwrap().to_godot();
                let end = data.end.clone().unwrap().to_godot();
                transform.scale = start + ((end - start) * ease_value);
                transform
            }
            Some(Mode::MoveContinuous(data)) => {
                // MoveContinuous: Apply incremental position change based on direction and speed
                let direction = data
                    .direction
                    .clone()
                    .map(|d| d.to_godot())
                    .unwrap_or(godot::builtin::Vector3::ZERO);
                let speed = data.speed;
                let delta = direction * speed * delta_time;
                transform.translation += delta;
                transform
            }
            Some(Mode::RotateContinuous(data)) => {
                // RotateContinuous: direction is an orientation quaternion
                // Speed is in DEGREES per second
                let raw_direction = data.direction.clone().map(|d| d.to_godot());
                let speed = data.speed;

                // Derive rotation axis by rotating Vector3::UP through the quaternion
                // This matches Unity's: axis = (direction * Vector3.up).normalized
                // The axis stays in DCL coordinate space (conversion happens in to_godot_transform_3d)
                let axis = raw_direction
                    .map(|q| q * godot::builtin::Vector3::UP)
                    .unwrap_or(godot::builtin::Vector3::ZERO);
                let axis_length = axis.length();

                if axis_length > 0.0001 {
                    let axis_normalized = axis / axis_length;

                    // Speed is in degrees per second, convert to radians
                    let angle_radians = speed.to_radians() * delta_time;

                    // Create rotation quaternion for this frame's rotation step
                    let half_angle = angle_radians / 2.0;
                    let sin_half = half_angle.sin();
                    let cos_half = half_angle.cos();
                    let rotation_step = godot::builtin::Quaternion::new(
                        axis_normalized.x * sin_half,
                        axis_normalized.y * sin_half,
                        axis_normalized.z * sin_half,
                        cos_half,
                    );

                    // Apply the rotation step to the current rotation
                    let current = transform.rotation.normalized();
                    transform.rotation = (current * rotation_step).normalized();

                    tracing::debug!(
                        "[Tween] RotateContinuous: entity={:?}, axis={:?}, speed={} deg/s, angle_rad={:.6}, result={:?}",
                        entity, axis_normalized, speed, angle_radians, transform.rotation
                    );
                }

                transform
            }
            Some(Mode::TextureMove(data)) => {
                // TextureMove: Interpolate UV offset/tiling between start and end
                let start = data
                    .start
                    .clone()
                    .map(|v| v.to_godot())
                    .unwrap_or(godot::builtin::Vector2::ZERO);
                let end = data
                    .end
                    .clone()
                    .map(|v| v.to_godot())
                    .unwrap_or(godot::builtin::Vector2::ZERO);
                let movement_type = TextureMovementType::from_i32(data.movement_type.unwrap_or(0))
                    .unwrap_or(TextureMovementType::TmtOffset);

                let value = start + ((end - start) * ease_value);

                // Get or create texture animation state
                let tex_anim = scene.texture_animations.entry(*entity).or_default();

                match movement_type {
                    TextureMovementType::TmtOffset => tex_anim.uv_offset = value,
                    TextureMovementType::TmtTiling => tex_anim.uv_scale = value,
                }

                // Queue UV animation to apply after the loop (to avoid borrow issues)
                texture_animations_to_apply.push((*entity, tex_anim.clone()));
                continue;
            }
            Some(Mode::TextureMoveContinuous(data)) => {
                // TextureMoveContinuous: Apply incremental UV change based on direction and speed
                let direction = data
                    .direction
                    .clone()
                    .map(|v| v.to_godot())
                    .unwrap_or(godot::builtin::Vector2::ZERO);
                let speed = data.speed;
                let movement_type = TextureMovementType::from_i32(data.movement_type.unwrap_or(0))
                    .unwrap_or(TextureMovementType::TmtOffset);

                let delta = direction * speed * delta_time;

                // Get or create texture animation state
                let tex_anim = scene.texture_animations.entry(*entity).or_default();

                match movement_type {
                    TextureMovementType::TmtOffset => tex_anim.uv_offset += delta,
                    TextureMovementType::TmtTiling => tex_anim.uv_scale += delta,
                }

                // Queue UV animation to apply after the loop (to avoid borrow issues)
                texture_animations_to_apply.push((*entity, tex_anim.clone()));
                continue;
            }
            _ => {
                continue;
            }
        };

        // set new transform to the entity
        crdt_state
            .get_transform_mut()
            .put(*entity, Some(new_transform));

        // set the component as dirty for further processing
        if let Some(dirty) = scene
            .current_dirty
            .lww_components
            .get_mut(&SceneComponentId::TRANSFORM)
        {
            dirty.insert_if_not_exists(*entity);
        } else {
            scene
                .current_dirty
                .lww_components
                .insert(SceneComponentId::TRANSFORM, vec![*entity]);
        }
    }

    // Apply texture animations to meshes (after the main loop to avoid borrow issues)
    for (entity, tex_anim) in texture_animations_to_apply {
        apply_texture_animation_to_entity(scene, &entity, &tex_anim);
    }
}
