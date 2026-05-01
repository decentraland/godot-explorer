use godot::{classes::Node3D, obj::Gd};
use std::{
    collections::{HashMap, HashSet},
    sync::atomic::Ordering,
};

use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::{
                common::{InputAction, InteractionType, PointerEventType},
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
        raycast_result: &godot::prelude::VarDictionary,
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

            tracing::debug!(
                "PointerEvents update: entity {:?}, has_pointer_events={}",
                entity,
                new_value.is_some()
            );

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

    pub fn has_pointer_event_without_proximity(&self, pet: PointerEventType) -> bool {
        self.pointer_events.iter().any(|pe| {
            pe.event_type() == pet
                && pe.interaction_type != Some(i32::from(InteractionType::Proximity))
        })
    }

    pub fn has_any_pointer_event_without_proximity(&self) -> bool {
        self.pointer_events
            .iter()
            .any(|pe| pe.interaction_type != Some(i32::from(InteractionType::Proximity)))
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

pub fn find_active_proximity_entity(
    scenes: &HashMap<SceneId, Scene>,
    player_position: godot::prelude::Vector3,
    camera_and_viewport: &Option<(
        godot::prelude::Gd<godot::classes::Camera3D>,
        godot::prelude::Vector2,
    )>,
) -> Option<(SceneId, SceneEntityId)> {
    let proximity_type = i32::from(InteractionType::Proximity);
    let mut best_priority = i64::MIN;
    let mut best_distance = f32::MAX;
    let mut best_entity: Option<(SceneId, SceneEntityId)> = None;

    for (scene_id, scene) in scenes.iter() {
        for (entity_id, entity_node) in scene.godot_dcl_scene.entities.iter() {
            let Some(ref pointer_events) = entity_node.pointer_events else {
                continue;
            };
            let Some(ref node_3d) = entity_node.base_3d else {
                continue;
            };
            for pe in pointer_events.pointer_events.iter() {
                if pe.interaction_type != Some(proximity_type) {
                    continue;
                }
                let Some(ref info) = pe.event_info else {
                    continue;
                };
                let max_player_distance = info.max_player_distance.unwrap_or(0.0);
                let entity_position = node_3d.get_global_position();
                let distance = player_position.distance_to(entity_position);
                if distance > max_player_distance {
                    continue;
                }
                if let Some((ref camera, viewport_size)) = camera_and_viewport {
                    if camera.is_position_behind(entity_position) {
                        continue;
                    }
                    let screen_pos = camera.unproject_position(entity_position);
                    let screen_center = *viewport_size / 2.0;
                    let threshold = viewport_size.x.min(viewport_size.y) / 3.0;
                    if screen_pos.distance_to(screen_center) >= threshold {
                        continue;
                    }
                }
                let priority = info.priority.unwrap_or(0) as i64;
                if priority < best_priority
                    || (priority == best_priority && distance >= best_distance)
                {
                    continue;
                }
                best_priority = priority;
                best_distance = distance;
                best_entity = Some((*scene_id, *entity_id));
            }
        }
    }

    best_entity
}

pub fn pointer_events_system(
    scenes: &mut HashMap<SceneId, Scene>,
    changed_inputs: &HashSet<(InputAction, bool)>,
    previous_raycast: &Option<GodotDclRaycastResult>,
    current_raycast: &Option<GodotDclRaycastResult>,
    player_position: godot::prelude::Vector3,
    camera_and_viewport: &Option<(
        godot::prelude::Gd<godot::classes::Camera3D>,
        godot::prelude::Vector2,
    )>,
    last_proximity_entity: &mut Option<(SceneId, SceneEntityId)>,
) {
    let global_tick_number = GLOBAL_TICK_NUMBER.load(Ordering::Relaxed);

    let pointing_at_cursor = current_raycast.as_ref().is_some_and(|raycast| {
        get_entity_pointer_event(scenes, &raycast.scene_id, &raycast.entity_id)
            .is_some_and(|pe| pe.has_any_pointer_event_without_proximity())
    });

    proximity_events_system(
        scenes,
        changed_inputs,
        player_position,
        camera_and_viewport,
        last_proximity_entity,
        pointing_at_cursor,
        global_tick_number,
    );

    pointer_events_system_without_proximity(
        scenes,
        changed_inputs,
        previous_raycast,
        current_raycast,
        global_tick_number,
    );
}

fn proximity_events_system(
    scenes: &mut HashMap<SceneId, Scene>,
    changed_inputs: &HashSet<(InputAction, bool)>,
    player_position: godot::prelude::Vector3,
    camera_and_viewport: &Option<(
        godot::prelude::Gd<godot::classes::Camera3D>,
        godot::prelude::Vector2,
    )>,
    last_proximity_entity: &mut Option<(SceneId, SceneEntityId)>,
    pointing_at_cursor: bool,
    global_tick_number: u32,
) {
    let proximity_type = i32::from(InteractionType::Proximity);

    let current_proximity_entity = if pointing_at_cursor {
        None
    } else {
        find_active_proximity_entity(scenes, player_position, camera_and_viewport)
    };

    if *last_proximity_entity != current_proximity_entity {
        if let Some((scene_id, entity_id)) = *last_proximity_entity {
            if let Some(pointer_events) = get_entity_pointer_event(scenes, &scene_id, &entity_id) {
                if pointer_events.has_pointer_event(PointerEventType::PetHoverLeave) {
                    let result = PbPointerEventsResult {
                        button: InputAction::IaAny as i32,
                        hit: None,
                        state: PointerEventType::PetHoverLeave as i32,
                        timestamp: GLOBAL_TIMESTAMP.fetch_add(1, Ordering::Relaxed),
                        analog: None,
                        tick_number: global_tick_number,
                    };
                    if let Some(scene) = scenes.get_mut(&scene_id) {
                        scene.pointer_events_result.push((entity_id, result));
                    }
                }
            }
        }

        if let Some((scene_id, entity_id)) = current_proximity_entity {
            if let Some(pointer_events) = get_entity_pointer_event(scenes, &scene_id, &entity_id) {
                if pointer_events.has_pointer_event(PointerEventType::PetHoverEnter) {
                    let result = PbPointerEventsResult {
                        button: InputAction::IaAny as i32,
                        hit: None,
                        state: PointerEventType::PetHoverEnter as i32,
                        timestamp: GLOBAL_TIMESTAMP.fetch_add(1, Ordering::Relaxed),
                        analog: None,
                        tick_number: global_tick_number,
                    };
                    if let Some(scene) = scenes.get_mut(&scene_id) {
                        scene.pointer_events_result.push((entity_id, result));
                    }
                }
            }
        }
    }

    if let Some((scene_id, entity_id)) = current_proximity_entity {
        if let Some(pointer_events) = get_entity_pointer_event(scenes, &scene_id, &entity_id) {
            let pointer_events = pointer_events.clone();
            for pe in pointer_events.pointer_events.iter() {
                if pe.interaction_type != Some(proximity_type) {
                    continue;
                }
                let Some(ref info) = pe.event_info else {
                    continue;
                };
                let pe_button = info.button.unwrap_or(InputAction::IaAny as i32);
                for (input_action, state) in changed_inputs.iter() {
                    let matches_button = *input_action as i32 == pe_button
                        || pe_button == InputAction::IaAny as i32
                        || *input_action == InputAction::IaAny;
                    if !matches_button {
                        continue;
                    }
                    let match_state = (*state && pe.event_type == PointerEventType::PetDown as i32)
                        || (!state && pe.event_type == PointerEventType::PetUp as i32);
                    if match_state {
                        let result = PbPointerEventsResult {
                            button: *input_action as i32,
                            hit: None,
                            state: pe.event_type,
                            timestamp: GLOBAL_TIMESTAMP.fetch_add(1, Ordering::Relaxed),
                            analog: None,
                            tick_number: global_tick_number,
                        };
                        if let Some(scene) = scenes.get_mut(&scene_id) {
                            scene.pointer_events_result.push((entity_id, result));
                        }
                    }
                }
            }
        }
    }

    *last_proximity_entity = current_proximity_entity;
}

fn pointer_events_system_without_proximity(
    scenes: &mut HashMap<SceneId, Scene>,
    changed_inputs: &HashSet<(InputAction, bool)>,
    previous_raycast: &Option<GodotDclRaycastResult>,
    current_raycast: &Option<GodotDclRaycastResult>,
    global_tick_number: u32,
) {
    if !GodotDclRaycastResult::eq_key(current_raycast, previous_raycast) {
        if let Some(raycast) = previous_raycast.as_ref() {
            if let Some(pointer_event) =
                get_entity_pointer_event(scenes, &raycast.scene_id, &raycast.entity_id)
            {
                if pointer_event
                    .has_pointer_event_without_proximity(PointerEventType::PetHoverLeave)
                {
                    let pointer_event_result = PbPointerEventsResult {
                        button: InputAction::IaAny as i32,
                        hit: None,
                        state: PointerEventType::PetHoverLeave as i32,
                        timestamp: GLOBAL_TIMESTAMP.fetch_add(1, Ordering::Relaxed),
                        analog: None,
                        tick_number: global_tick_number,
                    };

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
                if pointer_event
                    .has_pointer_event_without_proximity(PointerEventType::PetHoverEnter)
                {
                    let pointer_event_result = PbPointerEventsResult {
                        button: InputAction::IaAny as i32,
                        hit: None,
                        state: PointerEventType::PetHoverEnter as i32,
                        timestamp: GLOBAL_TIMESTAMP.fetch_add(1, Ordering::Relaxed),
                        analog: None,
                        tick_number: global_tick_number,
                    };

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

    if pointer_event.is_none() {
        if let Some(raycast) = current_raycast.as_ref() {
            tracing::debug!(
                "Raycast hit entity {:?} in scene {:?} but NO PointerEvents configured",
                raycast.entity_id,
                raycast.scene_id
            );
        }
    }

    if pointer_event.is_none() || changed_inputs.is_empty() {
        return;
    }

    let pointer_event = pointer_event.unwrap().clone();

    for pointer_event in pointer_event.pointer_events.iter() {
        if pointer_event.interaction_type == Some(i32::from(InteractionType::Proximity)) {
            continue;
        }

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
