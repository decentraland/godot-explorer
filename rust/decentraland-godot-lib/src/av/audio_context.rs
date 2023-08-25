use std::collections::VecDeque;

use ffmpeg_next::ffi::AVSampleFormat;
use ffmpeg_next::{decoder, format::context::Input, media::Type, util::frame, Packet};
use godot::prelude::{AudioStreamPlayer, Gd, PackedVector2Array, ToVariant, Vector2};
use thiserror::Error;
use tracing::{debug, error};

use super::stream_processor::AVCommand;
use super::stream_processor::FfmpegContext;

pub struct AudioSink {
    pub volume: f32,
    pub command_sender: tokio::sync::mpsc::Sender<AVCommand>,
}

trait SampleFormatHelper {
    fn is_planar(&self) -> bool;
    fn bytes(&self) -> usize;
    fn to_f32(&self, data: &[u8]) -> f32;
}

impl SampleFormatHelper for AVSampleFormat {
    fn is_planar(&self) -> bool {
        match self {
            AVSampleFormat::AV_SAMPLE_FMT_NONE
            | AVSampleFormat::AV_SAMPLE_FMT_U8
            | AVSampleFormat::AV_SAMPLE_FMT_S16
            | AVSampleFormat::AV_SAMPLE_FMT_S32
            | AVSampleFormat::AV_SAMPLE_FMT_FLT
            | AVSampleFormat::AV_SAMPLE_FMT_DBL
            | AVSampleFormat::AV_SAMPLE_FMT_NB => false,
            AVSampleFormat::AV_SAMPLE_FMT_U8P
            | AVSampleFormat::AV_SAMPLE_FMT_S16P
            | AVSampleFormat::AV_SAMPLE_FMT_S32P
            | AVSampleFormat::AV_SAMPLE_FMT_FLTP
            | AVSampleFormat::AV_SAMPLE_FMT_DBLP
            | AVSampleFormat::AV_SAMPLE_FMT_S64
            | AVSampleFormat::AV_SAMPLE_FMT_S64P => true,
        }
    }

    fn bytes(&self) -> usize {
        match self {
            AVSampleFormat::AV_SAMPLE_FMT_NONE => panic!(),
            AVSampleFormat::AV_SAMPLE_FMT_U8 => 1,
            AVSampleFormat::AV_SAMPLE_FMT_S16 => 2,
            AVSampleFormat::AV_SAMPLE_FMT_S32 => 4,
            AVSampleFormat::AV_SAMPLE_FMT_FLT => 4,
            AVSampleFormat::AV_SAMPLE_FMT_DBL => 8,
            AVSampleFormat::AV_SAMPLE_FMT_U8P => 1,
            AVSampleFormat::AV_SAMPLE_FMT_S16P => 2,
            AVSampleFormat::AV_SAMPLE_FMT_S32P => 4,
            AVSampleFormat::AV_SAMPLE_FMT_FLTP => 4,
            AVSampleFormat::AV_SAMPLE_FMT_DBLP => 8,
            AVSampleFormat::AV_SAMPLE_FMT_S64 => 8,
            AVSampleFormat::AV_SAMPLE_FMT_S64P => 8,
            AVSampleFormat::AV_SAMPLE_FMT_NB => panic!(),
        }
    }

    // TODO lots of optimization potential. branch per read?!
    fn to_f32(&self, data: &[u8]) -> f32 {
        assert!(data.len() == self.bytes());

        match self {
            AVSampleFormat::AV_SAMPLE_FMT_U8 | AVSampleFormat::AV_SAMPLE_FMT_U8P => {
                (data[0] as f32 / 255.0 - 0.5) * 2.0
            }
            AVSampleFormat::AV_SAMPLE_FMT_S16 | AVSampleFormat::AV_SAMPLE_FMT_S16P => unsafe {
                std::slice::from_raw_parts(data.as_ptr() as *const i16, 1)[0] as f32
                    / i16::MAX as f32
            },
            AVSampleFormat::AV_SAMPLE_FMT_S32 | AVSampleFormat::AV_SAMPLE_FMT_S32P => unsafe {
                std::slice::from_raw_parts(data.as_ptr() as *const i32, 1)[0] as f32
                    / i32::MAX as f32
            },
            AVSampleFormat::AV_SAMPLE_FMT_FLT | AVSampleFormat::AV_SAMPLE_FMT_FLTP => unsafe {
                std::slice::from_raw_parts(data.as_ptr() as *const f32, 1)[0]
            },
            AVSampleFormat::AV_SAMPLE_FMT_DBL | AVSampleFormat::AV_SAMPLE_FMT_DBLP => unsafe {
                std::slice::from_raw_parts(data.as_ptr() as *const f64, 1)[0] as f32
            },
            AVSampleFormat::AV_SAMPLE_FMT_S64 | AVSampleFormat::AV_SAMPLE_FMT_S64P => unsafe {
                std::slice::from_raw_parts(data.as_ptr() as *const i64, 1)[0] as f32
                    / i64::MAX as f32
            },
            AVSampleFormat::AV_SAMPLE_FMT_NONE | AVSampleFormat::AV_SAMPLE_FMT_NB => panic!(),
        }
    }
}

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("No Stream")]
    NoStream,
    #[error("Failed: {0}")]
    Failed(ffmpeg_next::Error),
}

