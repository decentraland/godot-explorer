//! Job queue management for the asset server.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio::time::Instant;
use uuid::Uuid;

use super::types::{
    AssetType, Batch, BatchStatus, Job, JobStatus, SceneOptimizationMetadata, TextureSize,
};

/// Maximum number of concurrent processing jobs.
const MAX_CONCURRENT_JOBS: usize = 4;

/// Manages the job queue for asset processing.
pub struct JobManager {
    /// Map of job_id -> Job
    jobs: Arc<RwLock<HashMap<String, Job>>>,
    /// Map of hash -> job_id (to detect duplicate requests)
    hash_to_job: Arc<RwLock<HashMap<String, String>>>,
    /// Map of batch_id -> Batch (for batch tracking)
    batches: Arc<RwLock<HashMap<String, Batch>>>,
    /// Semaphore to limit concurrent processing
    semaphore: Arc<Semaphore>,
}

impl JobManager {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            hash_to_job: Arc::new(RwLock::new(HashMap::new())),
            batches: Arc::new(RwLock::new(HashMap::new())),
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_JOBS)),
        }
    }

    /// Create a new job for an asset.
    /// Returns None if a job for this hash already exists (returns existing job_id).
    pub async fn create_job(&self, hash: String, asset_type: AssetType) -> Result<String, String> {
        // Check if we already have a job for this hash
        let hash_jobs = self.hash_to_job.read().await;
        if let Some(existing_job_id) = hash_jobs.get(&hash) {
            return Err(existing_job_id.clone());
        }
        drop(hash_jobs);

        // Create new job
        let job_id = Uuid::new_v4().to_string();
        let job = Job::new(job_id.clone(), hash.clone(), asset_type);

        // Insert into both maps
        let mut jobs = self.jobs.write().await;
        let mut hash_jobs = self.hash_to_job.write().await;

        jobs.insert(job_id.clone(), job);
        hash_jobs.insert(hash, job_id.clone());

        Ok(job_id)
    }

    /// Get a job by its ID.
    pub async fn get_job(&self, job_id: &str) -> Option<Job> {
        let jobs = self.jobs.read().await;
        jobs.get(job_id).cloned()
    }

    /// Get all jobs.
    pub async fn get_all_jobs(&self) -> Vec<Job> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }

    /// Update the status and progress of a job.
    pub async fn update_progress(&self, job_id: &str, status: JobStatus, progress: f32) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = status;
            job.progress = progress;
            job.updated_at = Instant::now();
        }
    }

    /// Mark a job as completed.
    pub async fn complete_job(&self, job_id: &str, optimized_path: String) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = JobStatus::Completed;
            job.progress = 1.0;
            job.optimized_path = Some(optimized_path);
            job.updated_at = Instant::now();
        }
    }

    /// Mark a job as failed.
    pub async fn fail_job(&self, job_id: &str, error: String) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = JobStatus::Failed;
            job.error = Some(error);
            job.updated_at = Instant::now();
        }
    }

    /// Set the original texture size for a texture job.
    pub async fn set_texture_original_size(&self, job_id: &str, width: u32, height: u32) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.original_size = Some(TextureSize { width, height });
            job.updated_at = Instant::now();
        }
    }

    /// Set the optimized file size for a job.
    pub async fn set_optimized_file_size(&self, job_id: &str, size: u64) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.optimized_file_size = Some(size);
            job.updated_at = Instant::now();
        }
    }

    /// Set the GLTF dependencies (texture hashes) for a GLTF job.
    pub async fn set_gltf_dependencies(&self, job_id: &str, dependencies: Vec<String>) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.gltf_dependencies = Some(dependencies);
            job.updated_at = Instant::now();
        }
    }

    /// Acquire a permit to process a job.
    /// This limits the number of concurrent jobs.
    pub async fn acquire_permit(&self) -> tokio::sync::OwnedSemaphorePermit {
        self.semaphore.clone().acquire_owned().await.unwrap()
    }

    // ==================== Batch Management ====================

    /// Create a new batch for a group of jobs.
    pub async fn create_batch(&self, output_hash: String, job_ids: Vec<String>) -> String {
        let batch_id = Uuid::new_v4().to_string();
        let batch = Batch::new(batch_id.clone(), output_hash, job_ids);

        let mut batches = self.batches.write().await;
        batches.insert(batch_id.clone(), batch);

        batch_id
    }

    /// Create a new scene batch with optional pack filter.
    pub async fn create_scene_batch(
        &self,
        output_hash: String,
        job_ids: Vec<String>,
        scene_hash: String,
        preloaded_hashes: Option<HashSet<String>>,
    ) -> String {
        let batch_id = Uuid::new_v4().to_string();
        let batch = Batch::new_scene_batch(
            batch_id.clone(),
            output_hash,
            job_ids,
            scene_hash,
            preloaded_hashes,
        );

        let mut batches = self.batches.write().await;
        batches.insert(batch_id.clone(), batch);

        batch_id
    }

    /// Get a batch by its ID.
    pub async fn get_batch(&self, batch_id: &str) -> Option<Batch> {
        let batches = self.batches.read().await;
        batches.get(batch_id).cloned()
    }

    /// Get all batches.
    pub async fn get_all_batches(&self) -> Vec<Batch> {
        let batches = self.batches.read().await;
        batches.values().cloned().collect()
    }

    /// Check if all jobs in a batch are complete (either Completed or Failed).
    pub async fn is_batch_complete(&self, batch_id: &str) -> bool {
        let batches = self.batches.read().await;
        let batch = match batches.get(batch_id) {
            Some(b) => b,
            None => return false,
        };

        // Don't consider batches that are already packing/completed/failed
        if batch.status != BatchStatus::Processing {
            return false;
        }

        let jobs = self.jobs.read().await;
        for job_id in &batch.job_ids {
            if let Some(job) = jobs.get(job_id) {
                if !matches!(job.status, JobStatus::Completed | JobStatus::Failed) {
                    return false;
                }
            }
        }
        true
    }

    /// Get all completed job results for a batch.
    /// Returns (hash, optimized_path, asset_type) for each completed job.
    pub async fn get_batch_results(
        &self,
        batch_id: &str,
    ) -> Vec<(String, String, super::types::AssetType)> {
        let batches = self.batches.read().await;
        let batch = match batches.get(batch_id) {
            Some(b) => b,
            None => {
                tracing::warn!("get_batch_results: batch {} not found", batch_id);
                return Vec::new();
            }
        };

        let jobs = self.jobs.read().await;
        let mut results = Vec::new();
        let mut completed_no_path = 0;
        let mut not_completed = 0;
        let mut not_found = 0;

        for job_id in &batch.job_ids {
            if let Some(job) = jobs.get(job_id) {
                if job.status == JobStatus::Completed {
                    if let Some(ref path) = job.optimized_path {
                        results.push((job.hash.clone(), path.clone(), job.asset_type));
                    } else {
                        completed_no_path += 1;
                        if completed_no_path <= 3 {
                            tracing::warn!(
                                "Job {} (hash={}) is Completed but has no optimized_path!",
                                job_id,
                                job.hash
                            );
                        }
                    }
                } else {
                    not_completed += 1;
                }
            } else {
                not_found += 1;
            }
        }

        if completed_no_path > 0 || not_found > 0 {
            tracing::warn!(
                "get_batch_results: {} jobs, {} with path, {} completed without path, {} not completed, {} not found",
                batch.job_ids.len(),
                results.len(),
                completed_no_path,
                not_completed,
                not_found
            );
        }

        results
    }

    /// Get the output hash for a batch.
    pub async fn get_batch_output_hash(&self, batch_id: &str) -> Option<String> {
        let batches = self.batches.read().await;
        batches.get(batch_id).map(|b| b.output_hash.clone())
    }

    /// Get the preloaded hashes for a batch.
    pub async fn get_batch_preloaded_hashes(&self, batch_id: &str) -> Option<HashSet<String>> {
        let batches = self.batches.read().await;
        batches
            .get(batch_id)
            .and_then(|b| b.preloaded_hashes.clone())
    }

    /// Add an individual ZIP info to a batch.
    pub async fn add_individual_zip(&self, batch_id: &str, hash: String, zip_path: String) {
        let mut batches = self.batches.write().await;
        if let Some(batch) = batches.get_mut(batch_id) {
            batch
                .individual_zips
                .push(super::types::IndividualZipInfo { hash, zip_path });
        }
    }

    /// Build scene optimization metadata from completed jobs in a batch.
    pub async fn build_scene_metadata(&self, batch_id: &str) -> SceneOptimizationMetadata {
        let batches = self.batches.read().await;
        let batch = match batches.get(batch_id) {
            Some(b) => b,
            None => {
                return SceneOptimizationMetadata {
                    optimized_content: Vec::new(),
                    external_scene_dependencies: HashMap::new(),
                    original_sizes: HashMap::new(),
                    hash_size_map: HashMap::new(),
                }
            }
        };

        let jobs = self.jobs.read().await;
        let mut optimized_content = Vec::new();
        let mut external_scene_dependencies = HashMap::new();
        let mut original_sizes = HashMap::new();
        let mut hash_size_map = HashMap::new();

        for job_id in &batch.job_ids {
            if let Some(job) = jobs.get(job_id) {
                if job.status != JobStatus::Completed {
                    continue;
                }

                optimized_content.push(job.hash.clone());

                // Add original size for textures
                if let Some(ref size) = job.original_size {
                    original_sizes.insert(job.hash.clone(), size.clone());
                }

                // Add optimized file size
                if let Some(size) = job.optimized_file_size {
                    hash_size_map.insert(job.hash.clone(), size);
                }

                // Add GLTF dependencies (include GLTFs with empty deps too)
                if let Some(ref deps) = job.gltf_dependencies {
                    external_scene_dependencies.insert(job.hash.clone(), deps.clone());
                }
            }
        }

        SceneOptimizationMetadata {
            optimized_content,
            external_scene_dependencies,
            original_sizes,
            hash_size_map,
        }
    }

    /// Update the status of a batch.
    pub async fn update_batch_status(&self, batch_id: &str, status: BatchStatus) {
        let mut batches = self.batches.write().await;
        if let Some(batch) = batches.get_mut(batch_id) {
            batch.status = status;
        }
    }

    /// Mark a batch as completed with the ZIP path.
    pub async fn complete_batch(&self, batch_id: &str, zip_path: String) {
        let mut batches = self.batches.write().await;
        if let Some(batch) = batches.get_mut(batch_id) {
            batch.status = BatchStatus::Completed;
            batch.zip_path = Some(zip_path);
        }
    }

    /// Mark a batch as failed with an error message.
    pub async fn fail_batch(&self, batch_id: &str, error: String) {
        let mut batches = self.batches.write().await;
        if let Some(batch) = batches.get_mut(batch_id) {
            batch.status = BatchStatus::Failed;
            batch.error = Some(error);
        }
    }

    /// Clean up old completed/failed jobs and batches (older than 1 hour).
    pub async fn cleanup_old_jobs(&self) {
        let one_hour = std::time::Duration::from_secs(3600);
        let mut jobs = self.jobs.write().await;
        let mut hash_jobs = self.hash_to_job.write().await;
        let mut batches = self.batches.write().await;

        // Clean up old jobs
        let old_job_ids: Vec<String> = jobs
            .iter()
            .filter(|(_, job)| {
                matches!(job.status, JobStatus::Completed | JobStatus::Failed)
                    && job.updated_at.elapsed() > one_hour
            })
            .map(|(id, _)| id.clone())
            .collect();

        for job_id in old_job_ids {
            if let Some(job) = jobs.remove(&job_id) {
                hash_jobs.remove(&job.hash);
            }
        }

        // Clean up old batches
        let old_batch_ids: Vec<String> = batches
            .iter()
            .filter(|(_, batch)| {
                matches!(batch.status, BatchStatus::Completed | BatchStatus::Failed)
                    && batch.created_at.elapsed() > one_hour
            })
            .map(|(id, _)| id.clone())
            .collect();

        for batch_id in old_batch_ids {
            batches.remove(&batch_id);
        }
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}
