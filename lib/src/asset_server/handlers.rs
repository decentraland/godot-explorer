//! HTTP request handlers for the asset server.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use super::job_manager::JobManager;
use super::packer::{pack_assets_to_zip, pack_scene_assets_to_zip};
use super::processor::{process_asset, ProcessorContext};
use super::scene_fetcher::fetch_scene_entity;
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

    tracing::info!("Created batch {} for {} jobs", batch_id, jobs.len());

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
            tracing::info!(
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

    tracing::info!(
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

    // Build pack filter from pack_hashes
    let pack_filter = request
        .pack_hashes
        .map(|hashes| hashes.into_iter().collect::<HashSet<String>>());
    let pack_assets = pack_filter.as_ref().map(|f| f.len());

    tracing::info!(
        "Discovered {} assets in scene {}, pack_filter: {:?}",
        total_assets,
        request.scene_hash,
        pack_assets
    );

    let mut jobs = Vec::with_capacity(total_assets);
    let mut job_ids = Vec::with_capacity(total_assets);

    // Create jobs for all GLTF assets
    for asset in &scene_assets.gltfs {
        let asset_request = AssetRequest {
            url: asset.url.clone(),
            asset_type: AssetType::Scene,
            hash: asset.hash.clone(),
            base_url: scene_assets.content_base_url.clone(),
            content_mapping: scene_assets.content_mapping.clone(),
        };

        match process_single_scene_asset(asset_request, job_manager.clone(), ctx.clone()).await {
            Ok(response) => {
                if !response.job_id.is_empty() {
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
        };

        match process_single_scene_asset(asset_request, job_manager.clone(), ctx.clone()).await {
            Ok(response) => {
                if !response.job_id.is_empty() {
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

    // Create scene batch with pack filter
    let batch_id = job_manager
        .create_scene_batch(
            output_hash.clone(),
            job_ids,
            request.scene_hash.clone(),
            pack_filter,
        )
        .await;

    tracing::info!("Created scene batch {} for {} jobs", batch_id, jobs.len());

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
        pack_assets,
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
            tracing::info!(
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

    tracing::info!(
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

/// Watch for scene batch completion and pack results into a ZIP file with metadata.
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
        "Scene batch {} complete, building metadata and packing",
        batch_id
    );

    // All jobs done - pack into ZIP with metadata
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

    // Get pack filter
    let pack_filter = job_manager.get_batch_pack_filter(&batch_id).await;

    // Build metadata from completed jobs
    let metadata = job_manager.build_scene_metadata(&batch_id).await;

    tracing::info!(
        "Scene batch {} metadata: {} optimized, {} dependencies, {} sizes",
        batch_id,
        metadata.optimized_content.len(),
        metadata.external_scene_dependencies.len(),
        metadata.original_sizes.len()
    );

    // Acquire Godot thread for ZIPPacker
    let _permit = ctx.godot_single_thread.acquire().await;

    match pack_scene_assets_to_zip(
        &output_hash,
        results,
        pack_filter.as_ref(),
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
