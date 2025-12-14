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
    scene_runner::scene::Scene,
};

const CL_PLAYER: u32 = 4;

/// State for a single trigger area instance
#[derive(Debug)]
pub struct TriggerAreaInstance {
    pub area_rid: Rid,
    pub shape_rid: Rid,
    /// Set of entities currently inside this trigger area (including player)
    pub entities_inside: HashSet<SceneEntityId>,
    pub mesh_type: TriggerAreaMeshType,
    pub collision_mask: u32,
}

/// Global trigger area state for a scene
#[derive(Debug, Default)]
pub struct TriggerAreaState {
    pub instances: HashMap<SceneEntityId, TriggerAreaInstance>,
}

impl TriggerAreaState {
    pub fn cleanup(&mut self) {
        let mut physics_server = PhysicsServer3D::singleton();
        for (_, instance) in self.instances.drain() {
            physics_server.free_rid(instance.area_rid);
            physics_server.free_rid(instance.shape_rid);
        }
    }
}

/// Called during scene update (with throttling) - handles component creation/deletion
pub fn update_trigger_area(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
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
                create_or_update_trigger_area(scene, &entity, &config);
            }
            None => {
                remove_trigger_area(scene, &entity);
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

    // Collect entity transforms for trigger areas
    let entity_transforms: HashMap<SceneEntityId, Transform3D> = scene
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

    // Collect transforms for all entities with colliders (for building trigger results)
    let collider_entity_transforms: HashMap<SceneEntityId, Transform3D> = scene
        .godot_dcl_scene
        .entities
        .iter()
        .filter_map(|(entity_id, godot_entity)| {
            godot_entity
                .base_3d
                .as_ref()
                .map(|n| (*entity_id, n.get_global_transform()))
        })
        .collect();

    for (trigger_entity, instance) in scene.trigger_areas.instances.iter_mut() {
        let Some(trigger_transform) = entity_transforms.get(trigger_entity) else {
            continue;
        };

        // Get all entities currently overlapping with this trigger area
        let current_entities = get_overlapping_entities(
            space_state,
            instance.shape_rid,
            instance.area_rid,
            *trigger_transform,
            instance.collision_mask,
            scene_id,
        );

        let previous_entities = &instance.entities_inside;

        // Process each entity that is currently inside or was previously inside
        let all_entities: HashSet<_> = current_entities
            .iter()
            .chain(previous_entities.iter())
            .cloned()
            .collect();

        for collider_entity in all_entities {
            let was_inside = previous_entities.contains(&collider_entity);
            let is_inside = current_entities.contains(&collider_entity);

            let event_type = match (was_inside, is_inside) {
                (false, true) => Some(TriggerAreaEventType::TaetEnter),
                (true, true) => Some(TriggerAreaEventType::TaetStay),
                (true, false) => Some(TriggerAreaEventType::TaetExit),
                (false, false) => None,
            };

            if let Some(event_type) = event_type {
                // Get the transform of the colliding entity
                let collider_transform = if collider_entity == SceneEntityId::PLAYER {
                    *player_global_transform
                } else {
                    collider_entity_transforms
                        .get(&collider_entity)
                        .copied()
                        .unwrap_or(Transform3D::IDENTITY)
                };

                // Determine which collision layer the collider is on
                let collider_layers = if collider_entity == SceneEntityId::PLAYER {
                    CL_PLAYER
                } else {
                    // MeshColliders typically use CL_POINTER | CL_PHYSICS (3)
                    instance.collision_mask & !CL_PLAYER
                };

                tracing::info!(
                    "[TriggerArea] EVENT: trigger={:?}, collider={:?}, event_type={:?}",
                    trigger_entity,
                    collider_entity,
                    event_type,
                );

                let result = build_trigger_result(
                    trigger_entity,
                    &collider_entity,
                    event_type,
                    tick_number,
                    collider_transform,
                    *trigger_transform,
                    scene_pos,
                    collider_layers,
                );
                // Store in scene struct - will be appended to CRDT during ComputeCrdtState
                scene.trigger_area_results.push((*trigger_entity, result));
            }
        }

        // Update the entities_inside set
        instance.entities_inside = current_entities;
    }
}

