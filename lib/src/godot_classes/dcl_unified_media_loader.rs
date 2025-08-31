use godot::{
    builtin::{
        meta::ToGodot, Array, GString, PackedByteArray, StringName, Variant, Vector2, Vector2i,
    },
    engine::{
        global::Error, image::CompressMode, portable_compressed_texture_2d::CompressionMode,
        sub_viewport::UpdateMode, Engine, Image, ImageTexture, Node, PortableCompressedTexture2D,
        ResourceLoader, SceneTree, SubViewport, Texture2D, VideoStreamPlayer, VideoStreamTheora,
    },
    obj::{Gd, NewAlloc},
    prelude::*,
};

use crate::{
    content::{content_provider::ContentProviderContext, packed_array::PackedByteArrayFromVec},
    utils::infer_mime,
};

// Re-implement TextureEntry locally since it's in a private module
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct UnifiedTextureEntry {
    #[var]
    pub image: Gd<Image>,
    #[var]
    pub texture: Gd<Texture2D>,
    #[var]
    pub original_size: Vector2i,
}

// Helper functions for texture processing
fn resize_image(image: &mut Gd<Image>, max_size: i32) -> bool {
    let image_width = image.get_width();
    let image_height = image.get_height();
    if image_width > image_height {
        if image_width > max_size {
            image.resize(max_size, (image_height * max_size) / image_width);
            return true;
        }
    } else if image_height > max_size {
        image.resize((image_width * max_size) / image_height, max_size);
        return true;
    }
    false
}

fn create_compressed_texture(image: &mut Gd<Image>, max_size: i32) -> Gd<Texture2D> {
    // Only resize if needed
    resize_image(image, max_size);

    // Check if already compressed to avoid recompression
    if image.is_compressed() {
        godot_print!("  Image is already compressed, using as-is");
        // Already compressed (e.g., ASTC from KTX2), create texture directly
        let texture = ImageTexture::create_from_image(image.clone())
            .expect("Failed to create texture from compressed image");
        texture.upcast()
    } else {
        godot_print!("  Compressing image to ETC2");
        // Not compressed, compress to ETC2 for iOS
        image.compress(CompressMode::ETC2);
        let mut texture = PortableCompressedTexture2D::new_gd();
        texture.create_from_image(image.clone(), CompressionMode::ETC2);
        texture.upcast()
    }
}

pub enum MediaType {
    Image,
    Video,
    Unknown,
}

// Not a Godot class - just a module with functions
pub struct DclUnifiedMediaLoader;

impl DclUnifiedMediaLoader {
    pub fn detect_media_type(bytes: &[u8], file_path: &str) -> MediaType {
        godot_print!("Detecting media type for URL: {}", file_path);

        // First check by file extension (metamorph adds extension to the final URL)
        let lower_path = file_path.to_lowercase();

        // Check for video extensions
        if lower_path.ends_with(".ogv") || lower_path.ends_with(".ogg") {
            godot_print!("  -> Detected as VIDEO by extension (.ogv/.ogg)");
            return MediaType::Video;
        }

        // Check for image extensions (metamorph converts to ktx2)
        if lower_path.ends_with(".ktx2") || lower_path.ends_with(".ktx") {
            godot_print!("  -> Detected as IMAGE by extension (.ktx2/.ktx)");
            return MediaType::Image;
        }

        // Check for standard image formats
        if lower_path.ends_with(".png")
            || lower_path.ends_with(".jpg")
            || lower_path.ends_with(".jpeg")
            || lower_path.ends_with(".webp")
            || lower_path.ends_with(".bmp")
            || lower_path.ends_with(".svg")
            || lower_path.ends_with(".tga")
        {
            godot_print!("  -> Detected as IMAGE by standard image extension");
            return MediaType::Image;
        }

        // Check for GIF (will be converted to video)
        if lower_path.ends_with(".gif") {
            godot_print!("  -> Detected as VIDEO by extension (.gif -> will be converted)");
            return MediaType::Video;
        }

        // If no extension match, check by content signature
        godot_print!("  No extension match, checking content signature...");

        if infer_mime::is_ogv(bytes) {
            godot_print!("  -> Detected as VIDEO by OGV content signature");
            MediaType::Video
        } else if infer_mime::is_ktx2(bytes) || infer_mime::is_ktx(bytes) {
            godot_print!("  -> Detected as IMAGE by KTX/KTX2 content signature");
            MediaType::Image
        } else if infer_mime::is_gif(bytes) {
            godot_print!("  -> Detected as VIDEO by GIF content signature (will be converted)");
            MediaType::Video
        } else if infer_mime::is_png(bytes)
            || infer_mime::is_jpeg(bytes)
            || infer_mime::is_webp(bytes)
            || infer_mime::is_bmp(bytes)
            || infer_mime::is_svg(bytes)
            || infer_mime::is_tga(bytes)
        {
            godot_print!("  -> Detected as IMAGE by image content signature");
            MediaType::Image
        } else {
            godot_print!("  -> Unknown format, defaulting to IMAGE");
            MediaType::Image
        }
    }

