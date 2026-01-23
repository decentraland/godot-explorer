//! Asset processing orchestration.
//!
//! This module calls the existing GLTF/texture loading functions from the content module.

use std::sync::Arc;

use godot::classes::image::CompressMode;
use godot::classes::resource_saver::SaverFlags;
use godot::classes::{Image, ImageTexture, Os, Resource, ResourceSaver};
use godot::prelude::*;
use tokio::sync::Semaphore;

use crate::content::content_mapping::{ContentMappingAndUrl, ContentMappingAndUrlRef};
use crate::content::content_provider::SceneGltfContext;
use crate::content::gltf::{
    get_dependencies, get_embedded_texture_size, load_and_save_emote_gltf,
    load_and_save_scene_gltf, load_and_save_wearable_gltf,
};
use crate::content::packed_array::PackedByteArrayFromVec;
use crate::content::resource_provider::ResourceProvider;
use crate::content::thread_safety::GodotSingleThreadSafety;
use crate::godot_classes::dcl_config::TextureQuality;
use crate::utils::infer_mime;

use super::job_manager::JobManager;
use super::types::{AssetRequest, AssetType, JobStatus};

/// Context for asset processing, similar to ContentProviderContext but standalone.
#[derive(Clone)]
pub struct ProcessorContext {
    pub content_folder: Arc<String>,
    pub output_folder: Arc<String>,
    pub resource_provider: Arc<ResourceProvider>,
    pub godot_single_thread: Arc<Semaphore>,
    pub texture_quality: TextureQuality,
}

impl ProcessorContext {
    pub fn new(
        content_folder: String,
        output_folder: String,
        resource_provider: Arc<ResourceProvider>,
    ) -> Self {
        Self {
            content_folder: Arc::new(content_folder),
            output_folder: Arc::new(output_folder),
            resource_provider,
            godot_single_thread: Arc::new(Semaphore::new(1)),
            texture_quality: TextureQuality::Medium,
        }
    }

    /// Convert to SceneGltfContext for GLTF loading functions.
    /// Sets force_compress=true since asset server always produces mobile-optimized output.
    pub fn to_scene_context(&self) -> SceneGltfContext {
        SceneGltfContext {
            content_folder: self.content_folder.clone(),
            resource_provider: self.resource_provider.clone(),
            godot_single_thread: self.godot_single_thread.clone(),
            texture_quality: self.texture_quality.clone(),
            force_compress: true, // Asset server always compresses for mobile
        }
    }

    /// Convert to ContentProviderContext for texture loading.
    pub fn to_content_context(&self) -> crate::content::content_provider::ContentProviderContext {
        use crate::godot_classes::dcl_global::DclGlobal;
        use crate::http_request::http_queue_requester::HttpQueueRequester;

        crate::content::content_provider::ContentProviderContext {
            content_folder: self.content_folder.clone(),
            resource_provider: self.resource_provider.clone(),
            godot_single_thread: self.godot_single_thread.clone(),
            texture_quality: self.texture_quality.clone(),
            http_queue_requester: Arc::new(HttpQueueRequester::new(
                6,
                DclGlobal::get_network_inspector_sender(),
            )),
        }
    }
}

/// Result of processing an asset, including metadata.
pub struct ProcessResult {
    /// Path to the optimized asset
    pub optimized_path: String,
    /// Original texture size (for textures only)
    pub original_size: Option<(u32, u32)>,
    /// Optimized file size in bytes
    pub optimized_file_size: Option<u64>,
    /// GLTF dependencies - texture hashes (for GLTFs only)
    pub gltf_dependencies: Option<Vec<String>>,
}

