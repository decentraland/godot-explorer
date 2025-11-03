use crate::{
    dcl::{
        components::{
            proto_components::{
                self,
                sdk::components::{
                    common::RaycastHit, pb_raycast, PbRaycast, PbRaycastResult, RaycastQueryType,
                },
            },
            SceneComponentId, SceneEntityId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_global::DclGlobal,
    scene_runner::scene::Scene,
};
use godot::{
    classes::{PhysicsDirectSpaceState3D, PhysicsRayQueryParameters3D},
    prelude::*,
};

pub fn update_raycasts(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let mut one_shot_raycast = Vec::new();
    let raycast_component = SceneCrdtStateProtoComponents::get_raycast(crdt_state);

    if let Some(raycast_dirty) = dirty_lww_components.get(&SceneComponentId::RAYCAST) {
        for entity in raycast_dirty {
            let new_value = raycast_component.get(entity);
            if new_value.is_none() {
                scene.continuos_raycast.remove(entity);
                continue;
            }

            if let Some(raycast_value) = new_value.unwrap().value.as_ref() {
                if raycast_value.continuous() {
                    scene.continuos_raycast.insert(*entity);
                } else {
                    one_shot_raycast.push(*entity);
                }
            } else {
                scene.continuos_raycast.remove(entity);
            }
        }
    }

    if one_shot_raycast.is_empty() && scene.continuos_raycast.is_empty() {
        return;
    }

    let raycasts = one_shot_raycast
        .iter()
        .chain(scene.continuos_raycast.iter());
    let mut raycast_results = Vec::new();
    for entity in raycasts {
        let (_, node_3d) = scene.godot_dcl_scene.ensure_node_3d(entity);

        let Some(raycast) = raycast_component.get(entity) else {
            continue;
        };

        let Some(raycast) = raycast.value.as_ref() else {
            continue;
        };

        let result = do_raycast(scene, &node_3d, raycast);
        raycast_results.push((entity, result));
    }

    let raycast_result_component =
        SceneCrdtStateProtoComponents::get_raycast_result_mut(crdt_state);

    for (entity, result) in raycast_results {
        raycast_result_component.put(*entity, Some(result));
    }
}

fn do_raycast(scene: &Scene, node_3d: &Gd<Node3D>, raycast: &PbRaycast) -> PbRaycastResult {
    let tick_number = scene.tick_number;
    let query_type = match RaycastQueryType::from_i32(raycast.query_type) {
        Some(query_type) => {
            if query_type == RaycastQueryType::RqtNone {
                return PbRaycastResult::default();
            } else {
                query_type
            }
        }
        _ => {
            return PbRaycastResult::default();
        }
    };

    let scene_position = scene.godot_dcl_scene.root_node_3d.get_global_position();

    let global_origin_offset = if let Some(offset) = raycast.origin_offset.as_ref() {
        let transform = node_3d
            .get_global_transform()
            .translated_local(Vector3::new(offset.x, offset.y, -offset.z));
        transform.origin
    } else {
        node_3d.get_global_position()
    };

    let raycast_from = global_origin_offset;

    let direction = match raycast.direction.as_ref() {
        Some(direction) => match direction {
            pb_raycast::Direction::LocalDirection(local_direction) => {
                let local_direction =
                    Vector3::new(local_direction.x, local_direction.y, -local_direction.z);

                quaternion_multiply_vector(
                    &node_3d.get_global_transform().basis.to_quat(),
                    &local_direction,
                )
            }
            pb_raycast::Direction::GlobalDirection(global_direction) => {
                Vector3::new(global_direction.x, global_direction.y, -global_direction.z)
            }

            pb_raycast::Direction::GlobalTarget(global_target) => {
                Vector3::new(global_target.x, global_target.y, -global_target.z) + scene_position
                    - raycast_from
            }
            pb_raycast::Direction::TargetEntity(target_entity) => {
                let target_entity: SceneEntityId = SceneEntityId::from_i32(*target_entity as i32);
                if let Some(target_entity_node_3d) =
                    scene.godot_dcl_scene.get_node_or_null_3d(&target_entity)
                {
                    target_entity_node_3d.get_global_position() - raycast_from
                } else {
                    scene_position - raycast_from
                }
            }
        },
        None => Vector3::UP,
    }
    .normalized();
    let raycast_distance = raycast.max_distance.max(1000.0); // engine constraints
    let raycast_to = raycast_from + direction * raycast_distance;

    // TODO: check unwrap
    let space = node_3d
        .get_world_3d()
        .unwrap()
        .get_direct_space_state()
        .unwrap();
    let mut raycast_query = PhysicsRayQueryParameters3D::new_gd();
    let collision_mask = raycast.collision_mask.unwrap_or(3);

    raycast_query.set_from(raycast_from);
    raycast_query.set_to(raycast_to);
    raycast_query.set_collision_mask(collision_mask);

    // debug drawing the ray
    if let Some(mut global) = DclGlobal::try_singleton() {
        let id: i64 = node_3d.instance_id().to_i64();
        global.call_deferred(
            "add_raycast",
            &[
                Variant::from(id),
                Variant::from(1.0),
                Variant::from(raycast_from),
                Variant::from(raycast_to),
            ],
        );
    }

    let hits = match query_type {
        RaycastQueryType::RqtHitFirst => {
            if let Some(hit) = get_raycast_hit(scene, space.clone(), raycast_query.clone()) {
                vec![hit.0]
            } else {
                vec![]
            }
        }
        RaycastQueryType::RqtQueryAll => {
            let mut counter = 0;
            let mut hits = vec![];
            while let Some((hit, rid)) =
                get_raycast_hit(scene, space.clone(), raycast_query.clone())
            {
                hits.push(hit);

                let mut arr = raycast_query.get_exclude();
                arr.push(rid);
                raycast_query.set_exclude(arr);

                // Limitation up to 10 hitss
                counter += 1;
                if counter > 10 {
                    break;
                }
            }
            hits
        }
        _ => {
            vec![]
        }
    };

    PbRaycastResult {
        // timestamp is a correlation id, copied from the PBRaycast
        timestamp: raycast.timestamp,
        // the starting point of the ray in global coordinates
        global_origin: Some(proto_components::common::Vector3 {
            x: raycast_from.x - scene_position.x,
            y: raycast_from.y - scene_position.y,
            z: -(raycast_from.z - scene_position.z),
        }),
        // the direction vector of the ray in global coordinates
        direction: Some(proto_components::common::Vector3 {
            x: direction.x,
            y: direction.y,
            z: -direction.z,
        }),
        // zero or more hits
        hits,
        // number of tick in which the event was produced, equals to EngineInfo.tick_number
        tick_number,
    }
}

fn get_raycast_hit(
    scene: &Scene,
    mut space: Gd<PhysicsDirectSpaceState3D>,
    raycast_query: Gd<PhysicsRayQueryParameters3D>,
) -> Option<(RaycastHit, Rid)> {
    let raycast_result = space.intersect_ray(raycast_query.clone());
    let collider = raycast_result.get("collider")?;

    let has_dcl_entity_id = collider
        .call(
            StringName::from("has_meta"),
            &[Variant::from("dcl_entity_id")],
        )
        .booleanize();

    // Note here, if the collider is not in the scene, it stops all query type
    if !has_dcl_entity_id {
        return None;
    }

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

    if dcl_scene_id != scene.scene_id.0 {
        return None;
    }

    let scene_position = scene.godot_dcl_scene.root_node_3d.get_global_position();
    let raycast_data: RaycastHit = RaycastHit::from_godot_raycast(
        scene_position,
        raycast_query.get_from(),
        &raycast_result,
        Some(dcl_entity_id as u32),
    )?;

    let rid = raycast_result.get("rid").unwrap().to::<Rid>();

    Some((raycast_data, rid))
}

// TODO: move to a impl for godot::Quaternion
fn quaternion_multiply_vector(q: &Quaternion, v: &Vector3) -> Vector3 {
    let ix = q.w * v.x + q.y * v.z - q.z * v.y;
    let iy = q.w * v.y + q.z * v.x - q.x * v.z;
    let iz = q.w * v.z + q.x * v.y - q.y * v.x;
    let iw = -q.x * v.x - q.y * v.y - q.z * v.z;
    Vector3 {
        x: ix * q.w + iw * -q.x + iy * -q.z - iz * -q.y,
        y: iy * q.w + iw * -q.y + iz * -q.x - ix * -q.z,
        z: iz * q.w + iw * -q.z + ix * -q.y - iy * -q.x,
    }
}
