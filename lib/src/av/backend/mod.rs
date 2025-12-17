use godot::{engine::ImageTexture, prelude::*};

use super::stream_processor::{AVCommand, StreamStateData};

pub struct AudioSink {
    pub command_sender: tokio::sync::mpsc::Sender<AVCommand>,
}

pub struct VideoSink {
    pub source: String,
    pub command_sender: tokio::sync::mpsc::Sender<AVCommand>,
    pub texture: Gd<ImageTexture>,
    pub size: (u32, u32),
    pub current_time: f64,
    pub length: Option<f64>,
    pub rate: Option<f64>,
    pub stream_data_state_receiver: tokio::sync::mpsc::Receiver<StreamStateData>,
}

pub fn av_sinks(
    source: String,
    texture: Option<Gd<ImageTexture>>,
    audio_stream_player: Gd<godot::prelude::AudioStreamPlayer>,
    playing: bool,
    repeat: bool,
    wait_for_resource: Option<tokio::sync::oneshot::Receiver<String>>,
) -> (Option<VideoSink>, AudioSink) {
    noop::av_sinks(
        source,
        texture,
        audio_stream_player,
        playing,
        repeat,
        wait_for_resource,
    )
}

pub mod noop;
