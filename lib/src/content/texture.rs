use crate::utils::infer_mime;

use super::{
    content_provider::ContentProviderContext, packed_array::PackedByteArrayFromVec,
    thread_safety::GodotSingleThreadSafety,
};
use godot::{
    builtin::{GString, PackedByteArray, Variant, Vector2i},
    classes::{
        image::CompressMode, image::Format as GodotFormat, AnimatedTexture, DirAccess, Image,
        ImageTexture, ResourceLoader, Texture2D,
    },
    global::Error,
    meta::ToGodot,
    obj::Gd,
    prelude::*,
};
use image::{codecs::gif::GifDecoder, codecs::webp::WebPDecoder, AnimationDecoder, ImageReader};
use std::io::Cursor;

/// Gets the fallback texture for unsupported image formats.
/// Loads from res://assets/image_not_supported.png (Godot caches loaded resources internally).
fn get_fallback_texture() -> Gd<Texture2D> {
    let mut loader = ResourceLoader::singleton();
    if let Some(resource) = loader.load("res://assets/image_not_supported.png") {
        if let Ok(texture) = resource.try_cast::<Texture2D>() {
            return texture;
        }
    }
    // Return an empty texture if loading fails
    tracing::error!("Failed to load fallback texture from res://assets/image_not_supported.png");
    ImageTexture::new_gd().upcast()
}

/// Creates a TextureEntry using the fallback texture for unsupported formats.
fn create_fallback_texture_entry() -> Gd<TextureEntry> {
    let texture = get_fallback_texture();
    let image = Image::new_gd();
    let original_size = Vector2i::new(256, 256); // Default size for fallback

    Gd::from_init_fn(|_base| TextureEntry {
        image,
        texture,
        original_size,
    })
}

/// Decodes an image using the Rust `image` crate and creates a Godot Image from the raw pixels.
/// This is used for formats not natively supported by Godot (AVIF, etc.)
fn decode_image_with_rust_crate(bytes: &[u8]) -> Result<Gd<Image>, String> {
    let cursor = Cursor::new(bytes);
    let reader = ImageReader::new(cursor)
        .with_guessed_format()
        .map_err(|e| format!("Failed to read image format: {}", e))?;

    let dynamic_image = reader
        .decode()
        .map_err(|e| format!("Failed to decode image: {}", e))?;

    // Convert to RGBA8 format
    let rgba_image = dynamic_image.to_rgba8();
    let width = rgba_image.width() as i32;
    let height = rgba_image.height() as i32;
    let raw_pixels = rgba_image.into_raw();

    // Create Godot Image from raw RGBA8 data
    let pixels = PackedByteArray::from_vec(&raw_pixels);
    let image = Image::create_from_data(width, height, false, GodotFormat::RGBA8, &pixels)
        .ok_or_else(|| "Failed to create Godot Image from decoded pixels".to_string())?;

    Ok(image)
}

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct TextureEntry {
    #[var]
    pub image: Gd<Image>,
    #[var]
    pub texture: Gd<Texture2D>,
    #[var]
    pub original_size: Vector2i,
}

