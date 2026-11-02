use godot::{
    classes::{AudioStreamPlayer, ImageTexture},
    prelude::*,
};

use super::{AudioSink, VideoSink};

pub fn av_sinks(
    source: String,
    texture: Option<Gd<ImageTexture>>,
    _audio_stream_player: Gd<AudioStreamPlayer>,
    _playing: bool,
    _repeat: bool,
    _wait_for_resource: Option<tokio::sync::oneshot::Receiver<String>>,
) -> (Option<VideoSink>, AudioSink) {
    let (command_sender, _command_receiver) = tokio::sync::mpsc::channel(10);
    let (_stream_data_state_sender, stream_data_state_receiver) = tokio::sync::mpsc::channel(10);

    tracing::warn!("Video playback not available: {}", source);

    (
        texture.map(|texture| VideoSink {
            source,
            command_sender: command_sender.clone(),
            size: (0, 0),
            texture,
            current_time: 0.0,
            length: None,
            rate: None,
            stream_data_state_receiver,
        }),
        AudioSink { command_sender },
    )
}
