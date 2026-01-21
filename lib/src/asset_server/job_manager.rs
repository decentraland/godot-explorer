//! Job queue management for the asset server.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio::time::Instant;
use uuid::Uuid;

use super::types::{AssetType, Batch, BatchStatus, Job, JobStatus};

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
            None => return Vec::new(),
        };

        let jobs = self.jobs.read().await;
        let mut results = Vec::new();

        for job_id in &batch.job_ids {
            if let Some(job) = jobs.get(job_id) {
                if job.status == JobStatus::Completed {
                    if let Some(ref path) = job.optimized_path {
                        results.push((job.hash.clone(), path.clone(), job.asset_type));
                    }
                }
            }
        }

        results
    }

    /// Get the output hash for a batch.
    pub async fn get_batch_output_hash(&self, batch_id: &str) -> Option<String> {
        let batches = self.batches.read().await;
        batches.get(batch_id).map(|b| b.output_hash.clone())
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
