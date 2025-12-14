use std::collections::{HashMap, HashSet};

use godot::{
    engine::{PhysicsDirectSpaceState3D, PhysicsServer3D, PhysicsShapeQueryParameters3D},
    prelude::*,
};

use crate::{
    dcl::{
        components::{
            proto_components::{
                self,
                sdk::components::{
                    pb_trigger_area_result::Trigger, PbTriggerArea, PbTriggerAreaResult,
                    TriggerAreaEventType, TriggerAreaMeshType,
                },
            },
            SceneComponentId, SceneEntityId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        SceneId,
    },
    scene_runner::{object_pool::PhysicsAreaPool, scene::Scene},
};

const CL_PLAYER: u32 = 4;

// Cached StringNames to avoid per-frame allocations
thread_local! {
    static META_DCL_ENTITY_ID: StringName = StringName::from("dcl_entity_id");
    static META_DCL_SCENE_ID: StringName = StringName::from("dcl_scene_id");
    static METHOD_HAS_META: StringName = StringName::from("has_meta");
    static METHOD_GET_META: StringName = StringName::from("get_meta");
    static METHOD_GET_COLLISION_LAYER: StringName = StringName::from("get_collision_layer");
}

/// State for a single trigger area instance
#[derive(Debug)]
pub struct TriggerAreaInstance {
    pub area_rid: Rid,
    pub shape_rid: Rid,
    /// Set of entities currently inside this trigger area (including player)
    pub entities_inside: HashSet<SceneEntityId>,
    /// Scratch buffer reused each frame to avoid allocations
    entities_scratch: HashSet<SceneEntityId>,
    pub mesh_type: TriggerAreaMeshType,
    pub collision_mask: u32,
}

/// Global trigger area state for a scene
pub struct TriggerAreaState {
    pub instances: HashMap<SceneEntityId, TriggerAreaInstance>,
    /// Pooled query parameters to avoid per-frame allocations
    query_params: Option<Gd<PhysicsShapeQueryParameters3D>>,
    /// Pooled exclude array to avoid per-frame allocations
    exclude_array: Array<Rid>,
}

impl Default for TriggerAreaState {
    fn default() -> Self {
        Self {
            instances: HashMap::new(),
            query_params: None,
            exclude_array: Array::new(),
        }
    }
}

impl std::fmt::Debug for TriggerAreaState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TriggerAreaState")
            .field("instances", &self.instances)
            .field("query_params", &self.query_params.is_some())
            .finish()
    }
}

impl TriggerAreaState {
    /// Cleanup all trigger areas, releasing RIDs back to the global pool
    pub fn cleanup(&mut self, pool: &mut PhysicsAreaPool) {
        // Release all instances back to pool
        for (_, instance) in self.instances.drain() {
            pool.release_area(instance.area_rid);
            match instance.mesh_type {
                TriggerAreaMeshType::TamtBox => {
                    pool.release_box_shape(instance.shape_rid);
                }
                TriggerAreaMeshType::TamtSphere => {
                    pool.release_sphere_shape(instance.shape_rid);
                }
            }
        }
        self.query_params = None;
        self.exclude_array.clear();
    }

    /// Cleanup without pool (frees RIDs directly) - used when scene is destroyed
    pub fn cleanup_without_pool(&mut self) {
        let mut physics_server = PhysicsServer3D::singleton();
        for (_, instance) in self.instances.drain() {
            physics_server.free_rid(instance.area_rid);
            physics_server.free_rid(instance.shape_rid);
        }
        self.query_params = None;
        self.exclude_array.clear();
    }

    /// Get or create pooled query parameters
    fn get_query_params(&mut self) -> &mut Gd<PhysicsShapeQueryParameters3D> {
        if self.query_params.is_none() {
            self.query_params = Some(PhysicsShapeQueryParameters3D::new_gd());
        }
        self.query_params.as_mut().unwrap()
    }
}

