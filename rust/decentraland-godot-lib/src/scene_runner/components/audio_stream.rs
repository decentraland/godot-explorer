use crate::{
    av::{stream_processor::AVCommand, video_stream::av_sinks},
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        SceneId,
    },
    scene_runner::scene::{Scene, SceneType},
};
use godot::{engine::AudioStreamGenerator, prelude::*};

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
            let new_value = audio_stream_component.get(*entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let node = godot_dcl_scene.ensure_node_mut(entity);

            let new_value = new_value.value.clone();
            // let existing = node
            //     .base
            //     .try_get_node_as::<AnimatableBody3D>(NodePath::from("MeshCollider"));

            if new_value.is_none() {
                if let Some(audio_stream_data) = node.audio_stream.as_ref() {
                    let _ = audio_stream_data
                        .command_sender
                        .blocking_send(AVCommand::Dispose);
                }
            } else if let Some(new_value) = new_value {
                if let Some(audio_stream_data) = node.audio_stream.as_ref() {
                    new_value.volume.unwrap_or(1.0);

                    new_value.playing.unwrap_or(true);

                    if new_value.playing.unwrap_or(true) {
                        let _ = audio_stream_data
                            .command_sender
                            .blocking_send(AVCommand::Play);
                    } else {
                        let _ = audio_stream_data
                            .command_sender
                            .blocking_send(AVCommand::Pause);
                    }
                    // let _ = audio_stream_data
                    //     .command_sender
                    //     .blocking_send(AVCommand::Repeat(new_value..unwrap_or(false)));
                } else {
                    let mut audio_stream_player = godot::engine::load::<PackedScene>(
                        "res://src/decentraland_components/audio_streaming.tscn",
                    )
                    .instantiate()
                    .unwrap()
                    .cast::<AudioStreamPlayer>();
                    let audio_stream_generator = AudioStreamGenerator::new();

                    audio_stream_player.set_stream(audio_stream_generator.upcast());
                    node.base.add_child(audio_stream_player.clone().upcast());
                    audio_stream_player.play();

                    let start_muted = if let SceneType::Parcel = scene.scene_type {
                        &scene.scene_id == current_parcel_scene_id
                    } else {
                        true
                    };

                    if start_muted {
                        audio_stream_player.set_volume_db(0.0);
                    }

                    let (_, audio_sink) = av_sinks(
                        new_value.url.clone(),
                        None,
                        audio_stream_player.clone(),
                        new_value.volume.unwrap_or(1.0),
                        new_value.playing.unwrap_or(true),
                        true,
                    );

                    node.audio_stream = Some(audio_sink);

                    scene
                        .audio_video_players
                        .insert(*entity, audio_stream_player.clone());
                }
            }
        }
    }
}
