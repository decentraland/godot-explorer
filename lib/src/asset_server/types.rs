//! Shared types for the asset optimization server.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tokio::time::Instant;

/// Type of asset to process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetType {
    /// Scene GLTF (creates colliders)
    Scene,
    /// Wearable GLTF (no colliders)
    Wearable,
    /// Emote GLTF (extracts animations)
    Emote,
    /// Texture (image processing)
    Texture,
}

impl AssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetType::Scene => "scene",
            AssetType::Wearable => "wearable",
            AssetType::Emote => "emote",
            AssetType::Texture => "texture",
        }
    }
}

/// Status of a processing job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// Job is queued, waiting to be processed
    Queued,
    /// Downloading the asset
    Downloading,
    /// Processing the asset (GLTF loading, texture conversion, etc.)
    Processing,
    /// Job completed successfully
    Completed,
    /// Job failed with an error
    Failed,
}

/// Status of a batch of assets being processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BatchStatus {
    /// Jobs are still being processed
    Processing,
    /// All jobs done, creating ZIP
    Packing,
    /// ZIP created successfully
    Completed,
    /// Error occurred
    Failed,
}

/// A batch of assets to process and pack together.
#[derive(Debug, Clone)]
pub struct Batch {
    /// Unique batch identifier
    pub id: String,
    /// Output hash for ZIP filename
    pub output_hash: String,
    /// Jobs in this batch
    pub job_ids: Vec<String>,
    /// Current status
    pub status: BatchStatus,
    /// Path to the final ZIP file (if completed)
    pub zip_path: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// When the batch was created
    pub created_at: Instant,
    /// Scene hash (for process-scene batches)
    pub scene_hash: Option<String>,
    /// Hashes of assets to preload into the main metadata ZIP
    pub preloaded_hashes: Option<HashSet<String>>,
    /// Individual ZIP files created for each asset
    pub individual_zips: Vec<IndividualZipInfo>,
}

impl Batch {
    pub fn new(id: String, output_hash: String, job_ids: Vec<String>) -> Self {
        Self {
            id,
            output_hash,
            job_ids,
            status: BatchStatus::Processing,
            zip_path: None,
            error: None,
            created_at: Instant::now(),
            scene_hash: None,
            preloaded_hashes: None,
            individual_zips: Vec::new(),
        }
    }

    pub fn new_scene_batch(
        id: String,
        output_hash: String,
        job_ids: Vec<String>,
        scene_hash: String,
        preloaded_hashes: Option<HashSet<String>>,
    ) -> Self {
        Self {
            id,
            output_hash,
            job_ids,
            status: BatchStatus::Processing,
            zip_path: None,
            error: None,
            created_at: Instant::now(),
            scene_hash: Some(scene_hash),
            preloaded_hashes,
            individual_zips: Vec::new(),
        }
    }
}

/// A single asset to process.
#[derive(Debug, Clone, Deserialize)]
pub struct AssetRequest {
    /// URL to fetch the asset from
    pub url: String,
    /// Type of asset to process
    #[serde(rename = "type")]
    pub asset_type: AssetType,
    /// Content hash of the asset
    pub hash: String,
    /// Base URL for content fetching (e.g., "https://content.decentraland.org/contents/")
    pub base_url: String,
    /// Content mapping for GLTF dependencies (file_path -> hash)
    #[serde(default)]
    pub content_mapping: HashMap<String, String>,
    /// If true, only use cached files - don't download anything.
    /// Useful when another process handles downloads.
    #[serde(default)]
    pub cache_only: bool,
}

/// Request body for POST /process endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct ProcessRequest {
    /// Output hash for the ZIP filename.
    /// Optional for single asset (uses asset's hash), required for multiple assets.
    #[serde(default)]
    pub output_hash: Option<String>,
    /// List of assets to process
    pub assets: Vec<AssetRequest>,
}

/// Original texture dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureSize {
    pub width: u32,
    pub height: u32,
}

/// A baked texture variant at a specific quality tier.
#[derive(Debug, Clone)]
pub struct TextureVariant {
    /// TextureQuality ordinal (0=Low, 1=Medium, 2=High, 3=Source)
    pub quality: i32,
    /// Path to the baked `.res` file
    pub path: String,
    /// File size in bytes
    pub file_size: u64,
}

/// A processing job.
#[derive(Debug, Clone)]
pub struct Job {
    /// Unique job identifier
    pub id: String,
    /// Asset hash being processed
    pub hash: String,
    /// Type of asset
    pub asset_type: AssetType,
    /// Current status
    pub status: JobStatus,
    /// Progress (0.0 to 1.0)
    pub progress: f32,
    /// Path to the optimized asset (if completed)
    pub optimized_path: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// When the job was created
    pub created_at: Instant,
    /// When the job was last updated
    pub updated_at: Instant,
    /// Original texture size (for texture jobs)
    pub original_size: Option<TextureSize>,
    /// Optimized file size in bytes
    pub optimized_file_size: Option<u64>,
    /// GLTF texture dependencies (for GLTF jobs)
    pub gltf_dependencies: Option<Vec<String>>,
    /// Baked quality variants (for texture jobs)
    pub texture_variants: Option<Vec<TextureVariant>>,
}

impl Job {
    pub fn new(id: String, hash: String, asset_type: AssetType) -> Self {
        let now = Instant::now();
        Self {
            id,
            hash,
            asset_type,
            status: JobStatus::Queued,
            progress: 0.0,
            optimized_path: None,
            error: None,
            created_at: now,
            updated_at: now,
            original_size: None,
            optimized_file_size: None,
            gltf_dependencies: None,
            texture_variants: None,
        }
    }
}

