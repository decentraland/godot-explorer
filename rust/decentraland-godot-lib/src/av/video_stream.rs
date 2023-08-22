use ffmpeg_next::format::input;
use godot::{
    engine::{Image, ImageTexture},
    prelude::{Gd, InstanceId, Share},
};
use kira::sound::streaming::StreamingSoundData;
use tracing::{debug, warn};

use super::{
    audio_context::{AudioContext, AudioError},
    audio_sink::AudioSink,
    ffmpeg_util::InputWrapper,
    stream_processor::{process_streams, AVCommand},
    video_context::{VideoContext, VideoData, VideoError},
};

#[derive(Debug)]
pub enum AudioDecoderError {
    StreamClosed,
    Other(String),
}

pub struct VideoSink {
    pub source: String,
    pub command_sender: tokio::sync::mpsc::Sender<AVCommand>,
    pub video_receiver: tokio::sync::mpsc::Receiver<VideoData>,
    pub tex: Gd<ImageTexture>,
    pub size: (u32, u32),
    pub current_time: f64,
    pub length: Option<f64>,
    pub rate: Option<f64>,
}

pub fn av_sinks(
    source: String,
    tex: Gd<ImageTexture>,
    volume: f32,
    playing: bool,
    repeat: bool,
) -> (VideoSink, AudioSink) {
    let (command_sender, command_receiver) = tokio::sync::mpsc::channel(10);
    let (video_sender, video_receiver) = tokio::sync::mpsc::channel(10);
    let (audio_sender, audio_receiver) = tokio::sync::mpsc::channel(1);

    spawn_av_thread(
        command_receiver,
        video_sender,
        audio_sender,
        source.clone(),
        tex.share(),
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
            video_receiver,
            size: (0, 0),
            tex,
            current_time: 0.0,
            length: None,
            rate: None,
        },
        AudioSink::new(volume, command_sender, audio_receiver),
    )
}

pub fn spawn_av_thread(
    commands: tokio::sync::mpsc::Receiver<AVCommand>,
    frames: tokio::sync::mpsc::Sender<VideoData>,
    audio: tokio::sync::mpsc::Sender<StreamingSoundData<AudioDecoderError>>,
    path: String,
    tex: Gd<ImageTexture>,
) {
    let instance_id = tex.instance_id().clone();
    std::thread::Builder::new()
        .name(format!("av thread"))
        .spawn(move || av_thread(commands, frames, audio, path, instance_id))
        .unwrap();
}

fn av_thread(
    commands: tokio::sync::mpsc::Receiver<AVCommand>,
    frames: tokio::sync::mpsc::Sender<VideoData>,
    audio: tokio::sync::mpsc::Sender<StreamingSoundData<AudioDecoderError>>,
    path: String,
    tex: InstanceId,
) {
    let tex = Gd::from_instance_id(tex);
    if let Err(error) = av_thread_inner(commands, frames, audio, path, tex) {
        warn!("av error: {error}");
    } else {
        debug!("av closed");
    }
}

pub fn av_thread_inner(
    commands: tokio::sync::mpsc::Receiver<AVCommand>,
    video: tokio::sync::mpsc::Sender<VideoData>,
    audio: tokio::sync::mpsc::Sender<StreamingSoundData<AudioDecoderError>>,
    path: String,
    tex: Gd<ImageTexture>,
) -> Result<(), String> {
    let mut input_context = input(&path).map_err(|e| format!("{:?} on line {}", e, line!()))?;

    // try and get a video context
    let video_context: Option<VideoContext> = {
        match VideoContext::init(&input_context, video.clone(), tex) {
            Ok(vc) => Some(vc),
            Err(VideoError::BadPixelFormat) => {
                // try to workaround ffmpeg remote streaming issue by downloading the file
                // debug!("failed to determine pixel format - downloading ...");
                // let mut resp =
                //     isahc::get(&path).map_err(|e| format!("{:?} on line {}", e, line!()))?;
                // let data = resp
                //     .bytes()
                //     .map_err(|e| format!("{:?} on line {}", e, line!()))?;
                // let local_folder = PathBuf::from("assets/video_downloads");
                // std::fs::create_dir_all(&local_folder)
                //     .map_err(|e| format!("{:?} on line {}", e, line!()))?;

                // // TODO
                // // let local_path = local_folder.join(Path::new(urlencoding::encode(&path).as_ref()));
                // // std::fs::write(&local_path, data)?;
                // // path = local_path.to_string_lossy().to_string();

                // input_context = input(&path).map_err(|e| format!("{:?} on line {}", e, line!()))?;
                // let context = VideoContext::init(&input_context, video)
                //     .map_err(|e| format!("{:?} on line {}", e, line!()))?;
                // Some(context)

                return Err("bad pixel format".to_string());
            }
            Err(VideoError::NoStream) => None,
            Err(VideoError::Failed(ffmpeg_err)) => return Err(ffmpeg_err.to_string()),
            Err(VideoError::ChannelClosed) => return Ok(()),
        }
    };

    // try and get an audio context
    let audio_context: Option<AudioContext> = match AudioContext::init(&input_context, audio) {
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