/// Process an asset request.
///
/// This function:
/// 1. Updates job status to Downloading
/// 2. Downloads the asset and dependencies
/// 3. Updates job status to Processing
/// 4. Processes the asset (GLTF loading, texture conversion, etc.)
/// 5. Saves to disk cache
/// 6. Marks job as Completed with the optimized path
pub async fn process_asset(
    request: AssetRequest,
    job_id: String,
    job_manager: Arc<JobManager>,
    ctx: ProcessorContext,
) {
    // Acquire a permit to limit concurrent jobs
    let _permit = job_manager.acquire_permit().await;

    job_manager
        .update_progress(&job_id, JobStatus::Downloading, 0.1)
        .await;

    let result = match request.asset_type {
        AssetType::Scene => process_scene_gltf(&request, &ctx).await,
        AssetType::Wearable => process_wearable_gltf(&request, &ctx).await,
        AssetType::Emote => process_emote_gltf(&request, &ctx).await,
        AssetType::Texture => process_texture(&request, &ctx).await,
    };

    match result {
        Ok(process_result) => {
            tracing::info!(
                "Asset processed successfully: {} -> {}",
                request.hash,
                process_result.optimized_path
            );

            // Set metadata before completing the job
            if let Some((width, height)) = process_result.original_size {
                job_manager
                    .set_texture_original_size(&job_id, width, height)
                    .await;
            }
            if let Some(size) = process_result.optimized_file_size {
                job_manager.set_optimized_file_size(&job_id, size).await;
            }
            if let Some(deps) = process_result.gltf_dependencies {
                job_manager.set_gltf_dependencies(&job_id, deps).await;
            }

            job_manager
                .complete_job(&job_id, process_result.optimized_path)
                .await;
        }
        Err(e) => {
            tracing::error!("Asset processing failed for {}: {}", request.hash, e);
            job_manager.fail_job(&job_id, e.to_string()).await;
        }
    }
}

/// Build content mapping from the request.
fn build_content_mapping(request: &AssetRequest) -> ContentMappingAndUrlRef {
    use crate::dcl::common::content_entity::TypedIpfsRef;

    // Convert HashMap to Vec<TypedIpfsRef>
    let content: Vec<TypedIpfsRef> = request
        .content_mapping
        .iter()
        .map(|(file_path, hash)| TypedIpfsRef {
            file: file_path.clone(),
            hash: hash.clone(),
        })
        .collect();

    Arc::new(ContentMappingAndUrl::from_base_url_and_content(
        request.base_url.clone(),
        content,
    ))
}

/// Process a scene GLTF.
async fn process_scene_gltf(
    request: &AssetRequest,
    ctx: &ProcessorContext,
) -> Result<ProcessResult, anyhow::Error> {
    tracing::info!("Processing scene GLTF: {}", request.hash);

    let content_mapping = build_content_mapping(request);

    // Find the file path for this hash
    let file_path = find_file_path_for_hash(&content_mapping, &request.hash)
        .ok_or_else(|| anyhow::anyhow!("Hash not found in content mapping"))?;

    // Get base path for resolving relative dependencies
    let base_path = get_base_dir(&file_path);

    // Download the GLTF file first to extract dependencies
    let gltf_file_path = format!("{}{}", ctx.content_folder, request.hash);
    ctx.resource_provider
        .fetch_resource(
            request.url.clone(),
            request.hash.clone(),
            gltf_file_path.clone(),
        )
        .await
        .map_err(anyhow::Error::msg)?;

    // Extract actual texture dependencies from the downloaded GLTF file
    let gltf_dependencies =
        extract_gltf_texture_dependencies(&gltf_file_path, &base_path, &content_mapping).await;

    // For GLTFs with no external dependencies, try to get embedded texture size
    let original_size = if gltf_dependencies.is_empty() {
        get_embedded_texture_size(&gltf_file_path)
            .await
    } else {
        None
    };

    let scene_ctx = ctx.to_scene_context();

    let optimized_path = load_and_save_scene_gltf(
        file_path,
        request.hash.clone(),
        content_mapping.clone(),
        scene_ctx,
    )
    .await?;

    // Get file size
    let optimized_file_size = std::fs::metadata(&optimized_path).ok().map(|m| m.len());

    Ok(ProcessResult {
        optimized_path,
        original_size,
        optimized_file_size,
        gltf_dependencies: Some(gltf_dependencies),
    })
}

