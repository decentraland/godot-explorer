use godot::{
    builtin::{meta::ToGodot, GString, PackedByteArray, StringName, Variant, Vector2, Vector2i},
    engine::{
        global::Error, image::CompressMode,
        portable_compressed_texture_2d::CompressionMode, ColorRect, Engine, Image,
        ImageTexture, Node, PortableCompressedTexture2D, ResourceLoader, SceneTree, SubViewport, Texture2D,
        VideoStreamPlayer, VideoStreamTheora, sub_viewport::UpdateMode,
    },
    obj::{Gd, NewAlloc},
    prelude::*,
};

use crate::{
    content::{
        content_provider::ContentProviderContext,
        packed_array::PackedByteArrayFromVec,
    },
    utils::infer_mime,
};


#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclUnifiedMediaLoader {
    base: Base<RefCounted>,
}

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
    resize_image(image, max_size);
    
    if !image.is_compressed() {
        image.compress(CompressMode::ETC2);
    }
    
    let mut texture = PortableCompressedTexture2D::new_gd();
    texture.create_from_image(image.clone(), CompressionMode::ETC2);
    texture.upcast()
}

pub enum MediaType {
    Image,
    Video,
    Unknown,
}

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
        if lower_path.ends_with(".png") || lower_path.ends_with(".jpg") || 
           lower_path.ends_with(".jpeg") || lower_path.ends_with(".webp") ||
           lower_path.ends_with(".bmp") || lower_path.ends_with(".svg") ||
           lower_path.ends_with(".tga") {
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
        } else if infer_mime::is_png(bytes) || infer_mime::is_jpeg(bytes) || 
                  infer_mime::is_webp(bytes) || infer_mime::is_bmp(bytes) || 
                  infer_mime::is_svg(bytes) || infer_mime::is_tga(bytes) {
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
        
        // First, detect what type of media we expect based on the URL
        // For metamorph URLs with .gif, we know it will be converted to .ogv
        let expected_extension = if url.contains(".gif") || url.contains("videoFormat=ogv") {
            ".ogv"
        } else if url.contains("imageFormat=astc") || url.contains("imageFormat=ktx2") {
            ".ktx2"
        } else {
            ""
        };
        
        // Set the file path with the expected extension for metamorph content
        let absolute_file_path = format!("{}{}{}", ctx.content_folder, file_hash, expected_extension);
        
        godot_print!("  Using file path with extension: {}", absolute_file_path);
        
        // Use resource_provider to fetch the resource with the correct path
        let bytes_vec = ctx
            .resource_provider
            .fetch_resource_with_data(&url, &file_hash, &absolute_file_path)
            .await
            .map_err(anyhow::Error::msg)?;

        godot_print!("load_unified_media - Fetched {} bytes", bytes_vec.len());

        if bytes_vec.is_empty() {
            return Err(anyhow::Error::msg("Empty media data"));
        }

        // Detect media type from the content
        let media_type = Self::detect_media_type(&bytes_vec, &url);
        
        godot_print!("load_unified_media - File path: {}", absolute_file_path);

        // Acquire the Godot single thread semaphore for creating textures/video players
        let semaphore = ctx.godot_single_thread.clone();
        let _permit = semaphore.acquire().await
            .map_err(|_| anyhow::Error::msg("Failed to acquire Godot thread semaphore"))?;

        match media_type {
            MediaType::Image => {
                godot_print!("Loading as IMAGE...");
                Self::load_as_image(bytes_vec, url, absolute_file_path, ctx.clone()).await
            }
            MediaType::Video => {
                godot_print!("Loading as VIDEO...");
                // For videos, we need to handle them on the main thread
                // The file is already saved by resource_provider with the correct extension
                Self::load_as_video_main_thread(url, absolute_file_path, file_hash, ctx.clone()).await
            }
            MediaType::Unknown => {
                godot_print!("Unknown type, trying as IMAGE first...");
                // Try to load as image first, then fallback
                match Self::load_as_image(bytes_vec.clone(), url.clone(), absolute_file_path.clone(), ctx.clone()).await {
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
        
        let bytes = PackedByteArray::from_vec(&bytes_vec);
        let mut image = Image::new_gd();
        
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
            // Try KTX2 by default for unknown formats
            image.load_ktx_from_buffer(bytes)
        };

        if err != Error::OK {
            godot_print!("  ERROR: Failed to load image, error code: {:?}", err);
            return Self::load_fallback_texture();
        }
        
        godot_print!("  Image loaded successfully!");

        let original_size = image.get_size();
        let max_size = ctx.texture_quality.to_max_size();
        
        let mut texture: Gd<Texture2D> = if std::env::consts::OS == "ios" {
            create_compressed_texture(&mut image, max_size)
        } else {
            resize_image(&mut image, max_size);
            let texture = ImageTexture::create_from_image(image.clone())
                .ok_or(anyhow::Error::msg(format!("Error creating texture from image {}", absolute_file_path)))?;
            texture.upcast()
        };

        texture.set_name(GString::from(&url));

        let texture_entry = Gd::from_init_fn(|_base| UnifiedTextureEntry {
            image,
            texture,
            original_size,
        });

        Ok(Some(texture_entry.to_variant()))
    }

    async fn load_as_video_main_thread(
        url: String,
        absolute_file_path: String,
        _file_hash: String,
        _ctx: ContentProviderContext,
    ) -> Result<Option<Variant>, anyhow::Error> {
        godot_print!("load_as_video - Loading video from URL: {}", url);
        godot_print!("  Video file path: {}", absolute_file_path);
        
        // The video is already saved in the content folder by resource_provider
        // Check if the file exists
        if !std::path::Path::new(&absolute_file_path).exists() {
            godot_print!("  ERROR: Video file does not exist at path: {}", absolute_file_path);
            return Self::load_fallback_texture();
        }
        
        godot_print!("  File exists, creating video components on main thread...");
        // Note: We're already on the main thread via the semaphore
        
        // Create all video components on the main thread
        let mut container = Node::new_alloc();
        container.set_name(GString::from("VideoContainer"));
        
        let mut viewport = SubViewport::new_alloc();
        viewport.set_size(Vector2i::new(1024, 1024));
        viewport.set_update_mode(UpdateMode::ALWAYS);
        viewport.set_disable_3d(true);
        viewport.set_transparent_background(false);
        
        // Add background
        let mut bg = ColorRect::new_alloc();
        bg.set_color(Color::BLACK);
        let size = viewport.get_size();
        bg.set_size(Vector2::new(size.x as f32, size.y as f32));
        viewport.add_child(bg.upcast());
        
        // Create and configure video player
        let mut video_player = VideoStreamPlayer::new_alloc();
        video_player.set_size(Vector2::new(size.x as f32, size.y as f32));
        video_player.set_expand(true);
        
        // Set up video stream
        let mut video_stream = VideoStreamTheora::new_gd();
        video_stream.set_file(GString::from(&absolute_file_path));
        
        let loaded_file = video_stream.get_file();
        godot_print!("  Loaded video file: {}", loaded_file);
        
        if loaded_file.is_empty() {
            godot_print!("  ERROR: VideoStreamTheora failed to load the file!");
            return Self::load_fallback_texture();
        }
        
        // Configure player
        video_player.set_loop(true);
        video_player.set_volume_db(-80.0);
        video_player.set_autoplay(true);
        video_player.set_paused(false);
        video_player.set_stream(video_stream.upcast());
        
        // Add video player to viewport
        viewport.add_child(video_player.clone().upcast());
        
        // Add viewport to container
        container.add_child(viewport.clone().upcast());
        
        // Add container to scene tree using deferred call to ensure we're on main thread
        if let Some(main_loop) = Engine::singleton().get_main_loop() {
            if let Ok(scene_tree) = main_loop.try_cast::<SceneTree>() {
                if let Some(mut root) = scene_tree.get_root() {
                    // Use call_deferred to add the container on the main thread
                    root.call_deferred(
                        StringName::from("add_child"),
                        &[container.to_variant()]
                    );
                    godot_print!("  Scheduled video container addition to scene tree");
                    
                    // Schedule the video to play after it's in the tree
                    video_player.call_deferred(StringName::from("play"), &[]);
                    godot_print!("  Scheduled video playback");
                    
                    // Since we can't get the viewport texture immediately (nodes aren't in tree yet),
                    // we need to return the viewport itself and let Godot handle the texture
                    // The viewport will generate its texture once it's in the scene tree
                    
                    // Get the viewport texture - this will work once the viewport is in the tree
                    if let Some(viewport_texture) = viewport.get_texture() {
                        godot_print!("  Got viewport texture successfully");
                        
                        let mut texture = viewport_texture;
                        
                        // Store metadata to keep references alive
                        texture.set_meta(
                            StringName::from("video_container"),
                            container.to_variant()
                        );
                        texture.set_meta(
                            StringName::from("video_viewport"),
                            viewport.to_variant()
                        );
                        texture.set_meta(
                            StringName::from("video_player"),
                            video_player.to_variant()
                        );
                        
                        // Create texture entry with viewport texture
                        let texture_entry = Gd::from_init_fn(|_base| UnifiedTextureEntry {
                            image: Image::new_gd(),
                            texture: texture.upcast(),
                            original_size: Vector2i::new(1024, 1024),
                        });
                        
                        godot_print!("  Video texture ready!");
                        return Ok(Some(texture_entry.to_variant()));
                    } else {
                        godot_print!("  WARNING: Viewport texture not immediately available, returning viewport for deferred texture access");
                        
                        // Create a placeholder entry that contains the viewport
                        // The texture will be available after the viewport is in the scene tree
                        let placeholder_texture = ImageTexture::new_gd();
                        
                        // Store the viewport and player as metadata
                        let mut texture_entry = Gd::from_init_fn(|_base| UnifiedTextureEntry {
                            image: Image::new_gd(),
                            texture: placeholder_texture.upcast(),
                            original_size: Vector2i::new(1024, 1024),
                        });
                        
                        texture_entry.bind_mut().texture.set_meta(
                            StringName::from("video_container"),
                            container.to_variant()
                        );
                        texture_entry.bind_mut().texture.set_meta(
                            StringName::from("video_viewport"),
                            viewport.to_variant()
                        );
                        texture_entry.bind_mut().texture.set_meta(
                            StringName::from("video_player"),
                            video_player.to_variant()
                        );
                        texture_entry.bind_mut().texture.set_meta(
                            StringName::from("pending_viewport_texture"),
                            true.to_variant()
                        );
                        
                        return Ok(Some(texture_entry.to_variant()));
                    }
                } else {
                    godot_print!("  ERROR: Could not get root node");
                }
            } else {
                godot_print!("  ERROR: Could not cast to SceneTree");
            }
        } else {
            godot_print!("  ERROR: Could not get main loop");
        }
        
        // If we got here, something went wrong
        Self::load_fallback_texture()
    }

    fn load_fallback_texture() -> Result<Option<Variant>, anyhow::Error> {
        // Load the placeholder SVG
        let placeholder_path = GString::from("res://assets/ui/no-image-placeholder.svg");
        
        let resource = ResourceLoader::singleton()
            .load(placeholder_path.clone());
        
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

#[godot_api]
impl DclUnifiedMediaLoader {
    #[func]
    fn detect_media_type_from_bytes(bytes: PackedByteArray, file_path: GString) -> GString {
        let media_type = Self::detect_media_type(&bytes.to_vec(), &file_path.to_string());
        match media_type {
            MediaType::Image => GString::from("image"),
            MediaType::Video => GString::from("video"),
            MediaType::Unknown => GString::from("unknown"),
        }
    }
    
    #[func]
    fn finalize_video_texture(viewport: Gd<SubViewport>) -> Option<Gd<Texture2D>> {
        // This is called on the main thread after the viewport is in the scene tree
        if let Some(texture) = viewport.get_texture() {
            godot_print!("  finalize_video_texture: Got viewport texture");
            Some(texture.upcast())
        } else {
            godot_print!("  finalize_video_texture: Failed to get viewport texture");
            None
        }
    }
}