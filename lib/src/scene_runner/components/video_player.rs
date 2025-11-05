use crate::{
    content::content_mapping::ContentMappingAndUrlRef,
    dcl::{
        components::{
            proto_components::sdk::components::{PbVideoEvent, VideoState},
            SceneComponentId,
        },
        crdt::{
            grow_only_set::GenericGrowOnlySetComponentOperation,
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        SceneId,
    },
    godot_classes::dcl_video_player::DclVideoPlayer,
    scene_runner::{
        godot_dcl_scene::VideoPlayerData,
        scene::{Scene, SceneType},
    },
};
use godot::{
    classes::{image::Format, AudioStreamGenerator, Image, ImageTexture},
    prelude::*,
};

use crate::av::{
    stream_processor::{AVCommand, StreamStateData},
    video_stream::av_sinks,
};

enum VideoUpdateMode {
    OnlyChangeValues,
    ChangeVideo,
    FirstSpawnVideo,
}

fn get_local_file_hash_future(
    content_mapping: &ContentMappingAndUrlRef,
    file_path: &str,
) -> Option<(
    tokio::sync::oneshot::Sender<String>,
    tokio::sync::oneshot::Receiver<String>,
    String,
)> {
    let file = content_mapping.get_hash(file_path)?.clone();
    let (sx, rx) = tokio::sync::oneshot::channel::<String>();
    Some((sx, rx, file))
}

pub fn update_video_player(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    current_parcel_scene_id: &SceneId,
) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let video_player_component = SceneCrdtStateProtoComponents::get_video_player(crdt_state);

    if let Some(video_player_dirty) = dirty_lww_components.get(&SceneComponentId::VIDEO_PLAYER) {
        for entity in video_player_dirty {
            let exist_current_node = godot_dcl_scene.get_godot_entity_node(entity).is_some();

            let next_value = if let Some(new_value) = video_player_component.get(entity) {
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
                let looping = next_value.r#loop.unwrap_or(false);

                let (godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);
                let update_mode =
                    if let Some(video_player_data) = godot_entity_node.video_player_data.as_ref() {
                        if next_value.src != video_player_data.video_sink.source {
                            VideoUpdateMode::ChangeVideo
                        } else {
                            VideoUpdateMode::OnlyChangeValues
                        }
                    } else {
                        VideoUpdateMode::FirstSpawnVideo
                    };

                match update_mode {
                    VideoUpdateMode::OnlyChangeValues => {
                        let video_player_data = godot_entity_node
                            .video_player_data
                            .as_ref()
                            .expect("video_player_data not found in node");

                        let mut video_player_node: Gd<DclVideoPlayer> = node_3d
                            .get_node_or_null("VideoPlayer".into())
                            .expect("enters on change video branch but a VideoPlayer wasn't found there")
                            .try_cast::<DclVideoPlayer>()
                            .expect("the expected VideoPlayer wasn't a DclVideoPlayer");

                        video_player_node.bind_mut().set_dcl_volume(dcl_volume);
                        video_player_node
                            .bind_mut()
                            .set_muted(muted_by_current_scene);

                        if next_value.playing.unwrap_or(true) {
                            let _ = video_player_data
                                .video_sink
                                .command_sender
                                .try_send(AVCommand::Play);
                        } else {
                            let _ = video_player_data
                                .video_sink
                                .command_sender
                                .try_send(AVCommand::Pause);
                        }

                        let _ = video_player_data
                            .video_sink
                            .command_sender
                            .try_send(AVCommand::Repeat(next_value.r#loop.unwrap_or(false)));
                    }
                    VideoUpdateMode::ChangeVideo => {
                        if let Some(video_player_data) =
                            godot_entity_node.video_player_data.as_ref()
                        {
                            let _ = video_player_data
                                .video_sink
                                .command_sender
                                .try_send(AVCommand::Dispose);
                        }

                        let mut video_player_node = node_3d.get_node_or_null("VideoPlayer".into()).expect(
                            "enters on change video branch but a VideoPlayer wasn't found there",
                        ).try_cast::<DclVideoPlayer>().expect("the expected VideoPlayer wasn't a DclVideoPlayer");

                        video_player_node.bind_mut().set_dcl_volume(dcl_volume);
                        video_player_node
                            .bind_mut()
                            .set_muted(muted_by_current_scene);

                        let texture = video_player_node
                            .bind()
                            .get_dcl_texture()
                            .expect("there should be a texture in the VideoPlayer node");

                        let (wait_for_resource_sender, wait_for_resource_receiver, file_hash) =
                            if let Some(local_scene_resource) =
                                get_local_file_hash_future(&scene.content_mapping, &next_value.src)
                            {
                                (
                                    Some(local_scene_resource.0),
                                    Some(local_scene_resource.1),
                                    local_scene_resource.2,
                                )
                            } else {
                                (None, None, "".to_string())
                            };

                        video_player_node.bind_mut().resolve_resource_sender =
                            wait_for_resource_sender;

                        let (video_sink, audio_sink) = av_sinks(
                            next_value.src.clone(),
                            Some(texture.clone()),
                            video_player_node.clone().upcast::<AudioStreamPlayer>(),
                            playing,
                            looping,
                            wait_for_resource_receiver,
                        );

                        let Some(video_sink) = video_sink else {
                            tracing::error!("couldn't create an video sink");
                            continue;
                        };

                        godot_entity_node.video_player_data = Some(VideoPlayerData {
                            video_sink,
                            audio_sink,
                            timestamp: 0,
                            length: -1.0,
                        });

                        if !file_hash.is_empty() {
                            video_player_node
                                .call_deferred("async_request_video", &[file_hash.to_variant()]);
                        }
                    }
                    VideoUpdateMode::FirstSpawnVideo => {
                        let image = Image::create(8, 8, false, Format::RGBA8)
                            .expect("couldn't create an video image");
                        let texture = ImageTexture::create_from_image(image)
                            .expect("couldn't create an video image texture");

                        let mut video_player_node = godot::tools::load::<PackedScene>(
                            "res://src/decentraland_components/video_player.tscn",
                        )
                        .instantiate()
                        .unwrap()
                        .cast::<DclVideoPlayer>();

                        let (wait_for_resource_sender, wait_for_resource_receiver, file_hash) =
                            if let Some(local_scene_resource) =
                                get_local_file_hash_future(&scene.content_mapping, &next_value.src)
                            {
                                (
                                    Some(local_scene_resource.0),
                                    Some(local_scene_resource.1),
                                    local_scene_resource.2,
                                )
                            } else {
                                (None, None, "".to_string())
                            };

                        video_player_node
                            .bind_mut()
                            .set_dcl_scene_id(scene.scene_id.0);
                        video_player_node.bind_mut().resolve_resource_sender =
                            wait_for_resource_sender;

                        video_player_node.set_name("VideoPlayer".into());

                        video_player_node
                            .bind_mut()
                            .set_dcl_texture(Some(texture.clone()));

                        let audio_stream_generator = AudioStreamGenerator::new_gd();
                        video_player_node.set_stream(audio_stream_generator.upcast());

                        node_3d.add_child(video_player_node.clone().upcast());
                        video_player_node.play();

                        video_player_node.bind_mut().set_dcl_volume(dcl_volume);
                        video_player_node
                            .bind_mut()
                            .set_muted(muted_by_current_scene);

                        let (video_sink, audio_sink) = av_sinks(
                            next_value.src.clone(),
                            Some(texture),
                            video_player_node.clone().upcast::<AudioStreamPlayer>(),
                            playing,
                            looping,
                            wait_for_resource_receiver,
                        );

                        let Some(video_sink) = video_sink else {
                            tracing::error!("couldn't create an video sink");
                            continue;
                        };

                        godot_entity_node.video_player_data = Some(VideoPlayerData {
                            video_sink,
                            audio_sink,
                            timestamp: 0,
                            length: -1.0,
                        });
                        scene
                            .video_players
                            .insert(*entity, video_player_node.clone());

                        if !file_hash.is_empty() {
                            video_player_node
                                .call_deferred("async_request_video", &[file_hash.to_variant()]);
                        }
                    }
                }
            } else if exist_current_node {
                let Some(node) = godot_dcl_scene.get_godot_entity_node_mut(entity) else {
                    continue;
                };

                if let Some(video_player_data) = node.video_player_data.as_ref() {
                    let _ = video_player_data
                        .video_sink
                        .command_sender
                        .try_send(AVCommand::Dispose);
                }

                node.video_player_data = None;
            }
        }
    }

    let video_player_entities = SceneCrdtStateProtoComponents::get_video_player(crdt_state)
        .values
        .keys()
        .copied()
        .collect::<Vec<_>>();
    let video_event_component = SceneCrdtStateProtoComponents::get_video_event_mut(crdt_state);

    for entity_id in video_player_entities {
        if let Some(video_players) = godot_dcl_scene.get_godot_entity_node_mut(&entity_id) {
            if let Some(video_sink) = video_players.video_player_data.as_mut() {
                loop {
                    match video_sink.video_sink.stream_data_state_receiver.try_recv() {
                        Ok(StreamStateData::Ready { length }) => {
                            video_sink.length = length as f32;
                            video_event_component.append(
                                entity_id,
                                PbVideoEvent {
                                    timestamp: video_sink.timestamp,
                                    tick_number: scene.tick_number,
                                    current_offset: 0.0,
                                    video_length: video_sink.length,
                                    state: VideoState::VsReady as i32,
                                },
                            );
                        }
                        Ok(StreamStateData::Playing { position }) => {
                            video_event_component.append(
                                entity_id,
                                PbVideoEvent {
                                    timestamp: video_sink.timestamp,
                                    tick_number: scene.tick_number,
                                    current_offset: position as f32,
                                    video_length: video_sink.length,
                                    state: VideoState::VsPlaying as i32,
                                },
                            );
                        }
                        Ok(StreamStateData::Buffering { position }) => {
                            video_event_component.append(
                                entity_id,
                                PbVideoEvent {
                                    timestamp: video_sink.timestamp,
                                    tick_number: scene.tick_number,
                                    current_offset: position as f32,
                                    video_length: video_sink.length,
                                    state: VideoState::VsBuffering as i32,
                                },
                            );
                        }
                        Ok(StreamStateData::Seeking {}) => {
                            video_event_component.append(
                                entity_id,
                                PbVideoEvent {
                                    timestamp: video_sink.timestamp,
                                    tick_number: scene.tick_number,
                                    current_offset: -1.0,
                                    video_length: video_sink.length,
                                    state: VideoState::VsSeeking as i32,
                                },
                            );
                        }
                        Ok(StreamStateData::Paused { position }) => {
                            video_event_component.append(
                                entity_id,
                                PbVideoEvent {
                                    timestamp: video_sink.timestamp,
                                    tick_number: scene.tick_number,
                                    current_offset: position as f32,
                                    video_length: video_sink.length,
                                    state: VideoState::VsPaused as i32,
                                },
                            );
                        }
                        _ => {
                            break;
                        }
                    }

                    video_sink.timestamp += 1;
                }
            }
        }
    }
}
