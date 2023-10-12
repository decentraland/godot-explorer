use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_audio_source::DCLAudioSource,
    scene_runner::scene::Scene,
};
use godot::prelude::*;

pub fn update_audio_source(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let audio_source_component = SceneCrdtStateProtoComponents::get_audio_source(crdt_state);

    if let Some(audio_source_dirty) = dirty_lww_components.get(&SceneComponentId::AUDIO_SOURCE) {
        for entity in audio_source_dirty {
            let new_value = audio_source_component.get(*entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let node = godot_dcl_scene.ensure_node_mut(entity);

            let new_value = new_value.value.clone();
            let existing = node
                .base
                .try_get_node_as::<Node>(NodePath::from("AudioSource"));

            if new_value.is_none() {
                if let Some(gltf_node) = existing {
                    node.base.remove_child(gltf_node);
                }
            } else if let Some(new_value) = new_value {
                let mut audio_source = if let Some(audio_source_node) = existing {
                    audio_source_node.cast::<DCLAudioSource>()
                } else {
                    let mut new_audio_source = godot::engine::load::<PackedScene>(
                        "res://src/decentraland_components/audio_source.tscn",
                    )
                    .instantiate()
                    .unwrap()
                    .cast::<DCLAudioSource>();

                    new_audio_source.set_name(GodotString::from("AudioSource"));
                    node.base.add_child(new_audio_source.clone().upcast());
                    new_audio_source
                };

                audio_source.call_deferred("_refresh_data".into(), &[]);

                let mut audio_source = audio_source.bind_mut();
                audio_source.set_dcl_audio_clip_url(GodotString::from(new_value.audio_clip_url));
                audio_source.set_dcl_loop_activated(new_value.r#loop.unwrap_or(false));
                audio_source.set_dcl_playing(new_value.playing.unwrap_or(false));
                audio_source.set_dcl_pitch(new_value.pitch.unwrap_or(1.0));
                audio_source.set_dcl_volume(new_value.volume.unwrap_or(1.0).clamp(0.0, 1.0));
                audio_source.set_dcl_scene_id(scene.scene_id.0)
            }
        }
    }
}
