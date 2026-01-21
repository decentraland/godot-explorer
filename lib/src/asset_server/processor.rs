//! Asset processing orchestration.
//!
//! This module calls the existing GLTF/texture loading functions from the content module.

use std::sync::Arc;

use godot::classes::Os;
use godot::obj::Singleton;
use tokio::sync::Semaphore;

use crate::content::content_mapping::{ContentMappingAndUrl, ContentMappingAndUrlRef};
use crate::content::content_provider::SceneGltfContext;
use crate::content::gltf::{
    load_and_save_emote_gltf, load_and_save_scene_gltf, load_and_save_wearable_gltf,
};
use crate::content::resource_provider::ResourceProvider;
use crate::content::texture::load_image_texture;
use crate::godot_classes::dcl_config::TextureQuality;

use super::job_manager::JobManager;
use super::types::{AssetRequest, AssetType, JobStatus};

/// Context for asset processing, similar to ContentProviderContext but standalone.
#[derive(Clone)]
pub struct ProcessorContext {
    pub content_folder: Arc<String>,
    pub resource_provider: Arc<ResourceProvider>,
    pub godot_single_thread: Arc<Semaphore>,
    pub texture_quality: TextureQuality,
}

impl ProcessorContext {
    pub fn new(content_folder: String, resource_provider: Arc<ResourceProvider>) -> Self {
        Self {
            content_folder: Arc::new(content_folder),
            resource_provider,
            godot_single_thread: Arc::new(Semaphore::new(1)),
            texture_quality: TextureQuality::Medium,
        }
    }

    /// Convert to SceneGltfContext for GLTF loading functions.
    pub fn to_scene_context(&self) -> SceneGltfContext {
        SceneGltfContext {
            content_folder: self.content_folder.clone(),
            resource_provider: self.resource_provider.clone(),
            godot_single_thread: self.godot_single_thread.clone(),
            texture_quality: self.texture_quality.clone(),
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
        Ok(optimized_path) => {
            tracing::info!(
                "Asset processed successfully: {} -> {}",
                request.hash,
                optimized_path
            );
            job_manager.complete_job(&job_id, optimized_path).await;
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
) -> Result<String, anyhow::Error> {
    tracing::info!("Processing scene GLTF: {}", request.hash);

    let content_mapping = build_content_mapping(request);

    // Find the file path for this hash
    let file_path = find_file_path_for_hash(&content_mapping, &request.hash)
        .ok_or_else(|| anyhow::anyhow!("Hash not found in content mapping"))?;

    let scene_ctx = ctx.to_scene_context();

    load_and_save_scene_gltf(file_path, request.hash.clone(), content_mapping, scene_ctx).await
}

/// Process a wearable GLTF.
async fn process_wearable_gltf(
    request: &AssetRequest,
    ctx: &ProcessorContext,
) -> Result<String, anyhow::Error> {
    tracing::info!("Processing wearable GLTF: {}", request.hash);

    let content_mapping = build_content_mapping(request);

    // Find the file path for this hash
    let file_path = find_file_path_for_hash(&content_mapping, &request.hash)
        .ok_or_else(|| anyhow::anyhow!("Hash not found in content mapping"))?;

    let scene_ctx = ctx.to_scene_context();

    load_and_save_wearable_gltf(file_path, request.hash.clone(), content_mapping, scene_ctx).await
}

/// Process an emote GLTF.
async fn process_emote_gltf(
    request: &AssetRequest,
    ctx: &ProcessorContext,
) -> Result<String, anyhow::Error> {
    tracing::info!("Processing emote GLTF: {}", request.hash);

    let content_mapping = build_content_mapping(request);

    // Find the file path for this hash
    let file_path = find_file_path_for_hash(&content_mapping, &request.hash)
        .ok_or_else(|| anyhow::anyhow!("Hash not found in content mapping"))?;

    let scene_ctx = ctx.to_scene_context();

    load_and_save_emote_gltf(file_path, request.hash.clone(), content_mapping, scene_ctx).await
}

/// Process a texture.
async fn process_texture(
    request: &AssetRequest,
    ctx: &ProcessorContext,
) -> Result<String, anyhow::Error> {
    tracing::info!("Processing texture: {}", request.hash);

    let content_ctx = ctx.to_content_context();

    // The result is a Variant containing TextureEntry, but we just want to confirm it succeeded
    // and return the cached path
    let _ = load_image_texture(request.url.clone(), request.hash.clone(), content_ctx).await?;

    // Return the path where the texture is cached
    Ok(format!("{}{}", ctx.content_folder, request.hash))
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

/// Create default processor context.
pub fn create_default_context() -> ProcessorContext {
    let content_folder = format!("{}/content/", Os::singleton().get_user_data_dir());

    let resource_provider = Arc::new(ResourceProvider::new(
        &content_folder,
        2048 * 1000 * 1000, // 2GB cache
        32,
        #[cfg(feature = "use_resource_tracking")]
        Arc::new(crate::content::resource_download_tracking::ResourceDownloadTracking::new()),
    ));

    ProcessorContext::new(content_folder, resource_provider)
}
