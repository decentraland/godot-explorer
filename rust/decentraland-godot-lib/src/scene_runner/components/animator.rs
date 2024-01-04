use crate::{
    dcl::{
        components::{proto_components::sdk::components::PbAnimator, SceneComponentId},
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::{godot_dcl_scene::GodotEntityNode, scene::Scene},
};
use godot::{
    engine::{animation::LoopMode, AnimationPlayer},
    prelude::*,
};

fn get_animation_player(godot_entity_node: &mut GodotEntityNode) -> Option<Gd<AnimationPlayer>> {
    godot_entity_node
        .base_3d
        .as_ref()?
        .try_get_node_as::<Node>(NodePath::from("GltfContainer"))?
        .get_child(0)?
        .try_get_node_as::<AnimationPlayer>("AnimationPlayer")
}

pub fn update_animator(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    if let Some(animator_dirty) = dirty_lww_components.get(&SceneComponentId::ANIMATOR) {
        let animator_component = SceneCrdtStateProtoComponents::get_animator(crdt_state);

        for entity in animator_dirty {
            let new_value = animator_component.get(entity);
            if new_value.is_none() {
                scene.dup_animator.remove(entity);
                continue;
            }

            let (godot_entity_node, _node_3d) = godot_dcl_scene.ensure_node_3d(entity);
            let animation_player = get_animation_player(godot_entity_node);
            let entry = new_value.unwrap();

            if animation_player.is_none() {
                let value = entry.value.clone();
                if let Some(value) = value {
                    scene.dup_animator.insert(*entity, value);
                } else {
                    scene.dup_animator.remove(entity);
                }
                continue;
            }

            let value = entry.value.clone().unwrap_or_default();
            apply_animator_value(&value, animation_player.unwrap());

            if entry.value.is_none() {
                scene.dup_animator.remove(entity);
            } else {
                scene.dup_animator.insert(*entity, value);
            }
        }
    }
}

pub fn apply_animator_value(value: &PbAnimator, mut animation_player: Gd<AnimationPlayer>) {
    let states = value.states.iter().filter(|s| {
        animation_player
            .get_animation(StringName::from(&s.clip))
            .is_some()
    });

    let mut should_reset_current_animation = false;
    let current_anim_name = animation_player.get_current_animation();

    let (_, req_state) = states.fold((0.0, None), |v, state| {
        if state.should_reset() && current_anim_name.eq(&GString::from(&state.clip)) {
            should_reset_current_animation = true;
        }

        if !state.playing.unwrap_or_default() {
            return v;
        }

        let current_weight = v.0;
        let state_weight = state.weight.unwrap_or(1.0);
        if state_weight > current_weight {
            (state_weight, Some(state))
        } else {
            v
        }
    });

    if let Some(state) = req_state {
        if let Some(mut animation) = animation_player.get_animation(StringName::from(&state.clip)) {
            if state.r#loop() {
                animation.set_loop_mode(LoopMode::LOOP_LINEAR);
            } else {
                animation.set_loop_mode(LoopMode::LOOP_NONE);
            }

            animation_player
                .play_ex()
                .name(StringName::from(&state.clip))
                .custom_speed(state.speed.unwrap_or(1.0))
                .done();

            if should_reset_current_animation {
                animation_player.seek(0.0);
            }
        }
    } else {
        animation_player
            .stop_ex()
            .keep_state(!should_reset_current_animation)
            .done();
    }
}