/// Process a wearable GLTF.
async fn process_wearable_gltf(
    request: &AssetRequest,
    ctx: &ProcessorContext,
) -> Result<ProcessResult, anyhow::Error> {
    tracing::info!("Processing wearable GLTF: {}", request.hash);

    let content_mapping = build_content_mapping(request);

    // Find the file path for this hash
    let file_path = find_file_path_for_hash(&content_mapping, &request.hash)
        .ok_or_else(|| anyhow::anyhow!("Hash not found in content mapping"))?;

    // Get base path for resolving relative dependencies
    let base_path = get_base_dir(&file_path);

    // Download the GLTF file first to extract dependencies
    let gltf_file_path = format!("{}{}", ctx.content_folder, request.hash);
    ctx.resource_provider
        .fetch_resource(
            request.url.clone(),
            request.hash.clone(),
            gltf_file_path.clone(),
        )
        .await
        .map_err(anyhow::Error::msg)?;

    // Extract actual texture dependencies from the downloaded GLTF file
    let gltf_dependencies =
        extract_gltf_texture_dependencies(&gltf_file_path, &base_path, &content_mapping).await;

    // For GLTFs with no external dependencies, try to get embedded texture size
    let original_size = if gltf_dependencies.is_empty() {
        get_embedded_texture_size(&gltf_file_path)
            .await
    } else {
        None
    };

    let scene_ctx = ctx.to_scene_context();

    let optimized_path = load_and_save_wearable_gltf(
        file_path,
        request.hash.clone(),
        content_mapping.clone(),
        scene_ctx,
    )
    .await?;

    // Get file size
    let optimized_file_size = std::fs::metadata(&optimized_path).ok().map(|m| m.len());

    Ok(ProcessResult {
        optimized_path,
        original_size,
        optimized_file_size,
        gltf_dependencies: Some(gltf_dependencies),
    })
}

/// Process an emote GLTF.
async fn process_emote_gltf(
    request: &AssetRequest,
    ctx: &ProcessorContext,
) -> Result<ProcessResult, anyhow::Error> {
    tracing::info!("Processing emote GLTF: {}", request.hash);

    let content_mapping = build_content_mapping(request);

    // Find the file path for this hash
    let file_path = find_file_path_for_hash(&content_mapping, &request.hash)
        .ok_or_else(|| anyhow::anyhow!("Hash not found in content mapping"))?;

    // Get base path for resolving relative dependencies
    let base_path = get_base_dir(&file_path);

    // Download the GLTF file first to extract dependencies
    let gltf_file_path = format!("{}{}", ctx.content_folder, request.hash);
    ctx.resource_provider
        .fetch_resource(
            request.url.clone(),
            request.hash.clone(),
            gltf_file_path.clone(),
        )
        .await
        .map_err(anyhow::Error::msg)?;

    // Extract actual texture dependencies from the downloaded GLTF file
    let gltf_dependencies =
        extract_gltf_texture_dependencies(&gltf_file_path, &base_path, &content_mapping).await;

    // For GLTFs with no external dependencies, try to get embedded texture size
    let original_size = if gltf_dependencies.is_empty() {
        get_embedded_texture_size(&gltf_file_path)
            .await
    } else {
        None
    };

    let scene_ctx = ctx.to_scene_context();

    let optimized_path = load_and_save_emote_gltf(
        file_path,
        request.hash.clone(),
        content_mapping.clone(),
        scene_ctx,
    )
    .await?;

    // Get file size
    let optimized_file_size = std::fs::metadata(&optimized_path).ok().map(|m| m.len());

    Ok(ProcessResult {
        optimized_path,
        original_size,
        optimized_file_size,
        gltf_dependencies: Some(gltf_dependencies),
    })
}

