use crate::{
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
    godot_classes::dcl_audio_source::{
        DclAudioSource, CLIP_STATE_ERROR, CLIP_STATE_LOADING, CLIP_STATE_READY,
    },
    scene_runner::{
        components::audio_analysis::update_audio_analysis,
        scene::{Scene, SceneType},
    },
};
use godot::prelude::*;

pub fn update_audio_source(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    current_parcel_scene_id: &SceneId,
) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let audio_source_component = SceneCrdtStateProtoComponents::get_audio_source(crdt_state);

    if let Some(audio_source_dirty) = dirty_lww_components.get(&SceneComponentId::AUDIO_SOURCE) {
        for entity in audio_source_dirty {
            let new_value = audio_source_component.get(entity);
            if new_value.is_none() {
                scene.audio_sources.remove(entity);
                continue;
            }

            let new_value = new_value.unwrap();
            let (_godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            let new_value = new_value.value.clone();
            let existing = node_3d.try_get_node_as::<Node>("AudioSource");

            if new_value.is_none() {
                if let Some(mut audio_source_node) = existing {
                    audio_source_node.queue_free();
                    node_3d.remove_child(&audio_source_node);
                }
                scene.audio_sources.remove(entity);
            } else if let Some(new_value) = new_value {
                let mut audio_source = if let Some(audio_source_node) = existing {
                    audio_source_node.cast::<DclAudioSource>()
                } else {
                    let mut new_audio_source = godot::tools::load::<PackedScene>(
                        "res://src/decentraland_components/audio_source.tscn",
                    )
                    .instantiate()
                    .unwrap()
                    .cast::<DclAudioSource>();

                    new_audio_source.set_name("AudioSource");
                    node_3d.add_child(&new_audio_source.clone().upcast::<Node>());
                    scene
                        .audio_sources
                        .insert(*entity, new_audio_source.clone());
                    new_audio_source
                };

                audio_source.call_deferred(
                    "_async_refresh_data",
                    &[new_value.current_time.is_some().to_variant()],
                );

                let mut audio_source = audio_source.bind_mut();
                audio_source.set_dcl_audio_clip_url(GString::from(&new_value.audio_clip_url));
                audio_source.set_dcl_loop_activated(new_value.r#loop.unwrap_or(false));
                audio_source.set_dcl_playing(new_value.playing.unwrap_or(false));
                audio_source.set_dcl_pitch(new_value.pitch.unwrap_or(1.0));
                audio_source.set_dcl_volume(new_value.volume.unwrap_or(1.0).clamp(0.0, 1.0));
                audio_source.set_dcl_current_time(new_value.current_time.unwrap_or(0.0));
                audio_source.set_dcl_global(new_value.global.unwrap_or(false));
                audio_source.set_dcl_scene_id(scene.scene_id.0);

                let dcl_enable = if let SceneType::Parcel = scene.scene_type {
                    &scene.scene_id == current_parcel_scene_id
                } else {
                    true
                };
                audio_source.set_dcl_enable(dcl_enable);
            }
        }
    }

    poll_audio_events(scene, crdt_state);
    update_audio_analysis(scene, crdt_state);
}

/// Mirrors Unity's `GetAudioSourceState`.
fn media_state(source: &DclAudioSource) -> MediaState {
    match source.get_dcl_clip_state() {
        CLIP_STATE_LOADING => MediaState::MsLoading,
        CLIP_STATE_ERROR => MediaState::MsError,
        CLIP_STATE_READY => {
            // Looping is emulated by replaying from the `finished` signal, so
            // `is_playing` reads false for a frame at every loop point.
            let looping = source.get_dcl_loop_activated() && source.get_dcl_playing();
            if source.is_clip_playing() || looping {
                MediaState::MsPlaying
            } else {
                MediaState::MsReady
            }
        }
        _ => MediaState::MsNone,
    }
}

/// Append a `PBAudioEvent` for every source whose playback state changed, and
/// report clips that ended on their own back to the scene as `playing = false`.
fn poll_audio_events(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    if scene.audio_sources.is_empty() {
        return;
    }

    let tick_number = scene.tick_number;
    let mut events: Vec<(SceneEntityId, MediaState)> = Vec::new();
    let mut natural_finishes: Vec<SceneEntityId> = Vec::new();

    for (entity_id, audio_source_node) in scene.audio_sources.iter_mut() {
        let mut source = audio_source_node.bind_mut();
        let state = media_state(&source);
        if state as i32 == source.last_media_state {
            continue;
        }

        let was_playing = source.last_media_state == MediaState::MsPlaying as i32;
        source.last_media_state = state as i32;
        events.push((*entity_id, state));

        // A non-looping clip that reached its end while the scene still believes
        // it is playing: report the stop so `AudioSource.playing` reads the same
        // on both clients.
        if was_playing && state == MediaState::MsReady && !source.get_dcl_loop_activated() {
            source.set_dcl_playing(false);
            natural_finishes.push(*entity_id);
        }
    }

    if !events.is_empty() {
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

    if !natural_finishes.is_empty() {
        let audio_source_component =
            SceneCrdtStateProtoComponents::get_audio_source_mut(crdt_state);
        for entity_id in natural_finishes {
            let Some(mut value) = audio_source_component
                .get(&entity_id)
                .and_then(|entry| entry.value.clone())
            else {
                continue;
            };
            if value.playing != Some(true) {
                continue;
            }
            value.playing = Some(false);
            audio_source_component.put(entity_id, Some(value));
        }
    }
}