/// Called during scene update (with throttling) - handles component creation/deletion
pub fn update_trigger_area(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    pool: &mut PhysicsAreaPool,
) {
    let trigger_area_component = SceneCrdtStateProtoComponents::get_trigger_area(crdt_state);

    // Process dirty TriggerArea components
    let entities_to_process: Vec<_> = scene
        .current_dirty
        .lww_components
        .get(&SceneComponentId::TRIGGER_AREA)
        .map(|dirty| {
            dirty
                .iter()
                .map(|entity| {
                    let config = trigger_area_component
                        .get(entity)
                        .and_then(|e| e.value.clone());
                    (*entity, config)
                })
                .collect()
        })
        .unwrap_or_default();

    for (entity, config) in entities_to_process {
        match config {
            Some(config) => {
                create_or_update_trigger_area(scene, &entity, &config, pool);
            }
            None => {
                remove_trigger_area(scene, &entity, pool);
            }
        }
    }

    // Update transforms for all trigger areas
    update_trigger_area_transforms(scene);
}

/// Called every physics frame (without throttling) - checks collisions using PhysicsDirectSpaceState3D
/// Results are stored in scene.trigger_area_results and will be appended to CRDT during ComputeCrdtState
pub fn physics_update_trigger_area(
    scene: &mut Scene,
    player_global_transform: &Transform3D,
    space_state: &mut Gd<PhysicsDirectSpaceState3D>,
) {
    if scene.trigger_areas.instances.is_empty() {
        return;
    }

    let tick_number = scene.tick_number;
    let scene_id = scene.scene_id;
    let scene_pos = scene.godot_dcl_scene.root_node_3d.get_global_position();

    // Check if any trigger area needs entity detection (not just player)
    // This is an optimization to skip collecting all entity transforms when only checking for player
    let needs_entity_detection = scene
        .trigger_areas
        .instances
        .values()
        .any(|instance| (instance.collision_mask & !CL_PLAYER) != 0);

    // Collect transforms for entities with colliders ONLY if needed
    // This avoids iterating all entities when only checking for player
    let collider_entity_transforms: Option<HashMap<SceneEntityId, Transform3D>> =
        if needs_entity_detection {
            Some(
                scene
                    .godot_dcl_scene
                    .entities
                    .iter()
                    .filter_map(|(entity_id, godot_entity)| {
                        godot_entity
                            .base_3d
                            .as_ref()
                            .map(|n| (*entity_id, n.get_global_transform()))
                    })
                    .collect(),
            )
        } else {
            None
        };

    // Collect trigger entity IDs and their transforms first (to avoid borrow conflicts)
    let trigger_data: Vec<_> = scene
        .trigger_areas
        .instances
        .keys()
        .filter_map(|entity| {
            scene
                .godot_dcl_scene
                .get_node_or_null_3d(entity)
                .map(|n| (*entity, n.get_global_transform()))
        })
        .collect();

    // Get pooled resources from trigger_areas state
    let query_params = scene.trigger_areas.get_query_params().clone();
    let exclude_array = &mut scene.trigger_areas.exclude_array;

    for (trigger_entity, trigger_transform) in &trigger_data {
        let Some(instance) = scene.trigger_areas.instances.get_mut(trigger_entity) else {
            continue;
        };

        // Reuse scratch buffer: clear and populate with current overlapping entities
        instance.entities_scratch.clear();
        get_overlapping_entities_into(
            space_state,
            &query_params,
            exclude_array,
            instance.shape_rid,
            instance.area_rid,
            *trigger_transform,
            instance.collision_mask,
            scene_id,
            &mut instance.entities_scratch,
        );

        // Process events by comparing previous vs current state
        // Iterate over previous entities to find EXIT events
        for prev_entity in &instance.entities_inside {
            let is_inside = instance.entities_scratch.contains(prev_entity);
            let event_type = if is_inside {
                TriggerAreaEventType::TaetStay
            } else {
                TriggerAreaEventType::TaetExit
            };

            let collider_transform = get_collider_transform(
                *prev_entity,
                player_global_transform,
                &collider_entity_transforms,
            );
            let collider_layers = get_collider_layers(*prev_entity, instance.collision_mask);

            // Only log ENTER/EXIT events, not STAY (reduces log spam significantly)
            if event_type != TriggerAreaEventType::TaetStay {
                tracing::debug!(
                    "[TriggerArea] EVENT: trigger={:?}, collider={:?}, event_type={:?}",
                    trigger_entity,
                    prev_entity,
                    event_type,
                );
            }

            let result = build_trigger_result(
                trigger_entity,
                prev_entity,
                event_type,
                tick_number,
                collider_transform,
                *trigger_transform,
                scene_pos,
                collider_layers,
            );
            scene.trigger_area_results.push((*trigger_entity, result));
        }

        // Iterate over current entities to find ENTER events (entities not in previous)
        for curr_entity in &instance.entities_scratch {
            if !instance.entities_inside.contains(curr_entity) {
                let collider_transform = get_collider_transform(
                    *curr_entity,
                    player_global_transform,
                    &collider_entity_transforms,
                );
                let collider_layers = get_collider_layers(*curr_entity, instance.collision_mask);

                tracing::debug!(
                    "[TriggerArea] EVENT: trigger={:?}, collider={:?}, event_type={:?}",
                    trigger_entity,
                    curr_entity,
                    TriggerAreaEventType::TaetEnter,
                );

                let result = build_trigger_result(
                    trigger_entity,
                    curr_entity,
                    TriggerAreaEventType::TaetEnter,
                    tick_number,
                    collider_transform,
                    *trigger_transform,
                    scene_pos,
                    collider_layers,
                );
                scene.trigger_area_results.push((*trigger_entity, result));
            }
        }

        // Swap scratch into entities_inside (reuses both HashSet allocations)
        std::mem::swap(&mut instance.entities_inside, &mut instance.entities_scratch);
    }
}

