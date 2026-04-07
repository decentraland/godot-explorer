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
pub fn test_godot_tools(
    with_build_envs: Option<HashMap<String, String>>,
) -> Result<(), anyhow::Error> {
    let avatar_result = test_avatar_generation(with_build_envs.clone());
    let scene_result = test_scene_generation(with_build_envs.clone());

    scene_result?;
    avatar_result?;

    Ok(())
}