pub struct AudioContext {
    stream_index: usize,
    decoder: decoder::Audio,

    rate: f64,

    buffer: VecDeque<frame::audio::Audio>,

    current_frame: usize,
    start_frame: usize,

    audio_stream_player: Gd<AudioStreamPlayer>,
    format: AVSampleFormat,
    frame_size: usize,
    channels: usize,
}

impl AudioContext {
    pub fn init(
        input_context: &Input,
        mut audio_stream_player: Gd<AudioStreamPlayer>,
    ) -> Result<Self, AudioError> {
        let input_stream = input_context
            .streams()
            .best(Type::Audio)
            .ok_or(AudioError::NoStream)?;

        let stream_index = input_stream.index();

        let context_decoder =
            ffmpeg_next::codec::context::Context::from_parameters(input_stream.parameters())
                .map_err(AudioError::Failed)?;

        let decoder = context_decoder
            .decoder()
            .audio()
            .map_err(AudioError::Failed)?;

        debug!(
            "decoder says: bitrate: {}, frame_rate: {:?}, channels: {}, frame_size: {}, channels: {}",
            decoder.bit_rate(),
            decoder.frame_rate(),
            decoder.channels(),
            decoder.frame_size(),
            decoder.channels()
        );

        let p = input_stream.parameters();
        let p_raw_sample_rate = unsafe { (*p.as_ptr()).sample_rate } as u32;
        let format: AVSampleFormat = unsafe { std::mem::transmute((*p.as_ptr()).format) };

        let frame_rate = p_raw_sample_rate as f64 / decoder.frame_size() as f64;
        let length = (input_stream.frames() as f64) / frame_rate;
        let length = if length.is_nan() { 1e6 } else { length };
        debug!(
            "frame_rate: {}, frames: {}, length: {}, format: {format:?}",
            frame_rate,
            input_stream.frames(),
            length
        );

        audio_stream_player.call_deferred(
            "init".into(),
            &[
                frame_rate.to_variant(),
                input_stream.frames().to_variant(),
                length.to_variant(),
                (format as i32).to_variant(),
                (decoder.bit_rate() as u32).to_variant(),
                decoder.frame_size().to_variant(),
                decoder.channels().to_variant(),
            ],
        );

        let frame_size = decoder.frame_size() as usize;
        let channels = decoder.channels() as usize;

        Ok(AudioContext {
            stream_index,
            decoder,
            buffer: VecDeque::default(),
            current_frame: 0,
            start_frame: 0,
            rate: frame_rate,
            audio_stream_player,
            format,

            frame_size,
            channels,
        })
    }
}

impl FfmpegContext for AudioContext {
    fn stream_index(&self) -> Option<usize> {
        Some(self.stream_index)
    }

    fn receive_packet(&mut self, packet: Packet) -> Result<(), anyhow::Error> {
        self.decoder.send_packet(&packet).unwrap();
        let mut decoded = frame::Audio::empty();
        if let Ok(()) = self.decoder.receive_frame(&mut decoded) {
            self.buffer.push_back(decoded);
        }
        Ok(())
    }

    fn has_frame(&self) -> bool {
        !self.buffer.is_empty()
    }

    fn buffered_time(&self) -> f64 {
        self.buffer.len() as f64 / self.rate
    }

    fn send_frame(&mut self) {
        let bytes_per_sample = self.format.bytes();
        let is_planar = self.format.is_planar();
        let frame = self.buffer.pop_front().unwrap();

        let data = if is_planar && self.channels > 1 {
            let d0 = frame.data(0);
            let d1 = frame.data(1);

            PackedVector2Array::from_iter(
                d0.chunks_exact(bytes_per_sample)
                    .take(self.frame_size)
                    .zip(d1.chunks_exact(bytes_per_sample))
                    .map(|(l, r)| Vector2 {
                        x: self.format.to_f32(l),
                        y: self.format.to_f32(r),
                    }),
            )
        } else if self.channels == 1 {
            PackedVector2Array::from_iter(
                frame
                    .data(0)
                    .chunks_exact(bytes_per_sample)
                    .take(self.frame_size)
                    .map(|c| {
                        let val = self.format.to_f32(c);
                        Vector2 { x: val, y: val }
                    }),
            )
        } else {
            PackedVector2Array::from_iter(
                frame
                    .data(0)
                    .chunks_exact(bytes_per_sample * self.channels)
                    .take(self.frame_size)
                    .map(|c| Vector2 {
                        x: self.format.to_f32(&c[0..bytes_per_sample]),
                        y: self
                            .format
                            .to_f32(&c[bytes_per_sample..bytes_per_sample * 2]),
                    }),
            )
        };

        self.audio_stream_player
            .call_deferred("stream_buffer".into(), &[data.to_variant()]);

        self.current_frame += 1;
    }

    fn set_start_frame(&mut self) {
        self.start_frame = self.current_frame;
    }

    fn reset_start_frame(&mut self) {
        self.start_frame = 0;
        self.current_frame = 0;
    }

    fn seconds_till_next_frame(&self) -> f64 {
        (self.current_frame - self.start_frame + 1) as f64 / self.rate
    }
}