#[inline]
fn get_collider_transform(
    entity: SceneEntityId,
    player_transform: &Transform3D,
    entity_transforms: &Option<HashMap<SceneEntityId, Transform3D>>,
) -> Transform3D {
    if entity == SceneEntityId::PLAYER {
        *player_transform
    } else {
        entity_transforms
            .as_ref()
            .and_then(|m| m.get(&entity))
            .copied()
            .unwrap_or(Transform3D::IDENTITY)
    }
}

#[inline]
fn get_collider_layers(entity: SceneEntityId, collision_mask: u32) -> u32 {
    if entity == SceneEntityId::PLAYER {
        CL_PLAYER
    } else {
        collision_mask & !CL_PLAYER
    }
}

fn create_or_update_trigger_area(
    scene: &mut Scene,
    entity: &SceneEntityId,
    config: &PbTriggerArea,
    pool: &mut PhysicsAreaPool,
) {
    let mut physics_server = PhysicsServer3D::singleton();
    let mesh_type = config.mesh();
    let collision_mask = config.collision_mask.unwrap_or(CL_PLAYER);

    // Check if mesh type changed (requires recreate)
    let needs_recreate = scene
        .trigger_areas
        .instances
        .get(entity)
        .map(|i| i.mesh_type != mesh_type)
        .unwrap_or(true);

    if needs_recreate {
        remove_trigger_area(scene, entity, pool);

        // Get physics space
        let space_rid = scene
            .godot_dcl_scene
            .root_node_3d
            .get_world_3d()
            .map(|w| w.get_space())
            .unwrap_or(Rid::Invalid);

        // Acquire area from pool (reuses existing RID if available)
        let area_rid = pool.acquire_area();
        physics_server.area_set_space(area_rid, space_rid);

        // Acquire shape from pool based on type
        let shape_rid = match mesh_type {
            TriggerAreaMeshType::TamtBox => {
                let rid = pool.acquire_box_shape();
                physics_server.shape_set_data(rid, Vector3::new(0.5, 0.5, 0.5).to_variant());
                rid
            }
            TriggerAreaMeshType::TamtSphere => {
                let rid = pool.acquire_sphere_shape();
                physics_server.shape_set_data(rid, (0.5_f32).to_variant());
                rid
            }
        };

        // Attach shape to area
        physics_server.area_add_shape(area_rid, shape_rid);

        // Configure collision layer/mask
        // Layer = 0: trigger areas don't need to be detected by others
        // Mask = collision_mask: configured in scene component (default CL_PLAYER=4)
        // Note: These settings are for Godot's internal area system, but we use intersect_shape for detection
        physics_server.area_set_collision_layer(area_rid, 0);
        physics_server.area_set_collision_mask(area_rid, collision_mask);
        physics_server.area_set_monitorable(area_rid, false);

        tracing::debug!(
            "[TriggerArea] Created area for entity {:?}: area_rid={:?}, shape_rid={:?}, collision_mask={}",
            entity,
            area_rid,
            shape_rid,
            collision_mask
        );

        scene.trigger_areas.instances.insert(
            *entity,
            TriggerAreaInstance {
                area_rid,
                shape_rid,
                entities_inside: HashSet::new(),
                entities_scratch: HashSet::new(),
                mesh_type,
                collision_mask,
            },
        );
    } else if let Some(instance) = scene.trigger_areas.instances.get_mut(entity) {
        // Update collision mask only (stored for reference)
        if instance.collision_mask != collision_mask {
            instance.collision_mask = collision_mask;
        }
    }
}