/// Response for a single job in POST /process.
#[derive(Debug, Serialize)]
pub struct JobResponse {
    pub job_id: String,
    pub hash: String,
    pub status: JobStatus,
}

/// Response for POST /process endpoint.
#[derive(Debug, Serialize)]
pub struct ProcessResponse {
    /// Batch ID for tracking all assets
    pub batch_id: String,
    /// Output hash for the ZIP filename
    pub output_hash: String,
    /// List of job responses (one per asset)
    pub jobs: Vec<JobResponse>,
    /// Total number of assets submitted
    pub total: usize,
}

/// Response for GET /status/{job_id} endpoint.
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub job_id: String,
    pub hash: String,
    pub asset_type: AssetType,
    pub status: JobStatus,
    pub progress: f32,
    /// Elapsed time since job creation in seconds
    pub elapsed_secs: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimized_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl From<&Job> for StatusResponse {
    fn from(job: &Job) -> Self {
        Self {
            job_id: job.id.clone(),
            hash: job.hash.clone(),
            asset_type: job.asset_type,
            status: job.status,
            progress: job.progress,
            elapsed_secs: job.created_at.elapsed().as_secs_f64(),
            optimized_path: job.optimized_path.clone(),
            error: job.error.clone(),
        }
    }
}

/// Response for GET /status/{batch_id} endpoint.
#[derive(Debug, Serialize)]
pub struct BatchStatusResponse {
    pub batch_id: String,
    pub output_hash: String,
    pub status: BatchStatus,
    pub progress: f32,
    pub jobs: Vec<StatusResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Individual asset ZIP files created so far
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub individual_zips: Vec<IndividualZipInfo>,
}

/// Response for GET /jobs endpoint.
#[derive(Debug, Serialize)]
pub struct JobsResponse {
    pub jobs: Vec<StatusResponse>,
    pub batches: Vec<BatchSummary>,
}

/// Summary of a batch for the jobs listing.
#[derive(Debug, Serialize)]
pub struct BatchSummary {
    pub batch_id: String,
    pub output_hash: String,
    pub status: BatchStatus,
    pub job_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip_path: Option<String>,
    pub elapsed_secs: f64,
}

/// Response for GET /health endpoint.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
}

/// Request body for POST /process-scene endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct ProcessSceneRequest {
    /// Scene entity hash to process
    pub scene_hash: String,
    /// Base URL for content fetching (e.g., "https://peer.decentraland.org/content/")
    pub content_base_url: String,
    /// Output hash for the ZIP filename (defaults to scene_hash)
    #[serde(default)]
    pub output_hash: Option<String>,
    /// Optional list of asset hashes to preload into the main metadata ZIP.
    /// If not provided, the main ZIP contains only metadata JSON.
    #[serde(default)]
    pub preloaded_hashes: Option<Vec<String>>,
    /// If true, only use cached files - don't download anything.
    /// Useful when another process handles downloads.
    #[serde(default)]
    pub cache_only: bool,
}

/// Response for POST /process-scene endpoint.
#[derive(Debug, Serialize)]
pub struct ProcessSceneResponse {
    /// Batch ID for tracking all assets
    pub batch_id: String,
    /// Output hash for the ZIP filename
    pub output_hash: String,
    /// Scene hash being processed
    pub scene_hash: String,
    /// Total number of assets discovered
    pub total_assets: usize,
    /// Number of assets that will be preloaded into the main ZIP
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preloaded_assets: Option<usize>,
    /// List of job responses (one per asset)
    pub jobs: Vec<JobResponse>,
}

/// Info about an individual asset ZIP file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndividualZipInfo {
    pub hash: String,
    pub zip_path: String,
}

/// Metadata stored in the ZIP file describing the optimization results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneOptimizationMetadata {
    /// List of all optimized content hashes
    pub optimized_content: Vec<String>,
    /// Map of GLTF hash -> list of texture hashes it depends on
    pub external_scene_dependencies: HashMap<String, Vec<String>>,
    /// Map of texture hash -> original dimensions
    pub original_sizes: HashMap<String, TextureSize>,
    /// Map of hash -> optimized file size in bytes
    /// (for textures: sum of all baked variant sizes)
    pub hash_size_map: HashMap<String, u64>,
    /// Map of texture hash -> baked quality ordinals (ascending).
    /// Ordinals follow TextureQuality: 0=Low, 1=Medium, 2=High, 3=Source.
    #[serde(default)]
    pub texture_qualities: HashMap<String, Vec<i32>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_optimization_metadata_serializes_camel_case() {
        let metadata = SceneOptimizationMetadata {
            optimized_content: vec!["hashA".to_string()],
            external_scene_dependencies: HashMap::new(),
            original_sizes: HashMap::new(),
            hash_size_map: HashMap::from([("hashA".to_string(), 1500)]),
            texture_qualities: HashMap::from([("hashA".to_string(), vec![0, 1, 3])]),
        };

        let json = serde_json::to_value(&metadata).expect("serializes");
        assert_eq!(
            json["textureQualities"]["hashA"],
            serde_json::json!([0, 1, 3])
        );
        assert_eq!(json["hashSizeMap"]["hashA"], 1500);

        let roundtrip: SceneOptimizationMetadata =
            serde_json::from_value(json).expect("deserializes");
        assert_eq!(roundtrip.texture_qualities["hashA"], vec![0, 1, 3]);
    }

    #[test]
    fn test_scene_optimization_metadata_texture_qualities_optional_on_read() {
        let json = r#"{
            "optimizedContent": [],
            "externalSceneDependencies": {},
            "originalSizes": {},
            "hashSizeMap": {}
        }"#;
        let metadata: SceneOptimizationMetadata =
            serde_json::from_str(json).expect("legacy metadata");
        assert!(metadata.texture_qualities.is_empty());
    }
}