/// Decodes a GIF and creates an AnimatedTexture with compressed frames.
/// Returns (texture as Texture2D, original_size, first_frame_image)
fn decode_gif_to_animated_texture(
    bytes: &[u8],
    max_size: i32,
) -> Result<(Gd<Texture2D>, Vector2i, Gd<Image>), String> {
    let cursor = Cursor::new(bytes);
    let decoder = GifDecoder::new(cursor).map_err(|e| format!("Failed to create GIF decoder: {}", e))?;

    let frames: Vec<_> = decoder
        .into_frames()
        .collect_frames()
        .map_err(|e| format!("Failed to decode GIF frames: {}", e))?;

    if frames.is_empty() {
        return Err("GIF has no frames".to_string());
    }

    let frame_count = frames.len().min(256) as i32; // AnimatedTexture max is 256 frames
    let mut animated_texture = AnimatedTexture::new_gd();
    animated_texture.set_frames(frame_count);

    let mut original_size = Vector2i::ZERO;
    let mut first_frame_image: Option<Gd<Image>> = None;

    for (i, frame) in frames.into_iter().take(256).enumerate() {
        let delay = frame.delay();
        let (numerator, denominator) = delay.numer_denom_ms();
        let duration_secs = (numerator as f32) / (denominator as f32) / 1000.0;
        // Minimum duration of 0.01s to avoid issues
        let duration_secs = duration_secs.max(0.01);

        let rgba_image = frame.into_buffer();
        let width = rgba_image.width() as i32;
        let height = rgba_image.height() as i32;

        if i == 0 {
            original_size = Vector2i::new(width, height);
        }

        let raw_pixels = rgba_image.into_raw();
        let pixels = PackedByteArray::from_vec(&raw_pixels);

        let mut image = Image::create_from_data(width, height, false, GodotFormat::RGBA8, &pixels)
            .ok_or_else(|| format!("Failed to create Godot Image for GIF frame {}", i))?;

        // Store the first frame image before any modifications
        if i == 0 {
            first_frame_image = Some(image.clone());
        }

        // Create texture for this frame (compressed on mobile)
        let frame_texture: Gd<Texture2D> = if std::env::consts::OS == "ios"
            || std::env::consts::OS == "android"
        {
            create_compressed_texture(&mut image, max_size)
        } else {
            resize_image(&mut image, max_size);
            ImageTexture::create_from_image(&image)
                .ok_or_else(|| format!("Failed to create ImageTexture for GIF frame {}", i))?
                .upcast()
        };

        animated_texture.set_frame_texture(i as i32, &frame_texture);
        animated_texture.set_frame_duration(i as i32, duration_secs);
    }

    let first_frame = first_frame_image.ok_or("Failed to capture first frame")?;
    // Upcast AnimatedTexture to Texture2D so it can be stored in TextureEntry
    Ok((animated_texture.upcast(), original_size, first_frame))
}

/// Decodes an animated WebP and creates an AnimatedTexture with compressed frames.
/// Returns (texture as Texture2D, original_size, first_frame_image)
fn decode_animated_webp_to_texture(
    bytes: &[u8],
    max_size: i32,
) -> Result<(Gd<Texture2D>, Vector2i, Gd<Image>), String> {
    let cursor = Cursor::new(bytes);
    let decoder = WebPDecoder::new(cursor).map_err(|e| format!("Failed to create WebP decoder: {}", e))?;

    let frames: Vec<_> = decoder
        .into_frames()
        .collect_frames()
        .map_err(|e| format!("Failed to decode WebP frames: {}", e))?;

    if frames.is_empty() {
        return Err("Animated WebP has no frames".to_string());
    }

    let frame_count = frames.len().min(256) as i32; // AnimatedTexture max is 256 frames
    let mut animated_texture = AnimatedTexture::new_gd();
    animated_texture.set_frames(frame_count);

    let mut original_size = Vector2i::ZERO;
    let mut first_frame_image: Option<Gd<Image>> = None;

    for (i, frame) in frames.into_iter().take(256).enumerate() {
        let delay = frame.delay();
        let (numerator, denominator) = delay.numer_denom_ms();
        let duration_secs = (numerator as f32) / (denominator as f32) / 1000.0;
        // Minimum duration of 0.01s to avoid issues
        let duration_secs = duration_secs.max(0.01);

        let rgba_image = frame.into_buffer();
        let width = rgba_image.width() as i32;
        let height = rgba_image.height() as i32;

        if i == 0 {
            original_size = Vector2i::new(width, height);
        }

        let raw_pixels = rgba_image.into_raw();
        let pixels = PackedByteArray::from_vec(&raw_pixels);

        let mut image = Image::create_from_data(width, height, false, GodotFormat::RGBA8, &pixels)
            .ok_or_else(|| format!("Failed to create Godot Image for WebP frame {}", i))?;

        // Store the first frame image before any modifications
        if i == 0 {
            first_frame_image = Some(image.clone());
        }

        // Create texture for this frame (compressed on mobile)
        let frame_texture: Gd<Texture2D> = if std::env::consts::OS == "ios"
            || std::env::consts::OS == "android"
        {
            create_compressed_texture(&mut image, max_size)
        } else {
            resize_image(&mut image, max_size);
            ImageTexture::create_from_image(&image)
                .ok_or_else(|| format!("Failed to create ImageTexture for WebP frame {}", i))?
                .upcast()
        };

        animated_texture.set_frame_texture(i as i32, &frame_texture);
        animated_texture.set_frame_duration(i as i32, duration_secs);
    }

    let first_frame = first_frame_image.ok_or("Failed to capture first frame")?;
    // Upcast AnimatedTexture to Texture2D so it can be stored in TextureEntry
    Ok((animated_texture.upcast(), original_size, first_frame))
}

