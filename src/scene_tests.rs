use std::path::Path;

use anyhow::Context;
use serde_json::Value;

const SCENES_FILE: &str = "./tests/scene-tests/scenes.json";
const SNAPSHOT_FOLDER: &str = "./tests/snapshots/scenes";
const REALM_BASE_URL: &str = "http://localhost:7666";

/// Which scenes a run visits: `ecs` (no screenshots), `render` (screenshots) or `all` (both).
fn in_tier(scene_tier: &str, tier: &str) -> bool {
    scene_tier != "disabled" && (tier == "all" || scene_tier == tier)
}

/// The Godot arguments that point a scene-test run at the tier's parcels on the local realm
/// that `scripts/serve-scene-tests.sh` serves.
pub fn scene_test_args(tier: &str) -> anyhow::Result<Vec<String>> {
    let text =
        std::fs::read_to_string(SCENES_FILE).with_context(|| format!("reading {SCENES_FILE}"))?;
    let list: Value = serde_json::from_str(&text).context("parsing scenes.json")?;
    let realm = list["realm"].as_str().context("scenes.json has no realm")?;
    let parcels: Vec<&Value> = list["scenes"]
        .as_array()
        .context("scenes.json has no scenes")?
        .iter()
        .filter(|scene| in_tier(scene["tier"].as_str().unwrap_or("disabled"), tier))
        .map(|scene| &scene["parcel"])
        .collect();
    if parcels.is_empty() {
        anyhow::bail!("no scenes in tier '{tier}' (expected ecs, render or all)");
    }
    let snapshot_folder = Path::new(SNAPSHOT_FOLDER)
        .canonicalize()
        .with_context(|| format!("resolving {SNAPSHOT_FOLDER}"))?;
    Ok(vec![
        "--scene-test".to_string(),
        serde_json::to_string(&parcels)?,
        "--realm".to_string(),
        format!("{REALM_BASE_URL}/{realm}"),
        "--snapshot-folder".to_string(),
        snapshot_folder.to_string_lossy().to_string(),
    ])
}
