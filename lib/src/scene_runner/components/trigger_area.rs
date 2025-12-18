use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use godot::{engine::PhysicsServer3D, prelude::*};
use once_cell::sync::Lazy;

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
    scene_runner::{
        pool_manager::PoolManager,
        scene::{Scene, SceneType},
    },
};

const CL_PLAYER: u32 = 4;

// ============================================================================
// Global Monitor Registry for PhysicsServer3D Callbacks
// ============================================================================

/// A pending ENTER/EXIT event from the PhysicsServer3D monitor callback
#[derive(Debug, Clone)]
struct PendingTriggerEvent {
    trigger_entity: SceneEntityId,
    /// The collider entity that entered/exited (PLAYER or scene entity)
    collider_entity: SceneEntityId,
    /// The collision layer of the collider
    collider_layer: u32,
    /// true = ENTER, false = EXIT
    is_enter: bool,
}

/// Global registry for trigger area callbacks.
/// Since all PhysicsServer3D callbacks run on the main thread, we use a simple Mutex.
#[derive(Default)]
struct TriggerAreaMonitor {
    /// Maps area RID -> (scene_id, entity_id, collision_mask)
    registry: HashMap<Rid, (SceneId, SceneEntityId, u32)>,
    /// Pending ENTER/EXIT events from callbacks, keyed by scene ID for O(1) drain per scene
    pending_events: HashMap<SceneId, Vec<PendingTriggerEvent>>,
}

static TRIGGER_MONITOR: Lazy<Mutex<TriggerAreaMonitor>> = Lazy::new(Default::default);

/// Register a trigger area in the global monitor
fn register_trigger_area(
    area_rid: Rid,
    scene_id: SceneId,
    entity_id: SceneEntityId,
    collision_mask: u32,
) {
    if let Ok(mut monitor) = TRIGGER_MONITOR.lock() {
        monitor
            .registry
            .insert(area_rid, (scene_id, entity_id, collision_mask));
    }
}

/// Unregister a trigger area from the global monitor
pub fn unregister_trigger_area(area_rid: Rid) {
    if let Ok(mut monitor) = TRIGGER_MONITOR.lock() {
        monitor.registry.remove(&area_rid);
    }
}

/// Drain pending events for a specific scene - O(1) lookup + O(E_scene) drain
fn drain_pending_events(scene_id: SceneId) -> Vec<PendingTriggerEvent> {
    if let Ok(mut monitor) = TRIGGER_MONITOR.lock() {
        monitor.pending_events.remove(&scene_id).unwrap_or_default()
    } else {
        Vec::new()
    }
}

/// Handle a body entering/exiting a trigger area (from PhysicsServer3D callback)
fn handle_body_monitor_event(
    area_rid: Rid,
    status: i64, // 0 = ADDED, 1 = REMOVED
    _body_rid: Rid,
    instance_id: i64,
    _body_shape_idx: i64,
    _local_shape_idx: i64,
) {
    let Ok(mut monitor) = TRIGGER_MONITOR.lock() else {
        return;
    };

    let Some(&(scene_id, trigger_entity, collision_mask)) = monitor.registry.get(&area_rid) else {
        // Stale callback from a released area, ignore
        return;
    };

    let is_enter = status == 0; // AREA_BODY_ADDED = 0

    // Try to get the collider object to determine entity type
    let (collider_entity, collider_layer) = if instance_id > 0 {
        let Ok(object) = Gd::<Object>::try_from_instance_id(InstanceId::from_i64(instance_id))
        else {
            return; // Invalid instance
        };

        // Check if instance is still valid (not being freed)
        if !object.is_instance_valid() {
            return;
        }

        // Check if this is a DCL entity or avatar with entity metadata
        if object.has_meta("dcl_entity_id".into()) {
            let dcl_entity_id = object.get_meta("dcl_entity_id".into()).to::<i32>();
            let dcl_scene_id = object.get_meta("dcl_scene_id".into()).to::<i32>();

            // Check if this is an avatar (has CL_PLAYER layer)
            let is_avatar = object
                .clone()
                .try_cast::<godot::engine::CollisionObject3D>()
                .ok()
                .map(|co| (co.get_collision_layer() & CL_PLAYER) != 0)
                .unwrap_or(false);

            if is_avatar {
                // Avatar detection (local player or remote avatar)
                // dcl_scene_id == -1 means it's local player or remote avatar
                // AvatarShapes (scene NPCs) have their trigger_detector freed, so they won't reach here
                (SceneEntityId::from_i32(dcl_entity_id), CL_PLAYER)
            } else {
                // Regular DCL scene entity (not avatar)
                // Only accept entities from the same scene
                if dcl_scene_id != scene_id.0 {
                    return; // Different scene, ignore
                }
                (
                    SceneEntityId::from_i32(dcl_entity_id),
                    collision_mask & !CL_PLAYER,
                )
            }
        } else {
            // No dcl_entity_id metadata - ignore
            return;
        }
    } else {
        return; // No instance ID
    };

    tracing::debug!(
        "[TriggerArea] {} trigger={:?}, collider={:?}",
        if is_enter { "ENTER" } else { "EXIT" },
        trigger_entity,
        collider_entity,
    );

    monitor
        .pending_events
        .entry(scene_id)
        .or_default()
        .push(PendingTriggerEvent {
            trigger_entity,
            collider_entity,
            collider_layer,
            is_enter,
        });
}