    pub async fn load_unified_media(
        url: String,
        file_hash: String,
        ctx: ContentProviderContext,
    ) -> Result<Option<Variant>, anyhow::Error> {
        godot_print!("load_unified_media - URL: {}", url);
        godot_print!("load_unified_media - file_hash: {}", file_hash);

        // First, fetch the data to determine the actual content type
        let bytes_vec = ctx
            .resource_provider
            .fetch_resource_with_data(
                &url,
                &file_hash,
                &format!("{}{}", ctx.content_folder, file_hash),
            )
            .await
            .map_err(anyhow::Error::msg)?;

        godot_print!("load_unified_media - Fetched {} bytes", bytes_vec.len());

        if bytes_vec.is_empty() {
            return Err(anyhow::Error::msg("Empty media data"));
        }

        // Detect media type from the actual content
        let media_type = Self::detect_media_type(&bytes_vec, &url);

        // Determine the correct file extension based on actual content
        let expected_extension = match media_type {
            MediaType::Video => ".ogv", // Videos need .ogv extension
            MediaType::Image => "",     // Images don't need extension, detected by content
            MediaType::Unknown => "",   // No extension for unknown
        };

        // Set the file path with the correct extension based on actual content
        let absolute_file_path =
            format!("{}{}{}", ctx.content_folder, file_hash, expected_extension);

        godot_print!("  Using file path with extension: {}", absolute_file_path);

        // If the file needs a different extension, save it with the correct one
        if !expected_extension.is_empty() {
            // Store the file with the correct extension
            ctx.resource_provider
                .store_file(&format!("{}{}", file_hash, expected_extension), &bytes_vec)
                .await
                .map_err(anyhow::Error::msg)?;
        }

        godot_print!("load_unified_media - File path: {}", absolute_file_path);

        // Acquire the Godot single thread semaphore for creating textures/video players
        let semaphore = ctx.godot_single_thread.clone();
        let _permit = semaphore
            .acquire()
            .await
            .map_err(|_| anyhow::Error::msg("Failed to acquire Godot thread semaphore"))?;

        match media_type {
            MediaType::Image => {
                godot_print!("Loading as IMAGE...");
                Self::load_as_image(bytes_vec, url, absolute_file_path, ctx.clone()).await
            }
            MediaType::Video => {
                godot_print!("Loading as VIDEO...");

                // We already have the semaphore, so we can create the video texture directly
                // without needing to defer to the main thread
                if let Some(texture) =
                    Self::create_video_texture_internal(absolute_file_path.clone())
                {
                    godot_print!("  Video texture created successfully");
                    let texture_entry = Gd::from_init_fn(|_base| UnifiedTextureEntry {
                        image: Image::new_gd(),
                        texture,
                        original_size: Vector2i::new(1024, 1024),
                    });
                    Ok(Some(texture_entry.to_variant()))
                } else {
                    godot_print!("  Failed to create video texture");
                    Self::load_fallback_texture()
                }
            }
            MediaType::Unknown => {
                godot_print!("Unknown type, trying as IMAGE first...");
                // Try to load as image first, then fallback
                match Self::load_as_image(
                    bytes_vec.clone(),
                    url.clone(),
                    absolute_file_path.clone(),
                    ctx.clone(),
                )
                .await
                {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        godot_print!("Failed to load as image: {}, loading fallback", e);
                        Self::load_fallback_texture()
                    }
                }
            }
        }
    }

    async fn load_as_image(
        bytes_vec: Vec<u8>,
        url: String,
        absolute_file_path: String,
        ctx: ContentProviderContext,
    ) -> Result<Option<Variant>, anyhow::Error> {
        godot_print!("load_as_image - Loading image from URL: {}", url);
        godot_print!("load_as_image - Bytes length: {}", bytes_vec.len());

        // Convert bytes to PackedByteArray - this is a necessary copy for Godot's API
        let bytes = PackedByteArray::from_vec(&bytes_vec);
        let mut image = Image::new_gd();

        // Load the appropriate format
        let err = if infer_mime::is_ktx2(&bytes_vec) || infer_mime::is_ktx(&bytes_vec) {
            godot_print!("  Trying to load as KTX/KTX2...");
            image.load_ktx_from_buffer(bytes)
        } else if infer_mime::is_png(&bytes_vec) {
            godot_print!("  Trying to load as PNG...");
            image.load_png_from_buffer(bytes)
        } else if infer_mime::is_jpeg(&bytes_vec) || infer_mime::is_jpeg2000(&bytes_vec) {
            godot_print!("  Trying to load as JPEG...");
            image.load_jpg_from_buffer(bytes)
        } else if infer_mime::is_webp(&bytes_vec) {
            godot_print!("  Trying to load as WebP...");
            image.load_webp_from_buffer(bytes)
        } else if infer_mime::is_bmp(&bytes_vec) {
            godot_print!("  Trying to load as BMP...");
            image.load_bmp_from_buffer(bytes)
        } else if infer_mime::is_svg(&bytes_vec) {
            godot_print!("  Trying to load as SVG...");
            image.load_svg_from_buffer(bytes)
        } else {
            godot_print!("  Unknown format, trying KTX2 by default...");
            image.load_ktx_from_buffer(bytes)
        };

        if err != Error::OK {
            godot_print!("  ERROR: Failed to load image, error code: {:?}", err);
            return Self::load_fallback_texture();
        }

        godot_print!("  Image loaded successfully!");
        let original_size = image.get_size();
        let max_size = ctx.texture_quality.to_max_size();

        // Check if image is already compressed (e.g., ASTC from metamorph KTX2)
        let is_compressed = image.is_compressed();
        godot_print!("  Image compressed: {}", is_compressed);

        // Create texture based on compression status and platform
        let texture: Gd<Texture2D> = if is_compressed {
            // Already compressed (ASTC from metamorph), don't recompress
            godot_print!("  Using existing compression (ASTC from metamorph)");
            resize_image(&mut image, max_size);
            let texture = ImageTexture::create_from_image(image.clone()).ok_or(
                anyhow::Error::msg(format!(
                    "Error creating texture from compressed image {}",
                    absolute_file_path
                )),
            )?;
            texture.upcast()
        } else if std::env::consts::OS == "ios" {
            // Only compress for iOS if not already compressed
            godot_print!("  Compressing for iOS (ETC2)");
            create_compressed_texture(&mut image, max_size)
        } else {
            // Regular uncompressed texture for other platforms
            godot_print!("  Creating uncompressed texture");
            resize_image(&mut image, max_size);
            let texture =
                ImageTexture::create_from_image(image.clone()).ok_or(anyhow::Error::msg(
                    format!("Error creating texture from image {}", absolute_file_path),
                ))?;
            texture.upcast()
        };

        // Create texture entry
        let mut texture_entry = Gd::from_init_fn(|_base| UnifiedTextureEntry {
            image,
            texture,
            original_size,
        });

        // Set texture metadata
        texture_entry
            .bind_mut()
            .texture
            .set_name(GString::from(&url));

        Ok(Some(texture_entry.to_variant()))
    }

    fn create_video_texture_internal(absolute_file_path: String) -> Option<Gd<Texture2D>> {
        godot_print!(
            "create_video_texture - Video file path: {}",
            absolute_file_path
        );

        // Verify file exists
        if !std::path::Path::new(&absolute_file_path).exists() {
            godot_print!(
                "  ERROR: Video file does not exist at path: {}",
                absolute_file_path
            );
            return None;
        }

        godot_print!("  Setting up video components...");

        // OPTIMIZATION: Create minimal node hierarchy
        // Container -> Viewport -> VideoPlayer
        let mut container = Node::new_alloc();
        container.set_name(GString::from("VideoContainer"));

        // Configure viewport for optimal performance
        let mut viewport = SubViewport::new_alloc();
        viewport.set_size(Vector2i::new(1024, 1024));
        viewport.set_update_mode(UpdateMode::ALWAYS);
        viewport.set_disable_3d(true);
        viewport.set_transparent_background(false);
        // OPTIMIZATION: Disable features we don't need
        viewport.set_use_debanding(false);

        // Create video player and stream
        let mut video_player = VideoStreamPlayer::new_alloc();
        video_player.set_size(Vector2::new(1024.0, 1024.0));
        video_player.set_expand(true);

        // Load video stream directly from file
        let mut video_stream = VideoStreamTheora::new_gd();
        video_stream.set_file(GString::from(&absolute_file_path));

        if video_stream.get_file().is_empty() {
            godot_print!("  ERROR: VideoStreamTheora failed to load the file!");
            return None;
        }

        // Configure for looping playback
        video_player.set_loop(true);
        video_player.set_volume_db(-80.0); // Mute
        video_player.set_autoplay(true);
        video_player.set_stream(video_stream.upcast());

        // Build hierarchy
        viewport.add_child(video_player.clone().upcast());
        container.add_child(viewport.clone().upcast());

        // Add to scene tree using deferred calls to avoid threading issues
        if let Some(main_loop) = Engine::singleton().get_main_loop() {
            if let Ok(scene_tree) = main_loop.try_cast::<SceneTree>() {
                if let Some(mut root) = scene_tree.get_root() {
                    // Use call_deferred for thread safety
                    root.call_deferred(StringName::from("add_child"), &[container.to_variant()]);

                    // Schedule playback
                    video_player.call_deferred(StringName::from("play"), &[]);

                    // Get viewport texture - this should work even if not in tree yet
                    // The texture will update once the viewport starts rendering
                    if let Some(viewport_texture) = viewport.get_texture() {
                        godot_print!("  Got viewport texture (zero-copy GPU reference)");

                        let mut texture = viewport_texture;

                        // Store minimal metadata to prevent GC
                        let mut refs = Array::new();
                        refs.push(container.to_variant());
                        refs.push(viewport.to_variant());
                        refs.push(video_player.to_variant());
                        texture.set_meta(StringName::from("_video_refs"), refs.to_variant());

                        return Some(texture.upcast());
                    } else {
                        godot_print!("  WARNING: Could not get viewport texture immediately");
                        // Return None, will need to handle fallback
                    }
                }
            }
        }

        None
    }

    fn load_fallback_texture() -> Result<Option<Variant>, anyhow::Error> {
        // Load the placeholder SVG
        let placeholder_path = GString::from("res://assets/ui/no-image-placeholder.svg");

        let resource = ResourceLoader::singleton().load(placeholder_path.clone());

        if let Some(resource) = resource {
            if let Ok(texture) = resource.try_cast::<Texture2D>() {
                let image = texture.get_image().unwrap_or_else(|| Image::new_gd());
                let original_size = image.get_size();

                let texture_entry = Gd::from_init_fn(|_base| UnifiedTextureEntry {
                    image,
                    texture,
                    original_size,
                });

                return Ok(Some(texture_entry.to_variant()));
            }
        }

        Err(anyhow::Error::msg("Failed to load fallback texture"))
    }
}