/// Process a texture and save as compressed .ctex format.
///
/// Downloads the texture, compresses it using ETC2 (mobile-optimized),
/// and saves as a PortableCompressedTexture2D (.ctex) file.
async fn process_texture(
    request: &AssetRequest,
    ctx: &ProcessorContext,
) -> Result<ProcessResult, anyhow::Error> {
    tracing::info!("Processing texture: {}", request.hash);

    let raw_file_path = format!("{}{}", ctx.content_folder, request.hash);
    // Use .res extension for compressed ImageTexture (Godot binary resource format)
    let res_file_path = format!("{}{}.res", ctx.content_folder, request.hash);

    // Download the raw image file
    let bytes_vec = ctx
        .resource_provider
        .fetch_resource_with_data(&request.url, &request.hash, &raw_file_path)
        .await
        .map_err(anyhow::Error::msg)?;

    if bytes_vec.is_empty() {
        return Err(anyhow::anyhow!("Empty texture data"));
    }

    // Check for unsupported formats
    if infer_mime::is_avif(&bytes_vec) {
        return Err(anyhow::anyhow!("Unsupported image format: AVIF"));
    }
    if infer_mime::is_heic(&bytes_vec) {
        return Err(anyhow::anyhow!("Unsupported image format: HEIC"));
    }

    // Animated formats (GIF, animated WebP) are not supported for .ctex compression
    if infer_mime::is_gif(&bytes_vec) {
        return Err(anyhow::anyhow!(
            "Animated GIF not supported for .ctex compression"
        ));
    }
    if infer_mime::is_animated_webp(&bytes_vec) {
        return Err(anyhow::anyhow!(
            "Animated WebP not supported for .ctex compression"
        ));
    }

    // Acquire Godot thread for texture processing
    let _thread_safe_check = GodotSingleThreadSafety::acquire_owned(&ctx.to_content_context())
        .await
        .ok_or_else(|| anyhow::anyhow!("Failed to acquire Godot thread"))?;

    // Load image from bytes
    let bytes = PackedByteArray::from_vec(&bytes_vec);
    let mut image = Image::new_gd();

    let err = if infer_mime::is_png(&bytes_vec) {
        image.load_png_from_buffer(&bytes)
    } else if infer_mime::is_jpeg(&bytes_vec) {
        image.load_jpg_from_buffer(&bytes)
    } else if infer_mime::is_webp(&bytes_vec) {
        image.load_webp_from_buffer(&bytes)
    } else if infer_mime::is_bmp(&bytes_vec) {
        image.load_bmp_from_buffer(&bytes)
    } else if infer_mime::is_tga(&bytes_vec) {
        image.load_tga_from_buffer(&bytes)
    } else if infer_mime::is_ktx(&bytes_vec) {
        image.load_ktx_from_buffer(&bytes)
    } else {
        // Try PNG as fallback
        image.load_png_from_buffer(&bytes)
    };

    if err != godot::global::Error::OK {
        return Err(anyhow::anyhow!("Failed to load image: {:?}", err));
    }

    // Capture original size before resizing
    let original_width = image.get_width() as u32;
    let original_height = image.get_height() as u32;

    // Verify image was loaded successfully
    if original_width == 0 || original_height == 0 {
        return Err(anyhow::anyhow!(
            "Image loaded with invalid dimensions: {}x{}",
            original_width,
            original_height
        ));
    }

    let image_format = image.get_format();
    tracing::debug!(
        "Loaded image {}x{} format={:?}",
        original_width,
        original_height,
        image_format
    );

    // Resize if needed based on texture quality
    let max_size = ctx.texture_quality.to_max_size();
    resize_image_if_needed(&mut image, max_size);

    // Compress the image using ETC2 (mobile-optimized format)
    // This uses the same approach as texture.rs which works on mobile platforms
    if !image.is_compressed() {
        tracing::info!(
            "Compressing image {}x{} with ETC2",
            image.get_width(),
            image.get_height()
        );
        image.compress(CompressMode::ETC2);
    }

    // Create ImageTexture from the compressed image
    let texture = ImageTexture::create_from_image(&image).ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to create ImageTexture from compressed image ({}x{})",
            image.get_width(),
            image.get_height()
        )
    })?;

    let texture_width = texture.get_width();
    let texture_height = texture.get_height();
    tracing::info!(
        "Created compressed texture: {}x{}",
        texture_width,
        texture_height
    );

    // Verify compression succeeded
    if texture_width == 0 || texture_height == 0 {
        return Err(anyhow::anyhow!(
            "Texture compression failed - resulting texture has no dimensions"
        ));
    }

    // Save as .res file (Godot binary resource format)
    // We use .res instead of .ctex because ImageTexture doesn't support .ctex
    let err = ResourceSaver::singleton()
        .save_ex(&texture.upcast::<Resource>())
        .path(&res_file_path)
        .flags(SaverFlags::COMPRESS)
        .done();

    if err != godot::global::Error::OK {
        return Err(anyhow::anyhow!(
            "Failed to save compressed texture to '{}': {:?}",
            res_file_path,
            err
        ));
    }

    // Get optimized file size
    let optimized_file_size = std::fs::metadata(&res_file_path).ok().map(|m| m.len());

    tracing::info!(
        "Texture compressed and saved: {} -> {} (original: {}x{}, size: {:?})",
        request.hash,
        res_file_path,
        original_width,
        original_height,
        optimized_file_size
    );

    Ok(ProcessResult {
        optimized_path: res_file_path,
        original_size: Some((original_width, original_height)),
        optimized_file_size,
        gltf_dependencies: None,
    })
}

