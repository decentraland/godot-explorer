use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        SceneId,
    },
    godot_classes::dcl_audio_stream::DclAudioStream,
    scene_runner::scene::{Scene, SceneType},
};
use godot::{engine::AudioStreamGenerator, prelude::*};
enum AudioUpdateMode {
    OnlyChangeValues,
    ChangeAudio,
    FirstSpawnAudio,
}

use crate::av::{stream_processor::AVCommand, video_stream::av_sinks};

pub fn update_audio_stream(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    current_parcel_scene_id: &SceneId,
) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let audio_stream_component = SceneCrdtStateProtoComponents::get_audio_stream(crdt_state);

    if let Some(audio_stream_dirty) = dirty_lww_components.get(&SceneComponentId::AUDIO_STREAM) {
        for entity in audio_stream_dirty {
            let exist_current_node = godot_dcl_scene.get_godot_entity_node(entity).is_some();

            let next_value = if let Some(new_value) = audio_stream_component.get(entity) {
                new_value.value.as_ref()
            } else {
                None
            };

            if let Some(next_value) = next_value {
                let muted_by_current_scene = if let SceneType::Parcel = scene.scene_type {
                    scene.scene_id != *current_parcel_scene_id
                } else {
                    true
                };

                let dcl_volume = next_value.volume.unwrap_or(1.0).clamp(0.0, 1.0);
                let playing = next_value.playing.unwrap_or(true);

                let (godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);
                let update_mode = if let Some((url, _)) = godot_entity_node.audio_stream.as_ref() {
                    if next_value.url != *url {
                        AudioUpdateMode::ChangeAudio
                    } else {
                        AudioUpdateMode::OnlyChangeValues
                    }
                } else {
                    AudioUpdateMode::FirstSpawnAudio
                };

                match update_mode {
                    AudioUpdateMode::OnlyChangeValues => {
                        let audio_stream_data = godot_entity_node
                            .audio_stream
                            .as_ref()
                            .expect("audio_stream_data not found in node");

                        let mut audio_stream_node = node_3d
                            .get_node_or_null("AudioStream".into())
                            .expect("enters on change audio branch but a AudioStream wasn't found there")
                            .try_cast::<DclAudioStream>()
                            .expect("the expected AudioStream wasn't a DclAudioStream");

                        audio_stream_node.bind_mut().set_dcl_volume(dcl_volume);
                        audio_stream_node
                            .bind_mut()
                            .set_muted(muted_by_current_scene);

                        if next_value.playing.unwrap_or(true) {
                            let _ = audio_stream_data.1.command_sender.try_send(AVCommand::Play);
                        } else {
                            let _ = audio_stream_data
                                .1
                                .command_sender
                                .try_send(AVCommand::Pause);
                        }
                    }
                    AudioUpdateMode::ChangeAudio => {
                        if let Some(audio_stream_data) = godot_entity_node.audio_stream.as_ref() {
                            let _ = audio_stream_data
                                .1
                                .command_sender
                                .try_send(AVCommand::Dispose);
                        }

                        let mut audio_stream_node = node_3d.get_node_or_null("AudioStream".into()).expect(
                            "enters on change audio branch but a AudioStream wasn't found there",
                        ).try_cast::<DclAudioStream>().expect("the expected AudioStream wasn't a DclAudioStream");

                        audio_stream_node.bind_mut().set_dcl_volume(dcl_volume);
                        audio_stream_node
                            .bind_mut()
                            .set_muted(muted_by_current_scene);

                        let (_, audio_sink) = av_sinks(
                            next_value.url.clone(),
                            None,
                            audio_stream_node.clone().upcast::<AudioStreamPlayer>(),
                            playing,
                            false,
                            None,
                        );

                        godot_entity_node.audio_stream = Some((next_value.url.clone(), audio_sink));
                    }
                    AudioUpdateMode::FirstSpawnAudio => {
                        let mut audio_stream_node = godot::engine::load::<PackedScene>(
                            "res://src/decentraland_components/audio_stream.tscn",
                        )
                        .instantiate()
                        .unwrap()
                        .cast::<DclAudioStream>();

                        audio_stream_node.set_name("AudioStream".into());

                        let audio_stream_generator = AudioStreamGenerator::new_gd();
                        audio_stream_node.set_stream(audio_stream_generator.upcast());

                        node_3d.add_child(audio_stream_node.clone().upcast());
                        audio_stream_node.play();

                        audio_stream_node.bind_mut().set_dcl_volume(dcl_volume);
                        audio_stream_node
                            .bind_mut()
                            .set_muted(muted_by_current_scene);

                        let (_, audio_sink) = av_sinks(
                            next_value.url.clone(),
                            None,
                            audio_stream_node.clone().upcast::<AudioStreamPlayer>(),
                            playing,
                            false,
                            None,
                        );

                        godot_entity_node.audio_stream = Some((next_value.url.clone(), audio_sink));

                        scene
                            .audio_streams
                            .insert(*entity, audio_stream_node.clone());
                    }
                }
            } else if exist_current_node {
                let Some(node) = godot_dcl_scene.get_godot_entity_node_mut(entity) else {
                    continue;
                };

                if let Some(audio_stream_data) = node.audio_stream.as_ref() {
                    let _ = audio_stream_data
                        .1
                        .command_sender
                        .try_send(AVCommand::Dispose);
                }

                node.audio_stream = None;
            }
        }
    }
}