// ============================================================================
// TriggerAreaInstance and TriggerAreaState
// ============================================================================

/// State for a single trigger area instance
#[derive(Debug)]
pub struct TriggerAreaInstance {
    pub area_rid: Rid,
    pub shape_rid: Rid,
    /// Set of entities physically overlapping this trigger area (tracked by physics)
    pub entities_inside: HashSet<SceneEntityId>,
    pub mesh_type: TriggerAreaMeshType,
    pub collision_mask: u32,
    /// Whether this trigger area is active (player is in this scene)
    /// When inactive, physics monitoring is disabled and no events are generated
    pub is_active: bool,
}

/// Global trigger area state for a scene
#[derive(Debug, Default)]
pub struct TriggerAreaState {
    pub instances: HashMap<SceneEntityId, TriggerAreaInstance>,
}

impl TriggerAreaState {
    /// Cleanup all trigger areas, releasing RIDs back to the global pool
    pub fn cleanup(&mut self, pool: &mut crate::scene_runner::object_pool::PhysicsAreaPool) {
        // Unregister from global monitor and release to pool
        for (_, instance) in self.instances.drain() {
            unregister_trigger_area(instance.area_rid);
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

    /// Cleanup without pool (frees RIDs directly) - used when scene is destroyed
    pub fn cleanup_without_pool(&mut self) {
        let mut physics_server = PhysicsServer3D::singleton();
        for (_, instance) in self.instances.drain() {
            unregister_trigger_area(instance.area_rid);
            // Clear monitor callback before freeing to prevent stale events
            physics_server.area_set_monitor_callback(instance.area_rid, Callable::invalid());
            physics_server.free_rid(instance.area_rid);
            physics_server.free_rid(instance.shape_rid);
        }
    }
}

/// Called during scene update (with throttling) - handles component creation/deletion
/// and processes ENTER/EXIT events from callbacks + throttled STAY events
pub fn update_trigger_area(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    pools: &mut PoolManager,
    current_parcel_scene_id: &SceneId,
) {
    let trigger_area_component = SceneCrdtStateProtoComponents::get_trigger_area(crdt_state);

    // Step 1: Process dirty TriggerArea components
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

    let pool = pools.physics_area();
    for (entity, config) in entities_to_process {
        match config {
            Some(config) => {
                create_or_update_trigger_area(
                    scene,
                    &entity,
                    &config,
                    pool,
                    current_parcel_scene_id,
                );
            }
            None => {
                remove_trigger_area(scene, &entity, pool);
            }
        }
    }

    // Step 2: Check if scene became active/inactive and enable/disable trigger areas
    check_scene_active(scene, current_parcel_scene_id);

    // Step 3: Update transforms for all trigger areas
    update_trigger_area_transforms(scene);

    // Step 4: Process pending callback events (ENTER/EXIT)
    process_callback_events(scene);

    // Step 5: Generate STAY events for entities inside active trigger areas
    generate_stay_events(scene);
}

/// Helper to create monitor callback for a trigger area
fn create_monitor_callback(area_rid: Rid) -> Callable {
    Callable::from_fn("trigger_body_monitor", move |args: &[&Variant]| {
        if args.len() >= 5 {
            let status = args[0].to::<i64>();
            let body_rid = args[1].to::<Rid>();
            let instance_id = args[2].to::<i64>();
            let body_shape_idx = args[3].to::<i64>();
            let local_shape_idx = args[4].to::<i64>();
            handle_body_monitor_event(
                area_rid,
                status,
                body_rid,
                instance_id,
                body_shape_idx,
                local_shape_idx,
            );
        }
        Ok(Variant::nil())
    })
}

/// Check if the scene became active/inactive (player entered/left) and enable/disable trigger areas accordingly.
/// When player leaves: generate EXIT for all entities inside, disable physics monitoring.
/// When player enters: re-enable physics monitoring (physics will auto-fire ENTERs for overlapping bodies).
fn check_scene_active(scene: &mut Scene, current_parcel_scene_id: &SceneId) {
    // Global scenes are always active
    if !matches!(scene.scene_type, SceneType::Parcel) {
        return;
    }

    let was_active = scene.last_player_scene_id == scene.scene_id;
    let is_active = *current_parcel_scene_id == scene.scene_id;

    // Update stored value
    scene.last_player_scene_id = *current_parcel_scene_id;

    // No transition
    if was_active == is_active {
        return;
    }

    let mut physics_server = PhysicsServer3D::singleton();
    let tick_number = scene.tick_number;
    let scene_pos = scene.godot_dcl_scene.root_node_3d.get_global_position();

    if was_active && !is_active {
        // Player LEFT this scene - deactivate all trigger areas
        tracing::debug!(
            "[TriggerArea] Scene {:?} became INACTIVE - disabling {} trigger areas",
            scene.scene_id,
            scene.trigger_areas.instances.len()
        );

        for (trigger_entity, instance) in &mut scene.trigger_areas.instances {
            // Generate EXIT for everyone inside
            for entity in instance.entities_inside.drain() {
                let trigger_transform = scene
                    .godot_dcl_scene
                    .get_node_or_null_3d(trigger_entity)
                    .map(|n| n.get_global_transform())
                    .unwrap_or(Transform3D::IDENTITY);

                let collider_transform = scene
                    .godot_dcl_scene
                    .get_node_or_null_3d(&entity)
                    .map(|n| n.get_global_transform())
                    .unwrap_or(Transform3D::IDENTITY);

                let collider_layer = if entity == SceneEntityId::PLAYER {
                    CL_PLAYER
                } else {
                    instance.collision_mask
                };

                let result = build_trigger_result(
                    trigger_entity,
                    &entity,
                    TriggerAreaEventType::TaetExit,
                    tick_number,
                    collider_transform,
                    trigger_transform,
                    scene_pos,
                    collider_layer,
                );
                scene.trigger_area_results.push((*trigger_entity, result));
            }

            // Disable physics monitoring
            physics_server.area_set_monitor_callback(instance.area_rid, Callable::invalid());
            instance.is_active = false;
        }
    } else if !was_active && is_active {
        // Player ENTERED this scene - reactivate all trigger areas
        tracing::debug!(
            "[TriggerArea] Scene {:?} became ACTIVE - enabling {} trigger areas",
            scene.scene_id,
            scene.trigger_areas.instances.len()
        );

        for instance in scene.trigger_areas.instances.values_mut() {
            // Re-enable physics monitoring (will auto-fire ENTERs for overlapping bodies)
            let callback = create_monitor_callback(instance.area_rid);
            physics_server.area_set_monitor_callback(instance.area_rid, callback);
            instance.is_active = true;
        }
    }
}

/// Process ENTER/EXIT events from the global monitor callback queue.
/// Since trigger areas are disabled when player is not in scene, all events
/// received here are for active trigger areas.
fn process_callback_events(scene: &mut Scene) {
    let events = drain_pending_events(scene.scene_id);
    if events.is_empty() {
        return;
    }

    let tick_number = scene.tick_number;
    let scene_pos = scene.godot_dcl_scene.root_node_3d.get_global_position();

    for event in events {
        let Some(instance) = scene.trigger_areas.instances.get_mut(&event.trigger_entity) else {
            continue;
        };

        // Skip events for inactive trigger areas (shouldn't happen, but be safe)
        if !instance.is_active {
            continue;
        }

        let trigger_transform = scene
            .godot_dcl_scene
            .get_node_or_null_3d(&event.trigger_entity)
            .map(|n| n.get_global_transform())
            .unwrap_or(Transform3D::IDENTITY);

        let collider_transform = scene
            .godot_dcl_scene
            .get_node_or_null_3d(&event.collider_entity)
            .map(|n| n.get_global_transform())
            .unwrap_or(Transform3D::IDENTITY);

        if event.is_enter {
            instance.entities_inside.insert(event.collider_entity);

            let result = build_trigger_result(
                &event.trigger_entity,
                &event.collider_entity,
                TriggerAreaEventType::TaetEnter,
                tick_number,
                collider_transform,
                trigger_transform,
                scene_pos,
                event.collider_layer,
            );
            scene
                .trigger_area_results
                .push((event.trigger_entity, result));
        } else if instance.entities_inside.remove(&event.collider_entity) {
            let result = build_trigger_result(
                &event.trigger_entity,
                &event.collider_entity,
                TriggerAreaEventType::TaetExit,
                tick_number,
                collider_transform,
                trigger_transform,
                scene_pos,
                event.collider_layer,
            );
            scene
                .trigger_area_results
                .push((event.trigger_entity, result));
        }
    }
}

/// Generate STAY events for entities inside active trigger areas
fn generate_stay_events(scene: &mut Scene) {
    let tick_number = scene.tick_number;
    let scene_pos = scene.godot_dcl_scene.root_node_3d.get_global_position();

    // Collect data for STAY events (avoid borrowing issues)
    let stay_data: Vec<_> = scene
        .trigger_areas
        .instances
        .iter()
        .filter_map(|(trigger_entity, instance)| {
            if !instance.is_active || instance.entities_inside.is_empty() {
                return None;
            }

            let entities: Vec<_> = instance
                .entities_inside
                .iter()
                .map(|e| (*e, instance.collision_mask))
                .collect();

            let trigger_transform = scene
                .godot_dcl_scene
                .get_node_or_null_3d(trigger_entity)
                .map(|n| n.get_global_transform())
                .unwrap_or(Transform3D::IDENTITY);

            Some((*trigger_entity, entities, trigger_transform))
        })
        .collect();

    // Generate STAY events
    for (trigger_entity, entities, trigger_transform) in stay_data {
        for (collider_entity, collision_mask) in entities {
            let collider_transform = scene
                .godot_dcl_scene
                .get_node_or_null_3d(&collider_entity)
                .map(|n| n.get_global_transform())
                .unwrap_or(Transform3D::IDENTITY);

            let collider_layer = if collider_entity == SceneEntityId::PLAYER {
                CL_PLAYER
            } else {
                collision_mask
            };

            let result = build_trigger_result(
                &trigger_entity,
                &collider_entity,
                TriggerAreaEventType::TaetStay,
                tick_number,
                collider_transform,
                trigger_transform,
                scene_pos,
                collider_layer,
            );
            scene.trigger_area_results.push((trigger_entity, result));
        }
    }
}

fn create_or_update_trigger_area(
    scene: &mut Scene,
    entity: &SceneEntityId,
    config: &PbTriggerArea,
    pool: &mut crate::scene_runner::object_pool::PhysicsAreaPool,
    current_parcel_scene_id: &SceneId,
) {
    let mut physics_server = PhysicsServer3D::singleton();
    let mesh_type = config.mesh();
    let collision_mask = config.collision_mask.unwrap_or(CL_PLAYER);
    let scene_id = scene.scene_id;

    // Determine if this trigger area should be active
    // Global scenes are always active, parcel scenes only when player is in them
    let is_active = !matches!(scene.scene_type, SceneType::Parcel)
        || *current_parcel_scene_id == scene.scene_id;

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
        physics_server.area_set_collision_layer(area_rid, collision_mask);
        physics_server.area_set_collision_mask(area_rid, collision_mask);
        physics_server.area_set_monitorable(area_rid, true);

        // Register in global monitor for callback routing
        register_trigger_area(area_rid, scene_id, *entity, collision_mask);

        // Only set up monitor callback if active
        if is_active {
            let callback = create_monitor_callback(area_rid);
            physics_server.area_set_monitor_callback(area_rid, callback);
        }

        tracing::debug!(
            "[TriggerArea] CREATE entity={:?}, mesh={:?}, mask={}, active={}",
            entity,
            mesh_type,
            collision_mask,
            is_active
        );

        scene.trigger_areas.instances.insert(
            *entity,
            TriggerAreaInstance {
                area_rid,
                shape_rid,
                entities_inside: HashSet::new(),
                mesh_type,
                collision_mask,
                is_active,
            },
        );
    } else if let Some(instance) = scene.trigger_areas.instances.get_mut(entity) {
        // Update collision mask in both instance and global registry
        if instance.collision_mask != collision_mask {
            instance.collision_mask = collision_mask;
            physics_server.area_set_collision_mask(instance.area_rid, collision_mask);
            register_trigger_area(instance.area_rid, scene_id, *entity, collision_mask);
        }
    }
}

fn remove_trigger_area(
    scene: &mut Scene,
    entity: &SceneEntityId,
    pool: &mut crate::scene_runner::object_pool::PhysicsAreaPool,
) {
    if let Some(instance) = scene.trigger_areas.instances.remove(entity) {
        tracing::debug!("[TriggerArea] DELETE entity={:?}", entity);
        unregister_trigger_area(instance.area_rid);
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

#[allow(clippy::too_many_arguments)]
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
