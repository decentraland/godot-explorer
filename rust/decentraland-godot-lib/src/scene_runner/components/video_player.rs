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
    godot_classes::dcl_video_player::DclVideoPlayer,
    scene_runner::{
        godot_dcl_scene::VideoPlayerData,
        scene::{Scene, SceneType},
    },
};
use godot::{
    engine::{image::Format, AudioStreamGenerator, Image, ImageTexture},
    prelude::*,
};

enum VideoUpdateMode {
    OnlyChangeValues,
    ChangeVideo,
    FirstSpawnVideo,
}

fn get_local_file_hash_future(
    content_mapping: &Dictionary,
    file_path: &str,
) -> Option<(
    tokio::sync::oneshot::Sender<String>,
    tokio::sync::oneshot::Receiver<String>,
    String,
)> {
    let file_path = file_path.to_lowercase();
    let dict = content_mapping.get("content".to_variant())?;
    let file_hash = Dictionary::from_variant(&dict)
        .get(file_path.to_variant())?
        .to_string();
    let (sx, rx) = tokio::sync::oneshot::channel::<String>();
    Some((sx, rx, file_hash))
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
            let exist_current_node = godot_dcl_scene.get_node(entity).is_some();

            let next_value = if let Some(new_value) = video_player_component.get(*entity) {
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

                let node = godot_dcl_scene.ensure_node_mut(entity);
                let update_mode = if let Some(video_player_data) = node.video_player_data.as_ref() {
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
                        let video_player_data = node
                            .video_player_data
                            .as_ref()
                            .expect("video_player_data not found in node");

                        let mut video_player_node = node
                            .base
                            .get_node("VideoPlayer".into())
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
                                .blocking_send(AVCommand::Play);
                        } else {
                            let _ = video_player_data
                                .video_sink
                                .command_sender
                                .blocking_send(AVCommand::Pause);
                        }

                        let _ = video_player_data
                            .video_sink
                            .command_sender
                            .blocking_send(AVCommand::Repeat(next_value.r#loop.unwrap_or(false)));
                    }
                    VideoUpdateMode::ChangeVideo => {
                        if let Some(video_player_data) = node.video_player_data.as_ref() {
                            let _ = video_player_data
                                .video_sink
                                .command_sender
                                .blocking_send(AVCommand::Dispose);
                        }

                        let mut video_player_node = node.base.get_node("VideoPlayer".into()).expect(
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

                        node.video_player_data = Some(VideoPlayerData {
                            video_sink,
                            audio_sink,
                        });

                        if !file_hash.is_empty() {
                            video_player_node
                                .call_deferred("request_video".into(), &[file_hash.to_variant()]);
                        }
                    }
                    VideoUpdateMode::FirstSpawnVideo => {
                        let image = Image::create(8, 8, false, Format::FORMAT_RGBA8)
                            .expect("couldn't create an video image");
                        let texture = ImageTexture::create_from_image(image)
                            .expect("couldn't create an video image texture");

                        let mut video_player_node = godot::engine::load::<PackedScene>(
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

                        let audio_stream_generator = AudioStreamGenerator::new();
                        video_player_node.set_stream(audio_stream_generator.upcast());

                        node.base.add_child(video_player_node.clone().upcast());
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

                        node.video_player_data = Some(VideoPlayerData {
                            video_sink,
                            audio_sink,
                        });
                        scene
                            .video_players
                            .insert(*entity, video_player_node.clone());

                        if !file_hash.is_empty() {
                            video_player_node
                                .call_deferred("request_video".into(), &[file_hash.to_variant()]);
                        }
                    }
                }
            } else if exist_current_node {
                let Some(node) = godot_dcl_scene.get_node_mut(entity) else {
                    continue;
                };

                if let Some(video_player_data) = node.video_player_data.as_ref() {
                    let _ = video_player_data
                        .video_sink
                        .command_sender
                        .blocking_send(AVCommand::Dispose);
                }

                node.video_player_data = None;
            }
        }
    }
}
