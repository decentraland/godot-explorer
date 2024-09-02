use std::{
    collections::{HashMap, HashSet},
    sync::atomic::Ordering,
};

use godot::{engine::Node3D, obj::Gd};

use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::{
                common::{InputAction, PointerEventType},
                PbPointerEvents, PbPointerEventsResult,
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
        scene::{GodotDclRaycastResult, Scene},
        scene_manager::{GLOBAL_TICK_NUMBER, GLOBAL_TIMESTAMP},
    },
};

impl crate::dcl::components::proto_components::sdk::components::common::RaycastHit {
    pub fn from_godot_raycast(
        scene_position: godot::prelude::Vector3,
        raycast_from: godot::prelude::Vector3,
        raycast_result: &godot::prelude::Dictionary,
        entity_id: Option<u32>,
    ) -> Option<Self> {
        let global_origin = raycast_from - scene_position;
        let position = raycast_result
            .get("position")?
            .to::<godot::prelude::Vector3>()
            - scene_position;
        let direction = global_origin - position;
        let normal = raycast_result
            .get("normal")?
            .to::<godot::prelude::Vector3>();

        let distance = global_origin.distance_to(position);

        // Get node name from the collider, and remove everything that is after `_colgen`
        let mesh_name: Option<String> = raycast_result
            .get("collider")
            .and_then(|collider| collider.try_to::<Gd<Node3D>>().ok())
            .map(|mesh| mesh.get_name().to_string())
            .map(|mesh_name| {
                mesh_name
                    .split("_colgen")
                    .next()
                    .unwrap_or(&mesh_name)
                    .to_string()
            });

        Some(Self {
            // the intersection point in global coordinates
            position: Some(crate::dcl::components::proto_components::common::Vector3 {
                x: position.x,
                y: position.y,
                z: -position.z,
            }),
            // the starting point of the ray in global coordinates
            global_origin: Some(crate::dcl::components::proto_components::common::Vector3 {
                x: global_origin.x,
                y: global_origin.y,
                z: -global_origin.z,
            }),
            // the direction vector of the ray in global coordinates
            direction: Some(crate::dcl::components::proto_components::common::Vector3 {
                x: direction.x,
                y: direction.y,
                z: -direction.z,
            }),
            // normal of the hit surface in global coordinates
            normal_hit: Some(crate::dcl::components::proto_components::common::Vector3 {
                x: normal.x,
                y: normal.y,
                z: -normal.z,
            }),
            // the distance between the ray origin and the hit position
            length: distance,
            // mesh name, if collision happened inside a GltfContainer
            mesh_name,
            // the ID of the Entity that has the impacted mesh attached
            entity_id,
        })
    }


    pub fn from_data(
        scene_position: godot::prelude::Vector3,
        raycast_from: godot::prelude::Vector3,
        position: godot::prelude::Vector3,
        normal: godot::prelude::Vector3,
        mesh_name: Option<String>,
        entity_id: Option<u32>,
    ) -> Option<Self> {
        let global_origin = raycast_from - scene_position;
        let position = position - scene_position;
        let direction = global_origin - position;

        let distance = global_origin.distance_to(position);

        Some(Self {
            // the intersection point in global coordinates
            position: Some(crate::dcl::components::proto_components::common::Vector3 {
                x: position.x,
                y: position.y,
                z: -position.z,
            }),
            // the starting point of the ray in global coordinates
            global_origin: Some(crate::dcl::components::proto_components::common::Vector3 {
                x: global_origin.x,
                y: global_origin.y,
                z: -global_origin.z,
            }),
            // the direction vector of the ray in global coordinates
            direction: Some(crate::dcl::components::proto_components::common::Vector3 {
                x: direction.x,
                y: direction.y,
                z: -direction.z,
            }),
            // normal of the hit surface in global coordinates
            normal_hit: Some(crate::dcl::components::proto_components::common::Vector3 {
                x: normal.x,
                y: normal.y,
                z: -normal.z,
            }),
            // the distance between the ray origin and the hit position
            length: distance,
            // mesh name, if collision happened inside a GltfContainer
            mesh_name,
            // the ID of the Entity that has the impacted mesh attached
            entity_id,
        })
    }
}

pub fn update_scene_pointer_events(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    if let Some(pointer_events_dirty) = dirty_lww_components.get(&SceneComponentId::POINTER_EVENTS)
    {
        let pointer_events_component =
            SceneCrdtStateProtoComponents::get_pointer_events(crdt_state);

        for entity in pointer_events_dirty {
            let new_value = pointer_events_component.get(entity);
            let new_value = if let Some(value) = new_value {
                value.value.clone()
            } else {
                None
            };

            let godot_entity_node = godot_dcl_scene.ensure_godot_entity_node(entity);

            if let Some(base_ui) = godot_entity_node.base_ui.as_mut() {
                base_ui
                    .base_control
                    .bind_mut()
                    .set_pointer_events(&new_value);
            }

            godot_entity_node.pointer_events = new_value;
        }
    }
}

impl PbPointerEvents {
    pub fn has_pointer_event(&self, pet: PointerEventType) -> bool {
        self.pointer_events.iter().any(|pe| pe.event_type() == pet)
    }
}

pub fn get_entity_pointer_event<'a>(
    scenes: &'a HashMap<SceneId, Scene>,
    scene_id: &SceneId,
    entity_id: &SceneEntityId,
) -> Option<&'a PbPointerEvents> {
    let scene = scenes.get(scene_id)?;
    let entity = scene.godot_dcl_scene.get_godot_entity_node(entity_id)?;
    let pointer_events = entity.pointer_events.as_ref()?;
    Some(pointer_events)
}