pub async fn load_image_texture(
    url: String,
    file_hash: String,
    ctx: ContentProviderContext,
) -> Result<Option<Variant>, anyhow::Error> {
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);
    let bytes_vec = ctx
        .resource_provider
        .fetch_resource_with_data(&url, &file_hash, &absolute_file_path)
        .await
        .map_err(anyhow::Error::msg)?;

    if bytes_vec.is_empty() {
        return Err(anyhow::Error::msg("Empty texture data"));
    }

    let _thread_safe_check = GodotSingleThreadSafety::acquire_owned(&ctx)
        .await
        .ok_or(anyhow::Error::msg("Failed trying to get thread-safe check"))?;

    // Check for formats that need special handling
    // AVIF: Decode using Rust image crate (Godot doesn't support AVIF natively)
    if infer_mime::is_avif(&bytes_vec) {
        tracing::debug!("Decoding AVIF image using Rust image crate: {}", url);
        let image = match decode_image_with_rust_crate(&bytes_vec) {
            Ok(img) => img,
            Err(e) => {
                DirAccess::remove_absolute(&GString::from(&absolute_file_path));
                return Err(anyhow::Error::msg(format!(
                    "Failed to decode AVIF image ({}): {}",
                    url, e
                )));
            }
        };

        let original_size = image.get_size();
        let mut image = image;

        let max_size = ctx.texture_quality.to_max_size();
        let mut texture: Gd<Texture2D> = if std::env::consts::OS == "ios"
            || std::env::consts::OS == "android"
        {
            create_compressed_texture(&mut image, max_size)
        } else {
            resize_image(&mut image, max_size);
            let texture = ImageTexture::create_from_image(&image.clone()).ok_or(anyhow::Error::msg(
                format!("Error creating texture from AVIF image {}", absolute_file_path),
            ))?;
            texture.upcast()
        };

        texture.set_name(&GString::from(&url));

        let texture_entry = Gd::from_init_fn(|_base| TextureEntry {
            image,
            texture,
            original_size,
        });

        return Ok(Some(texture_entry.to_variant()));
    }

    // HEIC: Not supported - use fallback texture
    if infer_mime::is_heic(&bytes_vec) {
        tracing::warn!("Unsupported image format: HEIC ({}), using fallback", url);
        DirAccess::remove_absolute(&GString::from(&absolute_file_path));
        return Ok(Some(create_fallback_texture_entry().to_variant()));
    }
    // GIF: Decode and create AnimatedTexture with compressed frames
    if infer_mime::is_gif(&bytes_vec) {
        tracing::debug!("Decoding GIF animation using Rust image crate: {}", url);
        let max_size = ctx.texture_quality.to_max_size();

        let (mut texture, original_size, image) =
            match decode_gif_to_animated_texture(&bytes_vec, max_size) {
                Ok(result) => result,
                Err(e) => {
                    tracing::warn!("Failed to decode GIF ({}): {}, using fallback", url, e);
                    DirAccess::remove_absolute(&GString::from(&absolute_file_path));
                    return Ok(Some(create_fallback_texture_entry().to_variant()));
                }
            };

        texture.set_name(&GString::from(&url));

        let texture_entry = Gd::from_init_fn(|_base| TextureEntry {
            image,
            texture,
            original_size,
        });

        return Ok(Some(texture_entry.to_variant()));
    }

    // Animated WebP: Decode using Rust image crate (Godot only supports static WebP)
    if infer_mime::is_animated_webp(&bytes_vec) {
        tracing::debug!("Decoding animated WebP using Rust image crate: {}", url);
        let max_size = ctx.texture_quality.to_max_size();

        let (mut texture, original_size, image) =
            match decode_animated_webp_to_texture(&bytes_vec, max_size) {
                Ok(result) => result,
                Err(e) => {
                    tracing::warn!("Failed to decode animated WebP ({}): {}, using fallback", url, e);
                    DirAccess::remove_absolute(&GString::from(&absolute_file_path));
                    return Ok(Some(create_fallback_texture_entry().to_variant()));
                }
            };

        texture.set_name(&GString::from(&url));

        let texture_entry = Gd::from_init_fn(|_base| TextureEntry {
            image,
            texture,
            original_size,
        });

        return Ok(Some(texture_entry.to_variant()));
    }

    let bytes = PackedByteArray::from_vec(&bytes_vec);

    let mut image = Image::new_gd();
    // Static WebP is handled by Godot's native loader below
    let err = if infer_mime::is_png(&bytes_vec) {
        image.load_png_from_buffer(&bytes)
    } else if infer_mime::is_jpeg(&bytes_vec) || infer_mime::is_jpeg2000(&bytes_vec) {
        image.load_jpg_from_buffer(&bytes)
    } else if infer_mime::is_webp(&bytes_vec) {
        image.load_webp_from_buffer(&bytes)
    } else if infer_mime::is_tga(&bytes_vec) {
        image.load_tga_from_buffer(&bytes)
    } else if infer_mime::is_ktx(&bytes_vec) {
        image.load_ktx_from_buffer(&bytes)
    } else if infer_mime::is_bmp(&bytes_vec) {
        image.load_bmp_from_buffer(&bytes)
    } else if infer_mime::is_svg(&bytes_vec) {
        image.load_svg_from_buffer(&bytes)
    } else {
        // Unknown format - use fallback texture
        let format_hint = if bytes_vec.len() >= 4 {
            format!("magic bytes: {:02x} {:02x} {:02x} {:02x}",
                bytes_vec[0], bytes_vec[1], bytes_vec[2], bytes_vec[3])
        } else {
            "insufficient data".to_string()
        };
        tracing::warn!(
            "Unknown/unsupported image format ({}) for {}, using fallback",
            format_hint, url
        );
        DirAccess::remove_absolute(&GString::from(&absolute_file_path));
        return Ok(Some(create_fallback_texture_entry().to_variant()));
    };

    if err != Error::OK {
        let err_code = err.to_variant().to::<i32>();
        tracing::warn!(
            "Error loading texture {}: error code {}, using fallback",
            absolute_file_path, err_code
        );
        DirAccess::remove_absolute(&GString::from(&absolute_file_path));
        return Ok(Some(create_fallback_texture_entry().to_variant()));
    }

    let original_size = image.get_size();

    let max_size = ctx.texture_quality.to_max_size();
    let mut texture: Gd<Texture2D> = if std::env::consts::OS == "ios"
        || std::env::consts::OS == "android"
    {
        create_compressed_texture(&mut image, max_size)
    } else {
        resize_image(&mut image, max_size);
        let texture = ImageTexture::create_from_image(&image.clone()).ok_or(anyhow::Error::msg(
            format!("Error creating texture from image {}", absolute_file_path),
        ))?;
        texture.upcast()
    };

    texture.set_name(&GString::from(&url));

    let texture_entry = Gd::from_init_fn(|_base| TextureEntry {
        image,
        texture,
        original_size,
    });

    Ok(Some(texture_entry.to_variant()))
}