/// Resize image if it exceeds max size while maintaining aspect ratio.
fn resize_image_if_needed(image: &mut Gd<Image>, max_size: i32) {
    let width = image.get_width();
    let height = image.get_height();

    if width <= max_size && height <= max_size {
        return;
    }

    let (new_width, new_height) = if width > height {
        let ratio = max_size as f32 / width as f32;
        (max_size, (height as f32 * ratio) as i32)
    } else {
        let ratio = max_size as f32 / height as f32;
        ((width as f32 * ratio) as i32, max_size)
    };

    image.resize(new_width, new_height);
}

/// Find the file path that maps to a given hash.
fn find_file_path_for_hash(
    content_mapping: &ContentMappingAndUrlRef,
    hash: &str,
) -> Option<String> {
    for (file_path, file_hash) in content_mapping.files() {
        if file_hash == hash {
            return Some(file_path.clone());
        }
    }
    None
}

/// Image file extensions for texture dependency extraction.
const IMAGE_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".webp", ".bmp", ".tga", ".ktx", ".ktx2",
];

/// Get the base directory from a file path.
fn get_base_dir(file_path: &str) -> String {
    if let Some(pos) = file_path.rfind('/') {
        file_path[..pos].to_string()
    } else {
        String::new()
    }
}

/// Check if a file path is an image file.
fn is_image_file(file_path: &str) -> bool {
    let lower = file_path.to_lowercase();
    IMAGE_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Extract actual texture dependencies from a GLTF file.
/// Parses the GLTF to find referenced images and maps them to content hashes.
async fn extract_gltf_texture_dependencies(
    gltf_file_path: &str,
    base_path: &str,
    content_mapping: &ContentMappingAndUrlRef,
) -> Vec<String> {
    // Get dependencies from the GLTF file
    let dependencies = match get_dependencies(gltf_file_path).await {
        Ok(deps) => deps,
        Err(e) => {
            tracing::warn!("Failed to extract GLTF dependencies: {}", e);
            return Vec::new();
        }
    };

    // Map dependency file paths to hashes, filtering only image files
    dependencies
        .into_iter()
        .filter_map(|dep| {
            // Only include image files (not buffers)
            if !is_image_file(&dep) {
                return None;
            }

            // Build full path (same logic as in gltf/common.rs)
            let full_path = if base_path.is_empty() {
                dep.clone()
            } else {
                format!("{}/{}", base_path, dep)
            };

            // Look up hash in content mapping
            content_mapping.get_hash(&full_path).cloned()
        })
        .collect()
}

/// Create default processor context.
pub fn create_default_context() -> ProcessorContext {
    let content_folder = format!("{}/content/", Os::singleton().get_user_data_dir());

    // Output folder for ZIP files - use env var or default to ./output/
    let output_folder =
        std::env::var("ASSET_SERVER_OUTPUT_DIR").unwrap_or_else(|_| "./output/".to_string());

    // Create output directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&output_folder) {
        tracing::warn!(
            "Failed to create output directory '{}': {}",
            output_folder,
            e
        );
    }

    // Convert to absolute path for consistent paths in responses
    let output_folder = std::fs::canonicalize(&output_folder)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or(output_folder);

    // Ensure output folder ends with /
    let output_folder = if output_folder.ends_with('/') {
        output_folder
    } else {
        format!("{}/", output_folder)
    };

    let resource_provider = Arc::new(ResourceProvider::new(
        &content_folder,
        5 * 1024 * 1000 * 1000, // 5GB cache for asset server mode
        32,
        #[cfg(feature = "use_resource_tracking")]
        Arc::new(crate::content::resource_download_tracking::ResourceDownloadTracking::new()),
    ));

    ProcessorContext::new(content_folder, output_folder, resource_provider)
}