fn remove_trigger_area(scene: &mut Scene, entity: &SceneEntityId, pool: &mut PhysicsAreaPool) {
    if let Some(instance) = scene.trigger_areas.instances.remove(entity) {
        // Release back to pool for reuse
        pool.release_area(instance.area_rid);
        match instance.mesh_type {
            TriggerAreaMeshType::TamtBox => {
                pool.release_box_shape(instance.shape_rid);
            }
            TriggerAreaMeshType::TamtSphere => {
                pool.release_sphere_shape(instance.shape_rid);
            }
        }
    }
}

fn update_trigger_area_transforms(scene: &mut Scene) {
    let mut physics_server = PhysicsServer3D::singleton();

    for (entity, instance) in scene.trigger_areas.instances.iter() {
        // Get global transform from Godot node
        let Some(node_3d) = scene.godot_dcl_scene.get_node_or_null_3d(entity) else {
            continue;
        };

        let global_transform = node_3d.get_global_transform();

        // The area transform already includes scale in the basis, which Godot applies to shapes.
        // We keep shape data at base size (0.5 half-extents for box, 0.5 radius for sphere)
        // and let the transform handle all scaling to avoid double-scaling.
        physics_server.area_set_transform(instance.area_rid, global_transform);
    }
}

/// Get all entities overlapping with the trigger area shape
/// Populates the output HashSet (should be cleared before calling)
/// Uses pooled query params and exclude array to minimize allocations
#[allow(clippy::too_many_arguments)]
fn get_overlapping_entities_into(
    space_state: &mut Gd<PhysicsDirectSpaceState3D>,
    query_params: &Gd<PhysicsShapeQueryParameters3D>,
    exclude_array: &mut Array<Rid>,
    shape_rid: Rid,
    area_rid: Rid,
    shape_transform: Transform3D,
    collision_mask: u32,
    scene_id: SceneId,
    output: &mut HashSet<SceneEntityId>,
) {
    // Configure pooled query parameters (reused across frames)
    let mut query = query_params.clone();
    query.set_shape_rid(shape_rid);
    query.set_transform(shape_transform);
    query.set_collision_mask(collision_mask);
    query.set_collide_with_areas(true);
    query.set_collide_with_bodies(true);

    // Reuse exclude array
    exclude_array.clear();
    exclude_array.push(area_rid);
    query.set_exclude(exclude_array.clone());

    // Query for overlapping shapes
    let results = space_state.intersect_shape(query);

    // Process results using cached StringNames
    METHOD_HAS_META.with(|has_meta| {
        METHOD_GET_META.with(|get_meta| {
            META_DCL_ENTITY_ID.with(|entity_id_key| {
                META_DCL_SCENE_ID.with(|scene_id_key| {
                    METHOD_GET_COLLISION_LAYER.with(|get_collision_layer| {
                        let entity_id_variant = Variant::from(entity_id_key.clone());
                        let scene_id_variant = Variant::from(scene_id_key.clone());

                        for i in 0..results.len() {
                            let Some(result_dict) = results.get(i) else {
                                continue;
                            };
                            let Some(collider) = result_dict.get("collider") else {
                                continue;
                            };
                            if collider.is_nil() {
                                continue;
                            }

                            // Check if this is a DCL entity
                            let has_dcl_entity_id = collider
                                .call(has_meta.clone(), &[entity_id_variant.clone()])
                                .to::<bool>();

                            if has_dcl_entity_id {
                                let dcl_entity_id = collider
                                    .call(get_meta.clone(), &[entity_id_variant.clone()])
                                    .to::<i32>();
                                let dcl_scene_id = collider
                                    .call(get_meta.clone(), &[scene_id_variant.clone()])
                                    .to::<i32>();

                                if dcl_scene_id == scene_id.0 {
                                    output.insert(SceneEntityId::from_i32(dcl_entity_id));
                                }
                            } else {
                                // Check if this is the player's area detector
                                let collider_layer = collider
                                    .call(get_collision_layer.clone(), &[])
                                    .to::<u32>();

                                if (collider_layer & CL_PLAYER) != 0
                                    && (collision_mask & CL_PLAYER) != 0
                                {
                                    output.insert(SceneEntityId::PLAYER);
                                }
                            }
                        }
                    });
                });
            });
        });
    });
}

