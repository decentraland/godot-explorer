use crate::{
    av::{
        audio_context,
        video_context::{VideoData, VideoInfo},
        video_stream::av_sinks,
    },
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};
use ffmpeg_next::codec::audio;
use godot::{
    engine::{image::Format, Image, ImageTexture},
    prelude::*,
};
use tokio::sync::mpsc::error::TryRecvError;

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
                //     if let Some(video_player_node) = existing {
                //         node.base.remove_child(video_player_node.upcast());
                //     }
            } else if let Some(new_value) = new_value {
                // let mut image = Image::new_fill(
                //     bevy::render::render_resource::Extent3d {
                //         width: 8,
                //         height: 8,
                //         depth_or_array_layers: 1,
                //     },
                //     TextureDimension::D2,
                //     &Color::PINK.as_rgba_u32().to_le_bytes(),
                //     TextureFormat::Rgba8UnormSrgb,
                // );
                // image.texture_descriptor.usage =
                //     TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING;
                // let image_handle = images.add(image);

                let image = Image::create(8, 8, false, Format::FORMAT_RGBA8)
                    .expect("couldn't create an video image");
                let mut texture = ImageTexture::create_from_image(image)
                    .expect("couldn't create an video image texture");

                let (video_sink, audio_sink) = av_sinks(
                    new_value.src.clone(),
                    texture,
                    new_value.volume.unwrap_or(1.0),
                    new_value.playing.unwrap_or(true),
                    new_value.r#loop.unwrap_or(false),
                );
                node.video_player_data = Some((video_sink, audio_sink));
            }
        }
    }

    for (entity, entry) in video_player_component.values.iter() {
        let video_player = scene
            .godot_dcl_scene
            .ensure_node_mut(entity)
            .video_player_data
            .as_mut();

        if let Some((video_sink, audio_sink)) = video_player {
            let mut last_frame_received = None;
            audio_sink.sound_data.try_recv();
            match video_sink.video_receiver.try_recv() {
                Ok(VideoData::Info(VideoInfo {
                    width,
                    height,
                    rate,
                    length,
                })) => {
                    tracing::trace!("godotandroid got video info");

                    // images.get_mut(&video_sink.image).unwrap().resize(Extent3d {
                    //     width,
                    //     height,
                    //     depth_or_array_layers: 1,
                    // });

                    let img =
                        Image::create(width as i32, height as i32, false, Format::FORMAT_RGBA8);
                    if let Some(img) = img {
                        video_sink.tex.set_image(img);
                    }

                    video_sink.size = (width, height);
                    video_sink.length = Some(length);
                    video_sink.rate = Some(rate);
                }
                Ok(VideoData::Frame(frame, time)) => {
                    tracing::info!("godotandroid got video frame");
                    last_frame_received = Some(frame);
                    video_sink.current_time = time;
                }
                Err(err) => {
                    if let TryRecvError::Empty = err {
                    } else {
                        tracing::info!("godotandroid got error {:?}", err);
                    }
                }
            }

            if let Some(frame) = last_frame_received {
                let data = PackedByteArray::from(frame.data(0));

                let img = Image::create_from_data(
                    video_sink.size.0 as i32,
                    video_sink.size.1 as i32,
                    false,
                    Format::FORMAT_RGBA8,
                    data,
                );

                if let Some(img) = img {
                    video_sink.tex.update(img);
                    tracing::trace!("godotandroid set frame on {:?}", video_sink.tex);
                } else {
                    tracing::error!("godotandroid failed to create image");
                }
            }
        }
    }
}