/// Creates a texture from a compressed image, resizing if needed.
/// Uses ETC2 compression for better memory usage on mobile platforms.
/// Returns an ImageTexture containing the compressed image data.
pub fn create_compressed_texture(image: &mut Gd<Image>, max_size: i32) -> Gd<Texture2D> {
    resize_image(image, max_size);

    if !image.is_compressed() {
        image.compress(CompressMode::ETC2);
    }

    // Create ImageTexture from the compressed image
    // The compressed image data will be preserved when saved/loaded
    let texture = ImageTexture::create_from_image(&*image)
        .expect("Failed to create ImageTexture from compressed image");
    texture.upcast()
}

pub fn resize_image(image: &mut Gd<Image>, max_size: i32) -> bool {
    let image_width = image.get_width();
    let image_height = image.get_height();
    if image_width > image_height {
        if image_width > max_size {
            image.resize(max_size, (image_height * max_size) / image_width);
            tracing::debug!(
                "Resize! {}x{} to {}x{}",
                image_width,
                image_height,
                image.get_width(),
                image.get_height()
            );
            return true;
        }
    } else if image_height > max_size {
        image.resize((image_width * max_size) / image_height, max_size);
        tracing::debug!(
            "Resize! {}x{} to {}x{}",
            image_width,
            image_height,
            image.get_width(),
            image.get_height()
        );
        return true;
    }

    false
}
