use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};

use crate::godot_classes::JsonGodotClass;

pub mod content_entity;
pub mod string;
pub mod wearable;

pub struct SceneJsFileContent(pub String);
pub struct SceneMainCrdtFileContent(pub Vec<u8>);

pub struct SceneStartTime(pub std::time::SystemTime);
pub struct SceneLogs(pub Vec<SceneLogMessage>);
pub struct SceneMainCrdt(pub Option<Vec<u8>>);
pub struct SceneTickCounter(pub u32);
pub struct SceneDying(pub bool);

pub struct SceneElapsedTime(pub f32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SceneLogLevel {
    Log = 1,
    SceneError = 2,
    SystemError = 3,
}

#[derive(Clone, Debug)]
pub struct SceneLogMessage {
    pub timestamp: f64, // scene local time
    pub level: SceneLogLevel,
    pub message: String,
}
static SCENE_LOG_ENABLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Deserialize, Serialize)]
pub struct GreyPixelDiffRequest {}

#[derive(Debug, Deserialize, Serialize)]
pub struct TestingScreenshotComparisonMethodRequest {
    grey_pixel_diff: Option<GreyPixelDiffRequest>,
}

impl JsonGodotClass for TestingScreenshotComparisonMethodRequest {}

#[derive(Debug, Deserialize, Serialize)]
pub struct GreyPixelDiffResult {
    pub similarity: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeAndCompareSnapshotResponse {
    pub stored_snapshot_found: bool,
    pub grey_pixel_diff: Option<GreyPixelDiffResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneTestResult {
    pub name: String,
    pub ok: bool,
    pub error: Option<String>,
    pub stack: Option<String>,
    pub total_frames: i32,
    pub total_time: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneTestPlan {
    pub tests: Vec<SceneTestPlanTestPlanEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneTestPlanTestPlanEntry {
    pub name: String,
}

pub fn set_scene_log_enabled(enabled: bool) {
    SCENE_LOG_ENABLED.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

pub fn is_scene_log_enabled() -> bool {
    SCENE_LOG_ENABLED.load(std::sync::atomic::Ordering::Relaxed)
}
