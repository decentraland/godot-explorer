//! HTTP request handlers for the asset server.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use super::bundle_writer::{write_bundle, BundleEntry};
use super::job_manager::JobManager;
use super::packer::{pack_assets_to_zip, pack_scene_assets_to_zip};
use super::processor::{process_asset, ProcessorContext};
use super::scene_fetcher::fetch_scene_entity;
use super::static_models::{scan_main_crdt, static_bundle_entries};
use super::types::{
    AssetRequest, AssetType, BatchStatus, BatchStatusResponse, BatchSummary, HealthResponse,
    JobResponse, JobsResponse, ProcessRequest, ProcessResponse, ProcessSceneRequest,
    ProcessSceneResponse, StatusResponse,
};

/// Handle POST /process request.
///
/// Creates processing jobs for all assets, creates a batch, and spawns a watcher
/// to pack the results into a ZIP when all jobs complete.
pub async fn handle_process(
    request: ProcessRequest,
    job_manager: Arc<JobManager>,
    ctx: ProcessorContext,
) -> Result<ProcessResponse, String> {
    let total = request.assets.len();

    if total == 0 {
        return Err("No assets provided in request".to_string());
    }

    // Determine output_hash
    let output_hash = match request.output_hash {
        Some(hash) => hash,
        None if total == 1 => request.assets[0].hash.clone(),
        None => return Err("output_hash is required when processing multiple assets".to_string()),
    };

    tracing::info!(
        "Processing {} assets with output_hash: {}",
        total,
        output_hash
    );

    let mut jobs = Vec::with_capacity(total);
    let mut job_ids = Vec::with_capacity(total);

    for asset in request.assets {
        match process_single_asset(asset, job_manager.clone(), ctx.clone()).await {
            Ok(response) => {
                if !response.job_id.is_empty() {
                    job_ids.push(response.job_id.clone());
                }
                jobs.push(response);
            }
            Err((hash, e)) => {
                // Create a failed response for this asset
                jobs.push(JobResponse {
                    job_id: String::new(),
                    hash,
                    status: super::types::JobStatus::Failed,
                });
                tracing::warn!("Failed to create job: {}", e);
            }
        }
    }

    // Create batch to track all jobs
    let batch_id = job_manager.create_batch(output_hash.clone(), job_ids).await;

    tracing::debug!("Created batch {} for {} jobs", batch_id, jobs.len());

    // Spawn batch completion watcher
    let batch_id_clone = batch_id.clone();
    let job_manager_clone = job_manager.clone();
    let ctx_clone = ctx.clone();
    tokio::spawn(async move {
        watch_and_pack_batch(batch_id_clone, job_manager_clone, ctx_clone).await;
    });

    Ok(ProcessResponse {
        batch_id,
        output_hash,
        jobs,
        total,
    })
}

