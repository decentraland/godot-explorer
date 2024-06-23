use ffmpeg_next::format::input;
use godot::{
    engine::ImageTexture,
    prelude::{AudioStreamPlayer, Gd, InstanceId},
};
use tracing::{debug, warn};

use super::{
    audio_context::{AudioContext, AudioError, AudioSink},
    ffmpeg_util::InputWrapper,
    stream_processor::{process_streams, AVCommand, StreamStateData},
    video_context::{VideoContext, VideoError},
};

#[derive(Debug)]
pub enum AudioDecoderError {
    StreamClosed,
    Other(String),
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
    audio_stream_player: Gd<AudioStreamPlayer>,
    playing: bool,
    repeat: bool,
    wait_for_resource: Option<tokio::sync::oneshot::Receiver<String>>,
) -> (Option<VideoSink>, AudioSink) {
    let (command_sender, command_receiver) = tokio::sync::mpsc::channel(10);
    let (stream_data_state_sender, stream_data_state_receiver) = tokio::sync::mpsc::channel(10);

    spawn_av_thread(
        command_receiver,
        source.clone(),
        texture.clone(),
        audio_stream_player,
        wait_for_resource,
        stream_data_state_sender,
    );

    if playing {
        command_sender.blocking_send(AVCommand::Play).unwrap();
    }
    command_sender
        .blocking_send(AVCommand::Repeat(repeat))
        .unwrap();

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

pub fn spawn_av_thread(
    commands: tokio::sync::mpsc::Receiver<AVCommand>,
    path: String,
    tex: Option<Gd<ImageTexture>>,
    audio_stream_player: Gd<AudioStreamPlayer>,
    wait_for_resource: Option<tokio::sync::oneshot::Receiver<String>>,
    sink: tokio::sync::mpsc::Sender<StreamStateData>,
) {
    let video_instance_id = tex.map(|value| value.instance_id());
    let audio_stream_player_instance_id = audio_stream_player.instance_id();
    std::thread::Builder::new()
        .name("av thread".to_string())
        .spawn(move || {
            av_thread(
                commands,
                path,
                video_instance_id,
                audio_stream_player_instance_id,
                wait_for_resource,
                sink,
            )
        })
        .unwrap();
}

fn av_thread(
    commands: tokio::sync::mpsc::Receiver<AVCommand>,
    path: String,
    tex: Option<InstanceId>,
    audio_stream: InstanceId,
    wait_for_resource: Option<tokio::sync::oneshot::Receiver<String>>,
    sink: tokio::sync::mpsc::Sender<StreamStateData>,
) {
    let tex = tex.map(Gd::from_instance_id);
    if let Err(error) = av_thread_inner(commands, path, tex, audio_stream, wait_for_resource, sink)
    {
        warn!("av error: {error}");
    } else {
        debug!("av closed");
    }
}

pub fn av_thread_inner(
    commands: tokio::sync::mpsc::Receiver<AVCommand>,
    mut path: String,
    texture: Option<Gd<ImageTexture>>,
    audio_stream_player_instance_id: InstanceId,
    wait_for_resource: Option<tokio::sync::oneshot::Receiver<String>>,
    sink: tokio::sync::mpsc::Sender<StreamStateData>,
) -> Result<(), String> {
    if let Some(wait_for_resource_receiver) = wait_for_resource {
        match wait_for_resource_receiver.blocking_recv() {
            Ok(file_source) => {
                if file_source.is_empty() {
                    return Err(format!("failed to get resource: {:?}", path));
                }
                path = file_source;
            }
            Err(err) => return Err(format!("failed to get resource: {:?}", err)),
        }
    }

    let input_context = input(&path).map_err(|e| format!("{:?} on line {}", e, line!()))?;

    // try and get a video context
    let video_context: Option<VideoContext> = {
        if let Some(texture) = texture {
            match VideoContext::init(&input_context, texture) {
                Ok(vc) => Some(vc),
                Err(VideoError::BadPixelFormat) => {
                    return Err("bad pixel format".to_string());
                }
                Err(VideoError::NoStream) => None,
                Err(VideoError::Failed(ffmpeg_err)) => return Err(ffmpeg_err.to_string()),
                Err(VideoError::ChannelClosed) => return Ok(()),
            }
        } else {
            None
        }
    };

    // try and get an audio context
    let audio_context: Option<AudioContext> =
        match AudioContext::init(&input_context, audio_stream_player_instance_id) {
            Ok(ac) => Some(ac),
            Err(AudioError::NoStream) => None,
            Err(AudioError::Failed(ffmpeg_err)) => return Err(ffmpeg_err.to_string()),
        };

    if video_context.is_none() && audio_context.is_none() {
        // no data
    }

    let input_context = InputWrapper::new(input_context, path);

    match (video_context, audio_context) {
        (None, None) => Ok(()),
        (None, Some(mut ac)) => process_streams(input_context, &mut [&mut ac], commands, sink)
            .map_err(|e| format!("{:?} on line {}", e, line!())),
        (Some(mut vc), None) => process_streams(input_context, &mut [&mut vc], commands, sink)
            .map_err(|e| format!("{:?} on line {}", e, line!())),
        (Some(mut vc), Some(mut ac)) => {
            process_streams(input_context, &mut [&mut vc, &mut ac], commands, sink)
                .map_err(|e| format!("{:?} on line {}", e, line!()))
        }
    }
}
