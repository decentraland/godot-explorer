use ffmpeg_next::format::input;
use godot::{
    engine::ImageTexture,
    prelude::{AudioStreamPlayer, Gd, InstanceId, Share},
};
use tracing::{debug, warn};

use super::{
    audio_context::{AudioContext, AudioError, AudioSink},
    ffmpeg_util::InputWrapper,
    stream_processor::{process_streams, AVCommand},
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
    pub tex: Gd<ImageTexture>,
    pub size: (u32, u32),
    pub current_time: f64,
    pub length: Option<f64>,
    pub rate: Option<f64>,
}

pub fn av_sinks(
    source: String,
    tex: Gd<ImageTexture>,
    audio_stream_player: Gd<AudioStreamPlayer>,
    volume: f32,
    playing: bool,
    repeat: bool,
) -> (VideoSink, AudioSink) {
    let (command_sender, command_receiver) = tokio::sync::mpsc::channel(10);

    spawn_av_thread(
        command_receiver,
        // video_sender,
        // audio_sender,
        source.clone(),
        tex.share(),
        audio_stream_player,
    );

    if playing {
        command_sender.blocking_send(AVCommand::Play).unwrap();
    }
    command_sender
        .blocking_send(AVCommand::Repeat(repeat))
        .unwrap();

    (
        VideoSink {
            source,
            command_sender: command_sender.clone(),
            size: (0, 0),
            tex,
            current_time: 0.0,
            length: None,
            rate: None,
        },
        AudioSink {
            volume,
            command_sender,
        },
    )
}

pub fn spawn_av_thread(
    commands: tokio::sync::mpsc::Receiver<AVCommand>,
    path: String,
    tex: Gd<ImageTexture>,
    audio_stream_player: Gd<AudioStreamPlayer>,
) {
    let video_instance_id = tex.instance_id();
    let audio_stream_player_instance_id = audio_stream_player.instance_id();
    std::thread::Builder::new()
        .name("av thread".to_string())
        .spawn(move || {
            av_thread(
                commands,
                path,
                video_instance_id,
                audio_stream_player_instance_id,
            )
        })
        .unwrap();
}

fn av_thread(
    commands: tokio::sync::mpsc::Receiver<AVCommand>,
    // frames: tokio::sync::mpsc::Sender<VideoData>,
    // audio: tokio::sync::mpsc::Sender<StreamingSoundData<AudioDecoderError>>,
    path: String,
    tex: InstanceId,
    audio_stream: InstanceId,
) {
    let tex = Gd::from_instance_id(tex);
    let audio_stream_player: Gd<AudioStreamPlayer> = Gd::from_instance_id(audio_stream);
    if let Err(error) = av_thread_inner(commands, path, tex, audio_stream_player) {
        warn!("av error: {error}");
    } else {
        debug!("av closed");
    }
}

pub fn av_thread_inner(
    commands: tokio::sync::mpsc::Receiver<AVCommand>,
    path: String,
    tex: Gd<ImageTexture>,
    audio_stream_player: Gd<AudioStreamPlayer>,
) -> Result<(), String> {
    let input_context = input(&path).map_err(|e| format!("{:?} on line {}", e, line!()))?;

    // try and get a video context
    let video_context: Option<VideoContext> = {
        match VideoContext::init(&input_context, tex) {
            Ok(vc) => Some(vc),
            Err(VideoError::BadPixelFormat) => {
                return Err("bad pixel format".to_string());
            }
            Err(VideoError::NoStream) => None,
            Err(VideoError::Failed(ffmpeg_err)) => return Err(ffmpeg_err.to_string()),
            Err(VideoError::ChannelClosed) => return Ok(()),
        }
    };

    // try and get an audio context
    let audio_context: Option<AudioContext> =
        match AudioContext::init(&input_context, audio_stream_player) {
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
        (None, Some(mut ac)) => process_streams(input_context, &mut [&mut ac], commands)
            .map_err(|e| format!("{:?} on line {}", e, line!())),
        (Some(mut vc), None) => process_streams(input_context, &mut [&mut vc], commands)
            .map_err(|e| format!("{:?} on line {}", e, line!())),
        (Some(mut vc), Some(mut ac)) => {
            process_streams(input_context, &mut [&mut vc, &mut ac], commands)
                .map_err(|e| format!("{:?} on line {}", e, line!()))
        }
    }
}