pub fn pointer_events_system(
    scenes: &mut HashMap<SceneId, Scene>,
    changed_inputs: &HashSet<(InputAction, bool)>,
    previous_raycast: &Option<GodotDclRaycastResult>,
    current_raycast: &Option<GodotDclRaycastResult>,
) {
    let global_tick_number = GLOBAL_TICK_NUMBER.load(Ordering::Relaxed);

    if !GodotDclRaycastResult::eq_key(current_raycast, previous_raycast) {
        if let Some(raycast) = previous_raycast.as_ref() {
            if let Some(pointer_event) =
                get_entity_pointer_event(scenes, &raycast.scene_id, &raycast.entity_id)
            {
                if pointer_event.has_pointer_event(PointerEventType::PetHoverLeave) {
                    let pointer_event_result = PbPointerEventsResult {
                        button: InputAction::IaAny as i32,
                        hit: None,
                        state: PointerEventType::PetHoverLeave as i32,
                        timestamp: GLOBAL_TIMESTAMP.fetch_add(1, Ordering::Relaxed),
                        analog: None,
                        tick_number: global_tick_number,
                    };

                    // Append pointer event result
                    scenes
                        .get_mut(&raycast.scene_id)
                        .unwrap()
                        .pointer_events_result
                        .push((raycast.entity_id, pointer_event_result));
                }
            }
        }

        if let Some(raycast) = current_raycast.as_ref() {
            if let Some(pointer_event) =
                get_entity_pointer_event(scenes, &raycast.scene_id, &raycast.entity_id)
            {
                if pointer_event.has_pointer_event(PointerEventType::PetHoverEnter) {
                    let pointer_event_result = PbPointerEventsResult {
                        button: InputAction::IaAny as i32,
                        hit: None,
                        state: PointerEventType::PetHoverEnter as i32,
                        timestamp: GLOBAL_TIMESTAMP.fetch_add(1, Ordering::Relaxed),
                        analog: None,
                        tick_number: global_tick_number,
                    };

                    // Append pointer event result
                    scenes
                        .get_mut(&raycast.scene_id)
                        .unwrap()
                        .pointer_events_result
                        .push((raycast.entity_id, pointer_event_result));
                }
            }
        }
    }

    let (current_raycast_scene_id, current_raycast_entity_id, raycast_hit) =
        if let Some(raycast) = current_raycast.as_ref() {
            (
                raycast.scene_id,
                raycast.entity_id,
                Some(raycast.hit.clone()),
            )
        } else {
            (SceneId::default(), SceneEntityId::new(0, 0), None)
        };

    for (scene_id, scene) in scenes.iter_mut() {
        for (input_action, state) in changed_inputs {
            let state = if *state {
                PointerEventType::PetDown
            } else {
                PointerEventType::PetUp
            } as i32;

            // Just send the raycast data if we hit something of that scene
            let raycast_hit = if current_raycast_scene_id == *scene_id {
                raycast_hit.clone()
            } else {
                None
            };

            scene.pointer_events_result.push((
                SceneEntityId::new(0, 0),
                PbPointerEventsResult {
                    button: *input_action as i32,
                    hit: raycast_hit,
                    state,
                    timestamp: GLOBAL_TIMESTAMP.fetch_add(1, Ordering::Relaxed),
                    analog: None,
                    tick_number: global_tick_number,
                },
            ));
        }
    }

    let pointer_event = get_entity_pointer_event(
        scenes,
        &current_raycast_scene_id,
        &current_raycast_entity_id,
    );

    if pointer_event.is_none() || changed_inputs.is_empty() {
        return;
    }

    let pointer_event = pointer_event.unwrap().clone();

    for pointer_event in pointer_event.pointer_events.iter() {
        if pointer_event.event_info.is_none() {
            continue;
        }

        let event_info = pointer_event.event_info.as_ref().unwrap();
        let pointer_event_button = event_info.button.unwrap_or(InputAction::IaAny as i32);

        if let Some(raycast_hit) = raycast_hit.clone() {
            let max_distance = *event_info.max_distance.as_ref().unwrap_or(&10.0);
            if raycast_hit.length > max_distance {
                continue;
            }
        }

        for (input_action, state) in changed_inputs {
            if *input_action == InputAction::IaAny // FIX: Is this possible? :S
                || *input_action as i32 == event_info.button.unwrap_or(InputAction::IaAny as i32)
                || pointer_event_button == InputAction::IaAny as i32
            {
                let match_state = (*state
                    && pointer_event.event_type == PointerEventType::PetDown as i32)
                    || (!*state && pointer_event.event_type == PointerEventType::PetUp as i32);

                if match_state {
                    let pointer_event_result = PbPointerEventsResult {
                        button: *input_action as i32,
                        hit: raycast_hit.clone(),
                        state: pointer_event.event_type,
                        timestamp: GLOBAL_TIMESTAMP.fetch_add(1, Ordering::Relaxed),
                        analog: None,
                        tick_number: global_tick_number,
                    };

                    // Append pointer event result
                    scenes
                        .get_mut(&current_raycast_scene_id)
                        .unwrap()
                        .pointer_events_result
                        .push((current_raycast_entity_id, pointer_event_result));
                }
            }
        }
    }
}
