use std::collections::VecDeque;
use std::time::Instant;

use ffmpeg_next::ffi::AVPixelFormat;
use ffmpeg_next::format::Pixel;
use ffmpeg_next::software::scaling::{context::Context, flag::Flags};
use ffmpeg_next::{decoder, format::context::Input, media::Type, util::frame, Packet};
use godot::builtin::PackedByteArray;
use godot::engine::image::Format;
use godot::engine::{Image, ImageTexture};
use godot::prelude::{Gd, Vector2};
use thiserror::Error;
use tracing::debug;

use crate::content::packed_array::PackedByteArrayFromVec;

use super::stream_processor::FfmpegContext;

pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    pub rate: f64,
    pub length: f64,
}

pub struct VideoContext {
    stream_index: usize,
    decoder: decoder::Video,
    scaler_context: Context,
    rate: f64,
    buffer: VecDeque<frame::video::Video>,
    current_frame: usize,
    start_frame: usize,

    // godot texture
    texture: Gd<ImageTexture>,
    video_info: VideoInfo,
    last_frame_time: Instant,
}

#[derive(Debug, Error)]
pub enum VideoError {
    #[error("Bad pixel format")]
    BadPixelFormat,
    #[error("No Stream")]
    NoStream,
    #[error("Remote channel closed")]
    ChannelClosed,
    #[error("Failed: {0}")]
    Failed(ffmpeg_next::Error),
}

impl VideoContext {
    pub fn init(input_context: &Input, tex: Gd<ImageTexture>) -> Result<Self, VideoError> {
        let input_stream = input_context
            .streams()
            .best(Type::Video)
            .ok_or(VideoError::NoStream)?;

        let pixel_format: AVPixelFormat =
            unsafe { std::mem::transmute((*input_stream.parameters().as_ptr()).format) };

        if pixel_format == AVPixelFormat::AV_PIX_FMT_NONE {
            return Err(VideoError::BadPixelFormat);
        }

        let stream_index = input_stream.index();

        let context_decoder =
            ffmpeg_next::codec::context::Context::from_parameters(input_stream.parameters())
                .map_err(VideoError::Failed)?;

        let decoder = context_decoder
            .decoder()
            .video()
            .map_err(VideoError::Failed)?;

        let roundup = |x: u32| {
            (x.saturating_sub(1) / 8 + 1) * 8
            // x
        };

        let width = roundup(decoder.width());
        let height = roundup(decoder.height());

        let scaler_context = Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::RGBA,
            width,
            height,
            Flags::BILINEAR,
        )
        .map_err(VideoError::Failed)?;

        let rate = f64::from(input_stream.rate());
        let length = (input_stream.frames() as f64) / rate;
        debug!(
            "frames: {}, length: {}, rate: {}",
            input_stream.frames(),
            length,
            rate
        );

        Ok(VideoContext {
            stream_index,
            rate,
            decoder,
            scaler_context,
            buffer: Default::default(),
            current_frame: 0,
            start_frame: 0,
            texture: tex,
            video_info: VideoInfo {
                width,
                height,
                rate,
                length,
            },
            last_frame_time: Instant::now(),
        })
    }
}

impl FfmpegContext for VideoContext {
    fn stream_index(&self) -> Option<usize> {
        Some(self.stream_index)
    }

    fn receive_packet(&mut self, packet: Packet) -> Result<(), anyhow::Error> {
        self.decoder.send_packet(&packet).unwrap();
        let mut decoded = frame::Video::empty();
        if let Ok(()) = self.decoder.receive_frame(&mut decoded) {
            let mut rgb_frame = frame::Video::empty();
            // run frame through scaler for color space conversion
            self.scaler_context.run(&decoded, &mut rgb_frame)?;
            self.buffer.push_back(rgb_frame);
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
        let current_frame = self.buffer.pop_front().unwrap();
        // let data_arr = PackedByteArray::from(current_frame.data(0));

        let raw_data = current_frame.data(0);
        let data_arr = PackedByteArray::from_vec(raw_data);

        let diff = self.last_frame_time.elapsed().as_secs_f32();
        debug!(
            "send video frame {:?} [{} in buffer] diff {:?} => fps {:?} ({:?} bytes)",
            self.current_frame,
            self.buffer.len(),
            diff,
            1.0 / diff,
            current_frame.data(0).len()
        );
        self.last_frame_time = Instant::now();

        let current_size =
            Vector2::new(self.video_info.width as f32, self.video_info.height as f32);
        let texture_size = self.texture.get_size();

        if current_size != texture_size {
            let image = Image::create_from_data(
                self.video_info.width as i32,
                self.video_info.height as i32,
                false,
                Format::RGBA8,
                data_arr,
            )
            .unwrap();
            self.texture.set_image(image.clone());
            self.texture.update(image);
        } else {
            let mut image = self.texture.get_image().unwrap();
            image.set_data(
                self.video_info.width as i32,
                self.video_info.height as i32,
                false,
                Format::RGBA8,
                data_arr,
            );
            self.texture.update(image);
        }

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

    fn length(&self) -> f64 {
        self.video_info.length
    }

    fn position(&self) -> f64 {
        self.current_frame as f64 / self.rate
    }
}