fn build_trigger_result(
    triggered_entity: &SceneEntityId,
    trigger_entity: &SceneEntityId,
    event_type: TriggerAreaEventType,
    timestamp: u32,
    trigger_transform: Transform3D,
    triggered_transform: Transform3D,
    scene_pos: Vector3,
    trigger_layers: u32,
) -> PbTriggerAreaResult {
    let triggered_pos = triggered_transform.origin - scene_pos;
    let triggered_rot = triggered_transform.basis.to_quat();

    let trigger_pos = trigger_transform.origin - scene_pos;
    let trigger_rot = trigger_transform.basis.to_quat();

    PbTriggerAreaResult {
        triggered_entity: triggered_entity.as_i32() as u32,
        triggered_entity_position: Some(proto_components::common::Vector3 {
            x: triggered_pos.x,
            y: triggered_pos.y,
            z: -triggered_pos.z,
        }),
        triggered_entity_rotation: Some(proto_components::common::Quaternion {
            x: triggered_rot.x,
            y: triggered_rot.y,
            z: -triggered_rot.z,
            w: -triggered_rot.w,
        }),
        event_type: event_type as i32,
        timestamp,
        trigger: Some(Trigger {
            entity: trigger_entity.as_i32() as u32,
            layers: trigger_layers,
            position: Some(proto_components::common::Vector3 {
                x: trigger_pos.x,
                y: trigger_pos.y,
                z: -trigger_pos.z,
            }),
            rotation: Some(proto_components::common::Quaternion {
                x: trigger_rot.x,
                y: trigger_rot.y,
                z: -trigger_rot.z,
                w: -trigger_rot.w,
            }),
            scale: Some(proto_components::common::Vector3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            }),
        }),
    }
}
