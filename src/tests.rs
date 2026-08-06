use std::{collections::HashMap, path::Path};

use anyhow::Ok;

use crate::{copy_files::move_dir_recursive, image_comparison::compare_images_folders, run};

/// Run Godot and tolerate a non-zero exit code (e.g. SIGABRT on shutdown) if output was produced.
fn run_godot_tolerating_shutdown_crash(
    extra_args: Vec<String>,
    output_dir: &Path,
    with_build_envs: Option<HashMap<String, String>>,
) -> Result<(), anyhow::Error> {
    // Clean output dir to avoid leftover files from previous runs
    if output_dir.exists() {
        std::fs::remove_dir_all(output_dir)?;
    }
    std::fs::create_dir_all(output_dir)?;

    run::build(false, false, vec![], with_build_envs, None)?;

    let run_result = run::run(false, false, extra_args, false, false, false);

    if let Err(e) = &run_result {
        let has_output = output_dir
            .read_dir()
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);

        if has_output {
            eprintln!(
                "Warning: Godot exited with error but output was generated, continuing: {}",
                e
            );
        } else {
            run_result?;
        }
    }

    Ok(())
}

pub fn test_avatar_generation(
    with_build_envs: Option<HashMap<String, String>>,
) -> Result<(), anyhow::Error> {
    let avatar_snapshot_folder =
        Path::new("./tests/snapshots/avatar-image-generation").canonicalize()?;
    let comparison_folder = avatar_snapshot_folder.join("comparison");

    println!("=== running godot avatar generation ===");

    let avatar_output = Path::new("./godot/output/");
    let avatar_test_input = Path::new("./../tests/avatars-test-input.json");
    let extra_args = [
        "--avatar-renderer",
        "--avatars",
        avatar_test_input.to_str().unwrap(),
    ]
    .iter()
    .map(|it| it.to_string())
    .collect();

    run_godot_tolerating_shutdown_crash(extra_args, avatar_output, with_build_envs)?;

    // Move files
    move_dir_recursive(&avatar_output.canonicalize()?, &comparison_folder)?;

    // Images comparison
    compare_images_folders(&avatar_snapshot_folder, &comparison_folder, 0.90)
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

pub fn test_scene_generation(
    with_build_envs: Option<HashMap<String, String>>,
) -> Result<(), anyhow::Error> {
    println!("=== running scene generation ===");
    let scene_output = Path::new("./godot/output/");
    let scene_test_input = Path::new("./../tests/scene-renderer-test-input.json");
    let extra_args = [
        "--scene-renderer",
        "--scene-input-file",
        scene_test_input.to_str().unwrap(),
    ]
    .iter()
    .map(|it| it.to_string())
    .collect();

    run_godot_tolerating_shutdown_crash(extra_args, scene_output, with_build_envs)?;

    let scene_renderer_snapshot_folder =
        Path::new("./tests/snapshots/scene-image-generation").canonicalize()?;
    let comparison_folder = scene_renderer_snapshot_folder.join("comparison");

    // Move files
    move_dir_recursive(&scene_output.canonicalize()?, &comparison_folder)?;

    // Images comparison
    compare_images_folders(&scene_renderer_snapshot_folder, &comparison_folder, 0.90)
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}
pub fn test_lighting_generation(
    with_build_envs: Option<HashMap<String, String>>,
) -> Result<(), anyhow::Error> {
    println!("=== running lighting snapshot generation ===");
    let lighting_output = Path::new("./godot/output/");
    let extra_args = ["--lighting-renderer"]
        .iter()
        .map(|it| it.to_string())
        .collect();

    run_godot_tolerating_shutdown_crash(extra_args, lighting_output, with_build_envs)?;

    let lighting_snapshot_folder = Path::new("./tests/snapshots/lighting").canonicalize()?;
    let comparison_folder = lighting_snapshot_folder.join("comparison");

    // Move files
    move_dir_recursive(&lighting_output.canonicalize()?, &comparison_folder)?;

    // Baselines must exist — otherwise there's nothing to regress against.
    let baseline_count = std::fs::read_dir(&lighting_snapshot_folder)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("png"))
        .count();
    if baseline_count == 0 {
        anyhow::bail!(
            "No lighting baselines in {}. Generate them once with \
             `cargo run -- test-tools`, then promote the results: \
             `mv tests/snapshots/lighting/comparison/*.png tests/snapshots/lighting/` \
             and commit them.",
            lighting_snapshot_folder.display()
        );
    }

    // Stricter than scenes/avatars (0.90): lighting regressions are exactly
    // the subtle color-grade shifts a loose threshold misses (issue #2516).
    compare_images_folders(&lighting_snapshot_folder, &comparison_folder, 0.95)
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

pub fn test_godot_tools(
    with_build_envs: Option<HashMap<String, String>>,
) -> Result<(), anyhow::Error> {
    let avatar_result = test_avatar_generation(with_build_envs.clone());
    let scene_result = test_scene_generation(with_build_envs.clone());
    let lighting_result = test_lighting_generation(with_build_envs.clone());

    scene_result?;
    avatar_result?;
    lighting_result?;

    Ok(())
}