fn create_or_update_trigger_area(scene: &mut Scene, entity: &SceneEntityId, config: &PbTriggerArea) {
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
        remove_trigger_area(scene, entity);

        // Get physics space
        let space_rid = scene
            .godot_dcl_scene
            .root_node_3d
            .get_world_3d()
            .map(|w| w.get_space())
            .unwrap_or(Rid::Invalid);

        // Create area via PhysicsServer3D
        let area_rid = physics_server.area_create();
        physics_server.area_set_space(area_rid, space_rid);

        // Create shape based on type
        let shape_rid = match mesh_type {
            TriggerAreaMeshType::TamtBox => {
                let rid = physics_server.box_shape_create();
                physics_server.shape_set_data(rid, Vector3::new(0.5, 0.5, 0.5).to_variant());
                rid
            }
            TriggerAreaMeshType::TamtSphere => {
                let rid = physics_server.sphere_shape_create();
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

        tracing::info!(
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

fn remove_trigger_area(scene: &mut Scene, entity: &SceneEntityId) {
    if let Some(instance) = scene.trigger_areas.instances.remove(entity) {
        let mut physics_server = PhysicsServer3D::singleton();
        physics_server.free_rid(instance.area_rid);
        physics_server.free_rid(instance.shape_rid);
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
/// Returns a HashSet of SceneEntityIds (including PLAYER if the player is inside)
fn get_overlapping_entities(
    space_state: &mut Gd<PhysicsDirectSpaceState3D>,
    shape_rid: Rid,
    area_rid: Rid,
    shape_transform: Transform3D,
    collision_mask: u32,
    scene_id: SceneId,
) -> HashSet<SceneEntityId> {
    let mut entities = HashSet::new();

    // Create query parameters
    let mut query = PhysicsShapeQueryParameters3D::new_gd();
    query.set_shape_rid(shape_rid);
    query.set_transform(shape_transform);
    query.set_collision_mask(collision_mask);
    // Detect both areas (player's camera_mode_area_detector) and bodies (MeshColliders)
    query.set_collide_with_areas(true);
    query.set_collide_with_bodies(true);

    // Exclude self from the query
    let mut exclude = godot::prelude::Array::new();
    exclude.push(area_rid);
    query.set_exclude(exclude);

    // Query for overlapping shapes
    let results = space_state.intersect_shape(query);

    // Process results to extract entity IDs
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

        // Check if this is a DCL entity by looking for metadata
        let has_dcl_entity_id = collider
            .call(
                StringName::from("has_meta"),
                &[Variant::from("dcl_entity_id")],
            )
            .to::<bool>();

        if has_dcl_entity_id {
            let dcl_entity_id = collider
                .call(
                    StringName::from("get_meta"),
                    &[Variant::from("dcl_entity_id")],
                )
                .to::<i32>();
            let dcl_scene_id = collider
                .call(
                    StringName::from("get_meta"),
                    &[Variant::from("dcl_scene_id")],
                )
                .to::<i32>();

            // Only include entities from the same scene
            if dcl_scene_id == scene_id.0 {
                entities.insert(SceneEntityId::from_i32(dcl_entity_id));
            }
        } else {
            // Check if this is the player's area detector (camera_mode_area_detector)
            // The player area detector has collision_layer that includes CL_PLAYER (bit 2)
            let collider_layer = collider
                .call(
                    StringName::from("get_collision_layer"),
                    &[],
                )
                .to::<u32>();

            // If the collider is on the CL_PLAYER layer and we're looking for CL_PLAYER
            if (collider_layer & CL_PLAYER) != 0 && (collision_mask & CL_PLAYER) != 0 {
                entities.insert(SceneEntityId::PLAYER);
            }
        }
    }

    entities
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
