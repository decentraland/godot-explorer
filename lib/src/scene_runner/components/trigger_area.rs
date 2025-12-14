use std::collections::HashMap;

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
    },
    scene_runner::scene::Scene,
};

const CL_PLAYER: u32 = 4;

// Collision layer for trigger areas - matches player's camera_mode_area_detector (bit 31)
const TRIGGER_COLLISION_LAYER: u32 = 2147483648;

/// State for a single trigger area instance
#[derive(Debug)]
pub struct TriggerAreaInstance {
    pub area_rid: Rid,
    pub shape_rid: Rid,
    pub player_inside: bool,
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
    let scene_pos = scene.godot_dcl_scene.root_node_3d.get_global_position();

    // Collect entity transforms
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

    for (entity, instance) in scene.trigger_areas.instances.iter_mut() {
        let Some(entity_transform) = entity_transforms.get(entity) else {
            continue;
        };

        // Check if player overlaps with this trigger area using intersect_shape
        let is_inside = check_player_overlaps_area(
            space_state,
            instance.shape_rid,
            instance.area_rid,
            *entity_transform,
        );

        let was_inside = instance.player_inside;

        let event_type = match (was_inside, is_inside) {
            (false, true) => Some(TriggerAreaEventType::TaetEnter),
            (true, true) => Some(TriggerAreaEventType::TaetStay),
            (true, false) => Some(TriggerAreaEventType::TaetExit),
            (false, false) => None,
        };

        // Update state
        instance.player_inside = is_inside;

        if let Some(event_type) = event_type {
            tracing::info!(
                "[TriggerArea] EVENT: entity={:?}, event_type={:?}, was_inside={}, is_inside={}",
                entity,
                event_type,
                was_inside,
                is_inside
            );
            let result = build_trigger_result(
                entity,
                &SceneEntityId::PLAYER,
                event_type,
                tick_number,
                *player_global_transform,
                *entity_transform,
                scene_pos,
            );
            // Store in scene struct - will be appended to CRDT during ComputeCrdtState
            scene.trigger_area_results.push((*entity, result));
        }
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
        // Mask = TRIGGER_COLLISION_LAYER: detect player's camera_mode_area_detector
        physics_server.area_set_collision_layer(area_rid, 0);
        physics_server.area_set_collision_mask(area_rid, TRIGGER_COLLISION_LAYER);
        physics_server.area_set_monitorable(area_rid, false);

        tracing::info!(
            "[TriggerArea] Created area for entity {:?}: area_rid={:?}, shape_rid={:?}, layer={}, mask={}",
            entity,
            area_rid,
            shape_rid,
            TRIGGER_COLLISION_LAYER,
            TRIGGER_COLLISION_LAYER
        );

        scene.trigger_areas.instances.insert(
            *entity,
            TriggerAreaInstance {
                area_rid,
                shape_rid,
                player_inside: false,
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

/// Check if the player (via camera_mode_area_detector layer) overlaps with the trigger area shape
fn check_player_overlaps_area(
    space_state: &mut Gd<PhysicsDirectSpaceState3D>,
    shape_rid: Rid,
    area_rid: Rid,
    shape_transform: Transform3D,
) -> bool {
    // Create query parameters
    let mut query = PhysicsShapeQueryParameters3D::new_gd();
    query.set_shape_rid(shape_rid);
    query.set_transform(shape_transform);
    query.set_collision_mask(TRIGGER_COLLISION_LAYER); // Detect player's area
    query.set_collide_with_areas(true);
    query.set_collide_with_bodies(false);

    // Exclude self from the query
    let mut exclude = godot::prelude::Array::new();
    exclude.push(area_rid);
    query.set_exclude(exclude);

    // Query for overlapping shapes
    let results = space_state.intersect_shape(query);

    // If any results, player is overlapping
    !results.is_empty()
}

fn build_trigger_result(
    triggered_entity: &SceneEntityId,
    trigger_entity: &SceneEntityId,
    event_type: TriggerAreaEventType,
    timestamp: u32,
    trigger_transform: Transform3D,
    triggered_transform: Transform3D,
    scene_pos: Vector3,
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
            layers: CL_PLAYER,
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