/// Watch for batch completion and pack results into a ZIP file.
async fn watch_and_pack_batch(
    batch_id: String,
    job_manager: Arc<JobManager>,
    ctx: ProcessorContext,
) {
    tracing::debug!("Starting batch watcher for {}", batch_id);

    // Poll until all jobs complete
    loop {
        if job_manager.is_batch_complete(&batch_id).await {
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    tracing::info!("Batch {} complete, starting packing", batch_id);

    // All jobs done - pack into ZIP
    job_manager
        .update_batch_status(&batch_id, BatchStatus::Packing)
        .await;

    let results = job_manager.get_batch_results(&batch_id).await;
    let output_hash = match job_manager.get_batch_output_hash(&batch_id).await {
        Some(hash) => hash,
        None => {
            job_manager
                .fail_batch(&batch_id, "Batch not found".to_string())
                .await;
            return;
        }
    };

    if results.is_empty() {
        job_manager
            .fail_batch(&batch_id, "No assets completed successfully".to_string())
            .await;
        return;
    }

    // Acquire Godot thread for ZIPPacker
    let _permit = ctx.godot_single_thread.acquire().await;

    match pack_assets_to_zip(&output_hash, results, &ctx.output_folder) {
        Ok(zip_path) => {
            tracing::info!("Batch {} packed to {}", batch_id, zip_path);
            job_manager.complete_batch(&batch_id, zip_path).await;
        }
        Err(e) => {
            tracing::error!("Failed to pack batch {}: {}", batch_id, e);
            job_manager.fail_batch(&batch_id, e.to_string()).await;
        }
    }
}

/// Process a single asset request.
async fn process_single_asset(
    asset: AssetRequest,
    job_manager: Arc<JobManager>,
    ctx: ProcessorContext,
) -> Result<JobResponse, (String, String)> {
    // Validate request
    if asset.hash.is_empty() {
        return Err((String::new(), "Missing required field: hash".to_string()));
    }
    if asset.url.is_empty() {
        return Err((asset.hash, "Missing required field: url".to_string()));
    }
    if asset.base_url.is_empty() {
        return Err((asset.hash, "Missing required field: base_url".to_string()));
    }

    let hash = asset.hash.clone();

    // Create job (or get existing one)
    let job_id = match job_manager
        .create_job(asset.hash.clone(), asset.asset_type)
        .await
    {
        Ok(id) => id,
        Err(existing_id) => {
            // Job already exists for this hash - return existing job
            tracing::debug!(
                "Job already exists for hash {}: {}",
                asset.hash,
                existing_id
            );
            return Ok(JobResponse {
                job_id: existing_id,
                hash,
                status: super::types::JobStatus::Queued,
            });
        }
    };

    tracing::debug!(
        "Created job {} for {} ({})",
        job_id,
        asset.hash,
        asset.asset_type.as_str()
    );

    // Spawn processing task
    let job_id_clone = job_id.clone();
    let job_manager_clone = job_manager.clone();
    tokio::spawn(async move {
        process_asset(asset, job_id_clone, job_manager_clone, ctx).await;
    });

    Ok(JobResponse {
        job_id,
        hash,
        status: super::types::JobStatus::Queued,
    })
}

/// Handle GET /status/{batch_id} request.
///
/// Returns the current status of a batch, including all its jobs.
pub async fn handle_batch_status(
    batch_id: String,
    job_manager: Arc<JobManager>,
) -> Result<BatchStatusResponse, String> {
    let batch = match job_manager.get_batch(&batch_id).await {
        Some(b) => b,
        None => return Err(format!("Batch not found: {}", batch_id)),
    };

    // Get all jobs in this batch
    let all_jobs = job_manager.get_all_jobs().await;
    let batch_jobs: Vec<StatusResponse> = all_jobs
        .iter()
        .filter(|job| batch.job_ids.contains(&job.id))
        .map(StatusResponse::from)
        .collect();

    // Calculate progress based on job statuses
    let completed_count = batch_jobs
        .iter()
        .filter(|j| {
            matches!(
                j.status,
                super::types::JobStatus::Completed | super::types::JobStatus::Failed
            )
        })
        .count();
    let progress = if batch_jobs.is_empty() {
        1.0
    } else {
        completed_count as f32 / batch_jobs.len() as f32
    };

    Ok(BatchStatusResponse {
        batch_id: batch.id,
        output_hash: batch.output_hash,
        status: batch.status,
        progress,
        jobs: batch_jobs,
        zip_path: batch.zip_path,
        error: batch.error,
        individual_zips: batch.individual_zips,
    })
}

/// Handle GET /status/job/{job_id} request.
///
/// Returns the current status of a single job.
pub async fn handle_job_status(
    job_id: String,
    job_manager: Arc<JobManager>,
) -> Result<StatusResponse, String> {
    match job_manager.get_job(&job_id).await {
        Some(job) => Ok(StatusResponse::from(&job)),
        None => Err(format!("Job not found: {}", job_id)),
    }
}

/// Handle GET /jobs request.
///
/// Returns all jobs and batches.
pub async fn handle_jobs(job_manager: Arc<JobManager>) -> JobsResponse {
    let jobs = job_manager.get_all_jobs().await;
    let batches = job_manager.get_all_batches().await;

    JobsResponse {
        jobs: jobs.iter().map(StatusResponse::from).collect(),
        batches: batches
            .iter()
            .map(|b| BatchSummary {
                batch_id: b.id.clone(),
                output_hash: b.output_hash.clone(),
                status: b.status,
                job_count: b.job_ids.len(),
                zip_path: b.zip_path.clone(),
                elapsed_secs: b.created_at.elapsed().as_secs_f64(),
            })
            .collect(),
    }
}

/// Handle GET /health request.
///
/// Returns server health status.
pub fn handle_health() -> HealthResponse {
    HealthResponse {
        status: "ok".to_string(),
    }
}

/// Handle POST /process-scene request.
///
/// Fetches a scene entity, discovers all assets, creates processing jobs,
/// and spawns a watcher to pack results with metadata.
pub async fn handle_process_scene(
    request: ProcessSceneRequest,
    job_manager: Arc<JobManager>,
    ctx: ProcessorContext,
) -> Result<ProcessSceneResponse, String> {
    tracing::info!(
        "Processing scene {} from {}",
        request.scene_hash,
        request.content_base_url
    );

    // Fetch scene entity and discover assets
    let scene_assets = fetch_scene_entity(&request.content_base_url, &request.scene_hash)
        .await
        .map_err(|e| format!("Failed to fetch scene entity: {}", e))?;

    let total_assets = scene_assets.total_count();

    if total_assets == 0 {
        return Err("No processable assets found in scene".to_string());
    }

    // Determine output_hash
    let output_hash = request
        .output_hash
        .unwrap_or_else(|| request.scene_hash.clone());

    // Build preloaded hashes set
    let preloaded_hashes = request
        .preloaded_hashes
        .map(|hashes| hashes.into_iter().collect::<HashSet<String>>());
    let preloaded_assets = preloaded_hashes.as_ref().map(|h| h.len());

    tracing::debug!(
        "Scene {} has {} assets, preloaded: {:?}",
        request.scene_hash,
        total_assets,
        preloaded_assets
    );

    let mut jobs = Vec::with_capacity(total_assets);
    // One job per hash: a deployment can list the same bytes under two paths
    // (same hash), and `process_single_scene_asset` hands back the existing
    // job for the duplicate. Listing it twice in the batch would publish —
    // and make the uploader upload — the same file twice.
    let mut job_ids: Vec<String> = Vec::with_capacity(total_assets);
    let mut seen_job_ids: HashSet<String> = HashSet::with_capacity(total_assets);

    let cache_only = request.cache_only;

    // Create jobs for all GLTF assets
    for asset in &scene_assets.gltfs {
        let asset_request = AssetRequest {
            url: asset.url.clone(),
            asset_type: AssetType::Scene,
            hash: asset.hash.clone(),
            base_url: scene_assets.content_base_url.clone(),
            content_mapping: scene_assets.content_mapping.clone(),
            cache_only,
        };

        match process_single_scene_asset(asset_request, job_manager.clone(), ctx.clone()).await {
            Ok(response) => {
                if !response.job_id.is_empty() && seen_job_ids.insert(response.job_id.clone()) {
                    job_ids.push(response.job_id.clone());
                }
                jobs.push(response);
            }
            Err((hash, e)) => {
                jobs.push(JobResponse {
                    job_id: String::new(),
                    hash,
                    status: super::types::JobStatus::Failed,
                });
                tracing::warn!("Failed to create GLTF job: {}", e);
            }
        }
    }

    // Create jobs for all texture assets
    for asset in &scene_assets.textures {
        let asset_request = AssetRequest {
            url: asset.url.clone(),
            asset_type: AssetType::Texture,
            hash: asset.hash.clone(),
            base_url: scene_assets.content_base_url.clone(),
            content_mapping: Default::default(), // Textures don't need content mapping
            cache_only,
        };

        match process_single_scene_asset(asset_request, job_manager.clone(), ctx.clone()).await {
            Ok(response) => {
                if !response.job_id.is_empty() && seen_job_ids.insert(response.job_id.clone()) {
                    job_ids.push(response.job_id.clone());
                }
                jobs.push(response);
            }
            Err((hash, e)) => {
                jobs.push(JobResponse {
                    job_id: String::new(),
                    hash,
                    status: super::types::JobStatus::Failed,
                });
                tracing::warn!("Failed to create texture job: {}", e);
            }
        }
    }

    // Create scene batch with preloaded hashes
    let batch_id = job_manager
        .create_scene_batch(
            output_hash.clone(),
            job_ids,
            request.scene_hash.clone(),
            preloaded_hashes,
            scene_assets.boot_files.clone(),
            Arc::new(scene_assets.content_mapping.clone()),
        )
        .await;

    tracing::debug!("Created scene batch {} for {} jobs", batch_id, jobs.len());

    // Spawn batch completion watcher
    let batch_id_clone = batch_id.clone();
    let job_manager_clone = job_manager.clone();
    let ctx_clone = ctx.clone();
    tokio::spawn(async move {
        watch_and_pack_scene_batch(batch_id_clone, job_manager_clone, ctx_clone).await;
    });

    Ok(ProcessSceneResponse {
        batch_id,
        output_hash,
        scene_hash: request.scene_hash,
        total_assets,
        preloaded_assets,
        jobs,
    })
}

/// Process a single asset from a scene.
async fn process_single_scene_asset(
    asset: AssetRequest,
    job_manager: Arc<JobManager>,
    ctx: ProcessorContext,
) -> Result<JobResponse, (String, String)> {
    // Validate request
    if asset.hash.is_empty() {
        return Err((String::new(), "Missing required field: hash".to_string()));
    }
    if asset.url.is_empty() {
        return Err((asset.hash, "Missing required field: url".to_string()));
    }

    let hash = asset.hash.clone();

    // Create job (or get existing one)
    let job_id = match job_manager
        .create_job(asset.hash.clone(), asset.asset_type)
        .await
    {
        Ok(id) => id,
        Err(existing_id) => {
            // Job already exists for this hash - return existing job
            tracing::debug!(
                "Job already exists for hash {}: {}",
                asset.hash,
                existing_id
            );
            return Ok(JobResponse {
                job_id: existing_id,
                hash,
                status: super::types::JobStatus::Queued,
            });
        }
    };

    tracing::debug!(
        "Created job {} for {} ({})",
        job_id,
        asset.hash,
        asset.asset_type.as_str()
    );

    // Spawn processing task
    let job_id_clone = job_id.clone();
    let job_manager_clone = job_manager.clone();
    tokio::spawn(async move {
        process_asset(asset, job_id_clone, job_manager_clone, ctx).await;
    });

    Ok(JobResponse {
        job_id,
        hash,
        status: super::types::JobStatus::Queued,
    })
}

/// Watch for scene batch completion, create individual ZIPs per asset,
/// then create the main metadata ZIP with optional preloaded assets.
async fn watch_and_pack_scene_batch(
    batch_id: String,
    job_manager: Arc<JobManager>,
    ctx: ProcessorContext,
) {
    tracing::debug!("Starting scene batch watcher for {}", batch_id);

    // Poll until all jobs complete
    loop {
        if job_manager.is_batch_complete(&batch_id).await {
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    tracing::info!(
        "Scene batch {} complete, creating individual ZIPs and metadata",
        batch_id
    );

    job_manager
        .update_batch_status(&batch_id, BatchStatus::Packing)
        .await;

    // A scene .scn references its textures as external `res://content/*.res`;
    // never ship one whose textures failed to bake.
    for (hash, missing) in job_manager
        .fail_jobs_with_missing_dependencies(&batch_id)
        .await
    {
        tracing::error!(
            "Scene batch {}: dropping GLB {} — {} external texture(s) failed to bake: {:?}",
            batch_id,
            hash,
            missing.len(),
            missing
        );
    }

    let results = job_manager.get_batch_results(&batch_id).await;
    tracing::debug!(
        "Scene batch {} has {} results with optimized_path",
        batch_id,
        results.len()
    );

    let output_hash = match job_manager.get_batch_output_hash(&batch_id).await {
        Some(hash) => hash,
        None => {
            job_manager
                .fail_batch(&batch_id, "Batch not found".to_string())
                .await;
            return;
        }
    };

    if results.is_empty() {
        tracing::error!(
            "Scene batch {} has NO results! Metadata will be generated but no assets will be packed",
            batch_id
        );
    }

    // Acquire Godot thread for ZIPPacker (held for all ZIP operations)
    let _permit = ctx.godot_single_thread.acquire().await;

    // R17 architecture: per-impostor ShaderMaterial + per-impostor
    // ImageTexture embedded in each .scn. No global atlas finalize
    // step.

    // Get preloaded hashes
    let preloaded_hashes = job_manager.get_batch_preloaded_hashes(&batch_id).await;
    tracing::debug!(
        "Scene batch {} preloaded_hashes: {:?}",
        batch_id,
        preloaded_hashes.as_ref().map(|h| h.len())
    );

    // Build metadata from completed jobs (first: the published name of a scene
    // GLB depends on its texture set, `external_scene_dependencies`).
    let mut metadata = job_manager.build_scene_metadata(&batch_id).await;

    // v6 layout: publish each baked resource as a plain file
    // (`{key}.scn` / `{hash}.res`) — the client downloads it straight into
    // `user://content/` and loads it by path, no resource-pack mount.
    for (hash, path, asset_type) in &results {
        match publish_raw_asset(
            hash,
            path,
            *asset_type,
            &ctx.output_folder,
            &metadata.external_scene_dependencies,
        ) {
            Ok(out_path) => {
                job_manager
                    .add_individual_zip(&batch_id, hash.clone(), out_path)
                    .await;
            }
            Err(e) => {
                tracing::warn!("Failed to publish raw asset for {}: {}", hash, e);
            }
        }
    }

    // Scene boot files (`main.js`, `main.crdt`): published by hash next to the
    // manifest so the client pulls the whole scene from the optimized CDN —
    // one host, HTTP/2, pre-cacheable, and (with the right content types on
    // upload) brotli-compressed: Plaza's 2.26 MB main.js is 390 KB as brotli-11.
    // The content server is the source; a failure here just leaves the file
    // out of the manifest and the client falls back to the content server.
    for boot in job_manager.get_batch_boot_files(&batch_id).await {
        match publish_boot_file(&ctx, &boot).await {
            Ok(out_path) => {
                metadata
                    .boot_files
                    .insert(boot.name.clone(), boot.hash.clone());
                job_manager
                    .add_individual_zip(&batch_id, boot.hash.clone(), out_path)
                    .await;
            }
            Err(e) => tracing::warn!(
                "Scene batch {}: boot file {} ({}) not published: {}",
                batch_id,
                boot.name,
                boot.hash,
                e
            ),
        }
    }

    // Static bundle: every model `main.crdt` composes (plus the textures its
    // .scn files reference) as ONE Stored zip the client extracts into
    // user://content/ — one stream instead of hundreds of small requests.
    // Scenes composed in code (Genesis Plaza: 1 GLB in main.crdt) get a tiny
    // one or none; Creator Hub scenes get most of their world.
    let static_bundle =
        write_static_bundle(&ctx, &batch_id, &output_hash, &metadata, &job_manager).await;
    metadata.static_bundle = static_bundle;
    let boot_zip_name = format!("{}-boot.zip", output_hash);
    metadata.boot_bundle = Some(super::types::BootBundleInfo {
        file: boot_zip_name.clone(),
    });

    // v6 layout: the manifest is a plain `{output_hash}-optimized.json` too.
    // Serialized ONCE: the same bytes go to the file and into the boot zip
    // (serde over HashMaps is not order-stable across calls). Registered after
    // every asset it lists and before the boot zip: `individual_zips` is the
    // uploader's list AND its upload order, and a client fetching mid-upload
    // must never see a manifest whose dependencies are not there yet.
    let manifest_json = match serde_json::to_string(&metadata) {
        Ok(json) => {
            let json_path = format!("{}{}-optimized.json", ctx.output_folder, output_hash);
            match std::fs::write(&json_path, &json) {
                Ok(()) => {
                    job_manager
                        .add_individual_zip(&batch_id, output_hash.clone(), json_path)
                        .await
                }
                Err(e) => tracing::error!("Failed to write {}: {}", json_path, e),
            }
            Some(json)
        }
        Err(e) => {
            tracing::error!("Failed to serialize scene metadata: {}", e);
            None
        }
    };

    // Boot bundle: manifest + main.js + main.crdt in one Deflated zip, entry
    // names == client cache names (bare hash for the boot files).
    if let Some(manifest_json) = manifest_json {
        let mut entries = vec![BundleEntry::Bytes {
            name: format!("{}-optimized.json", output_hash),
            data: manifest_json.into_bytes(),
        }];
        for boot in job_manager.get_batch_boot_files(&batch_id).await {
            if !metadata.boot_files.contains_key(&boot.name) {
                continue; // not published
            }
            entries.push(BundleEntry::File {
                name: boot.hash.clone(),
                path: format!(
                    "{}{}.{}",
                    ctx.output_folder,
                    boot.hash,
                    boot_file_extension(&boot.name)
                ),
            });
        }
        let boot_zip_path = format!("{}{}", ctx.output_folder, boot_zip_name);
        match write_bundle(&boot_zip_path, &entries, zip::CompressionMethod::Deflated) {
            Ok(bytes) => {
                tracing::info!(
                    "Scene batch {}: boot bundle {} ({} entries, {} bytes)",
                    batch_id,
                    boot_zip_name,
                    entries.len(),
                    bytes
                );
                job_manager
                    .add_individual_zip(&batch_id, output_hash.clone(), boot_zip_path)
                    .await;
            }
            Err(e) => tracing::error!("Scene batch {}: boot bundle failed: {}", batch_id, e),
        }
    }

    tracing::debug!(
        "Scene batch {} metadata: {} optimized, {} dependencies, {} sizes",
        batch_id,
        metadata.optimized_content.len(),
        metadata.external_scene_dependencies.len(),
        metadata.original_sizes.len()
    );

    // Create main metadata ZIP (with optional preloaded assets)
    match pack_scene_assets_to_zip(
        &output_hash,
        results,
        preloaded_hashes.as_ref(),
        metadata,
        &ctx.output_folder,
    ) {
        Ok(zip_path) => {
            tracing::info!("Scene batch {} packed to {}", batch_id, zip_path);
            job_manager.complete_batch(&batch_id, zip_path).await;
        }
        Err(e) => {
            tracing::error!("Failed to pack scene batch {}: {}", batch_id, e);
            job_manager.fail_batch(&batch_id, e.to_string()).await;
        }
    }
}

/// Copy a baked asset to the output folder under its published v6 name
/// (`{hash}.scn` for GLBs, `{hash}.res` for textures).
fn publish_raw_asset(
    hash: &str,
    optimized_path: &str,
    asset_type: AssetType,
    output_folder: &str,
    external_scene_dependencies: &HashMap<String, Vec<String>>,
) -> Result<String, anyhow::Error> {
    use crate::content::content_provider::{optimized_remote_name, scene_bake_key, OptimizedKind};
    let (key, kind) = match asset_type {
        AssetType::Texture => (hash.to_string(), OptimizedKind::Texture),
        _ => (
            scene_bake_key(
                hash,
                external_scene_dependencies
                    .get(hash)
                    .into_iter()
                    .flatten()
                    .map(String::as_str),
            ),
            OptimizedKind::Scene,
        ),
    };
    let out_path = format!("{}{}", output_folder, optimized_remote_name(&key, kind));
    std::fs::copy(optimized_path, &out_path)
        .map_err(|e| anyhow::anyhow!("copy {} -> {}: {}", optimized_path, out_path, e))?;
    Ok(out_path)
}

/// Published extension of a scene boot file (`{hash}.js` / `{hash}.crdt`).
pub fn boot_file_extension(name: &str) -> &'static str {
    if name.ends_with(".js") {
        "js"
    } else {
        "crdt"
    }
}

/// Download a scene boot file from the content server and copy it to the
/// output folder as `{hash}.{js|crdt}`.
async fn publish_boot_file(
    ctx: &ProcessorContext,
    boot: &super::scene_fetcher::BootFile,
) -> Result<String, anyhow::Error> {
    let cache_path = format!("{}{}", ctx.content_folder, boot.hash);
    ctx.resource_provider
        .fetch_resource(boot.url.clone(), boot.hash.clone(), cache_path.clone())
        .await
        .map_err(anyhow::Error::msg)?;
    let out_path = format!(
        "{}{}.{}",
        ctx.output_folder,
        boot.hash,
        boot_file_extension(&boot.name)
    );
    std::fs::copy(&cache_path, &out_path)
        .map_err(|e| anyhow::anyhow!("copy {} -> {}: {}", cache_path, out_path, e))?;
    Ok(out_path)
}

/// Scan the batch's `main.crdt` (already downloaded by `publish_boot_file`) and
/// write `{output_hash}-static.zip` with every referenced model and texture,
/// Stored (the payloads are zstd already). Returns the manifest entry, or
/// `None` when the scene has no static composition, the file is unreadable
/// or the zip could not be written — the client then loads per-file as usual.
async fn write_static_bundle(
    ctx: &ProcessorContext,
    batch_id: &str,
    output_hash: &str,
    metadata: &super::types::SceneOptimizationMetadata,
    job_manager: &Arc<JobManager>,
) -> Option<super::types::StaticBundleInfo> {
    let crdt_hash = metadata.boot_files.get("main.crdt")?.clone();
    let crdt_path = format!("{}{}", ctx.content_folder, crdt_hash);
    let bytes = match tokio::fs::read(&crdt_path).await {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!(
                "Scene batch {}: cannot read main.crdt at {}: {}",
                batch_id,
                crdt_path,
                e
            );
            return None;
        }
    };
    let content_mapping = job_manager.get_batch_content_mapping(batch_id).await;
    let assets = scan_main_crdt(&bytes, &content_mapping);
    let entries = static_bundle_entries(&assets, metadata);
    if entries.is_empty() {
        tracing::info!(
            "Scene batch {}: main.crdt references no baked models; no static bundle",
            batch_id
        );
        return None;
    }

    let zip_name = format!("{}-static.zip", output_hash);
    let zip_path = format!("{}{}", ctx.output_folder, zip_name);
    let zip_entries: Vec<BundleEntry> = entries
        .iter()
        .map(|(name, key)| BundleEntry::File {
            name: name.clone(),
            path: format!(
                "{}{}",
                ctx.output_folder,
                crate::content::content_provider::optimized_remote_name(
                    key,
                    if name.ends_with(".scn") {
                        crate::content::content_provider::OptimizedKind::Scene
                    } else {
                        crate::content::content_provider::OptimizedKind::Texture
                    }
                )
            ),
        })
        .collect();
    match write_bundle(&zip_path, &zip_entries, zip::CompressionMethod::Stored) {
        Ok(bytes) => {
            tracing::info!(
                "Scene batch {}: static bundle {} ({} GLBs from main.crdt, {} entries, {} bytes)",
                batch_id,
                zip_name,
                assets.gltfs.len(),
                entries.len(),
                bytes
            );
            job_manager
                .add_individual_zip(batch_id, output_hash.to_string(), zip_path)
                .await;
            Some(super::types::StaticBundleInfo {
                file: zip_name,
                files: entries.into_iter().map(|(name, _)| name).collect(),
                bytes,
            })
        }
        Err(e) => {
            tracing::error!("Scene batch {}: static bundle failed: {}", batch_id, e);
            None
        }
    }
}
