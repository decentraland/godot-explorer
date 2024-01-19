use std::{collections::HashSet, time::Duration};

use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::{
                pb_tween::Mode, EasingFunction, PbTween, PbTweenState, TweenStateStatus,
            },
            transform_and_parent::DclTransformAndParent,
            SceneComponentId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};

pub struct Tween {
    pub data: PbTween,
    pub ease_fn: fn(f32) -> f32,
    pub start_time: std::time::Instant,
    pub paused_time: Option<std::time::Instant>,
    pub duration: Duration,
    pub playing: Option<bool>,
}

impl Tween {
    fn get_progress(&self, elapsed_time: Duration) -> f32 {
        elapsed_time.as_millis() as f32 / self.duration.as_millis() as f32 // 0 to 1...
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

pub fn update_tween(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let tween_component = SceneCrdtStateProtoComponents::get_tween(crdt_state);

    let now = std::time::Instant::now();

    let mut tweens_to_delete = Vec::new();

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
                let duration = std::time::Duration::from_millis(new_value.duration as u64);

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
                            duration,
                            playing: None,
                        },
                    );
                };
            }
        }
    }

    for entity in tweens_to_delete {
        scene.tweens.remove(entity);

        // update tween state
        SceneCrdtStateProtoComponents::get_tween_state_mut(crdt_state).put(*entity, None);
    }

    for (entity, tween) in &mut scene.tweens {
        if tween.playing == Some(false) {
            continue;
        }

        let mut current_tween_state: TweenStateStatus = TweenStateStatus::TsActive;

        let elapsed_time = now - tween.start_time;

        let progress = if elapsed_time >= tween.duration {
            tween.playing = Some(false);
            current_tween_state = TweenStateStatus::TsCompleted;
            1.0 // finished
        } else {
            tween.get_progress(elapsed_time)
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

        // if we paused the tween, we skip the
        if tween.playing == Some(false) {
            continue;
        }

        // get entity transform from crdt state
        let mut transform: DclTransformAndParent = crdt_state
            .get_transform_mut()
            .get(entity)
            .and_then(|transform| transform.value.clone())
            .unwrap_or_else(DclTransformAndParent::default);

        // calculate new transform with the tween
        let ease_value = (tween.ease_fn)(progress);
        let new_transform = match &tween.data.mode {
            Some(Mode::Move(data)) => {
                let start = data.start.clone().unwrap().to_godot();
                let end = data.end.clone().unwrap().to_godot();

                if data.face_direction == Some(true) {
                    // TODO: This must be calculated one per tween data update, not per frame
                    // TODO: Implement transform.rotation = start.look_at(end)
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
            dirty.insert(*entity);
        } else {
            let mut new_dirty = HashSet::new();
            new_dirty.insert(*entity);
            scene
                .current_dirty
                .lww_components
                .insert(SceneComponentId::TRANSFORM, new_dirty.clone());
        }
    }
}
