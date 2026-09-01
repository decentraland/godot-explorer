use crate::{
    av::backend::BackendType,
    dcl::{
        components::{
            proto_components::sdk::components::{common::MediaState, PbAudioEvent},
            SceneComponentId, SceneEntityId,
        },
        crdt::{
            grow_only_set::GenericGrowOnlySetComponentOperation,
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        SceneId,
    },
    godot_classes::dcl_audio_stream::{
        DclAudioStream, STREAM_STATE_ERROR, STREAM_STATE_PAUSED, STREAM_STATE_PLAYING,
    },
    scene_runner::scene::{Scene, SceneType},
};
use godot::prelude::*;

enum AudioUpdateMode {
    OnlyChangeValues,
    ChangeAudio,
    FirstSpawnAudio,
}

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
                let update_mode = if let Some(url) = godot_entity_node.audio_stream.as_ref() {
                    if next_value.url != *url {
                        AudioUpdateMode::ChangeAudio
                    } else {
                        AudioUpdateMode::OnlyChangeValues
                    }
                } else {
                    AudioUpdateMode::FirstSpawnAudio
                };

                let mut audio_stream_node = match update_mode {
                    AudioUpdateMode::FirstSpawnAudio => {
                        let mut node = godot::tools::load::<PackedScene>(
                            "res://src/decentraland_components/audio_stream.tscn",
                        )
                        .instantiate()
                        .expect("Failed to instantiate audio_stream.tscn")
                        .cast::<DclAudioStream>();

                        node.set_name("AudioStream");
                        node.bind_mut().set_dcl_scene_id(scene.scene_id.0);
                        node_3d.add_child(&node.clone().upcast::<Node>());
                        scene.audio_streams.insert(*entity, node.clone());
                        node
                    }
                    _ => node_3d
                        .get_node_or_null("AudioStream")
                        .expect("audio_stream node missing for an entity that already had one")
                        .try_cast::<DclAudioStream>()
                        .expect("the expected AudioStream wasn't a DclAudioStream"),
                };

                audio_stream_node.bind_mut().set_volume(dcl_volume);
                audio_stream_node
                    .bind_mut()
                    .set_muted(muted_by_current_scene);

                match update_mode {
                    AudioUpdateMode::OnlyChangeValues => {
                        if playing {
                            audio_stream_node.bind_mut().backend_play();
                        } else {
                            audio_stream_node.bind_mut().backend_pause();
                        }
                    }
                    _ => {
                        audio_stream_node.bind_mut().backend_dispose();

                        // Streams have no LiveKit variant: `from_source` only ever
                        // returns the platform's native player or Noop for them.
                        let backend_type = BackendType::from_source(&next_value.url);
                        audio_stream_node.bind_mut().init_backend(
                            backend_type.to_gd_int(),
                            next_value.url.to_godot(),
                            playing,
                        );
                        godot_entity_node.audio_stream = Some(next_value.url.clone());
                    }
                }
            } else if exist_current_node {
                if let Some(mut node) = scene.audio_streams.remove(entity) {
                    node.bind_mut().backend_dispose();
                    node.queue_free();
                }

                let Some(node) = godot_dcl_scene.get_godot_entity_node_mut(entity) else {
                    continue;
                };
                node.audio_stream = None;
            }
        }
    }

    poll_audio_stream_events(scene, crdt_state);
}

/// Mirrors Unity's `GetAudioStreamState`, which is a direct cast of the media
/// player state. Unity's state machine never produces `MS_LOADING` or
/// `MS_READY` for a stream — a stream that is loading or finished reads
/// `MS_NONE` — so neither is emitted here.
fn media_state(stream_state: i32) -> MediaState {
    match stream_state {
        STREAM_STATE_PLAYING => MediaState::MsPlaying,
        STREAM_STATE_PAUSED => MediaState::MsPaused,
        STREAM_STATE_ERROR => MediaState::MsError,
        _ => MediaState::MsNone,
    }
}

fn poll_audio_stream_events(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    if scene.audio_streams.is_empty() {
        return;
    }

    let tick_number = scene.tick_number;
    let mut events: Vec<(SceneEntityId, MediaState)> = Vec::new();

    for (entity_id, audio_stream_node) in scene.audio_streams.iter_mut() {
        let mut stream = audio_stream_node.bind_mut();
        let state = media_state(stream.get_stream_state());
        if state as i32 == stream.last_media_state {
            continue;
        }

        stream.last_media_state = state as i32;
        events.push((*entity_id, state));
    }

    if events.is_empty() {
        return;
    }

    let audio_event_component = SceneCrdtStateProtoComponents::get_audio_event_mut(crdt_state);
    for (entity_id, state) in events {
        audio_event_component.append(
            entity_id,
            PbAudioEvent {
                state: state as i32,
                timestamp: tick_number,
            },
        );
    }
}
