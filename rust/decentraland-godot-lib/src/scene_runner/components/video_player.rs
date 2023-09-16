use crate::{
    av::{stream_processor::AVCommand, video_stream::av_sinks},
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::{godot_dcl_scene::VideoPlayerData, scene::Scene},
};
use godot::{
    engine::{image::Format, AudioStreamGenerator, Image, ImageTexture},
    prelude::*,
};

pub fn update_video_player(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let video_player_component = SceneCrdtStateProtoComponents::get_video_player(crdt_state);

    if let Some(video_player_dirty) = dirty_lww_components.get(&SceneComponentId::VIDEO_PLAYER) {
        for entity in video_player_dirty {
            let new_value = video_player_component.get(*entity);
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
                if let Some(video_player_data) = node.video_player_data.as_ref() {
                    let _ = video_player_data
                        .video_sink
                        .command_sender
                        .blocking_send(AVCommand::Dispose);
                }
            } else if let Some(new_value) = new_value {
                if let Some(video_player_data) = node.video_player_data.as_ref() {
                    new_value.volume.unwrap_or(1.0);

                    new_value.playing.unwrap_or(true);

                    if new_value.playing.unwrap_or(true) {
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
                        .blocking_send(AVCommand::Repeat(new_value.r#loop.unwrap_or(false)));
                } else {
                    let image = Image::create(8, 8, false, Format::FORMAT_RGBA8)
                        .expect("couldn't create an video image");
                    let texture = ImageTexture::create_from_image(image)
                        .expect("couldn't create an video image texture");

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

                    let (video_sink, audio_sink) = av_sinks(
                        new_value.src.clone(),
                        texture,
                        audio_stream_player.clone(),
                        new_value.volume.unwrap_or(1.0),
                        new_value.playing.unwrap_or(true),
                        new_value.r#loop.unwrap_or(false),
                    );
                    node.video_player_data = Some(VideoPlayerData {
                        video_sink,
                        audio_sink,
                    });
                }
            }
        }
    }
}
