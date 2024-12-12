use std::{collections::HashMap, path::Path};

use anyhow::Ok;

use crate::{copy_files::move_dir_recursive, image_comparison::compare_images_folders, run};

fn test_avatar_generation(
    with_build_envs: Option<HashMap<String, String>>,
) -> Result<(), anyhow::Error> {
    let avatar_snapshot_folder =
        Path::new("./tests/snapshots/avatar-image-generation").canonicalize()?;
    let comparison_folder = avatar_snapshot_folder.join("comparison");

    println!("=== running godot avatar generation ===");

    let avatar_output = Path::new("./godot/output/");
    if !avatar_output.exists() {
        std::fs::create_dir_all(avatar_output)?;
    }

    let avatar_test_input = Path::new("./../tests/avatars-test-input.json");
    let extra_args = [
        "--rendering-driver",
        "opengl3",
        "--rendering-method",
        "gl_compatibility",
        "--avatar-renderer",
        "--avatars",
        avatar_test_input.to_str().unwrap(),
    ]
    .iter()
    .map(|it| it.to_string())
    .collect();

    run::build(false, false, vec![], with_build_envs, None)?;

    run::run(false, false, extra_args, false)?;

    // Move files
    move_dir_recursive(&avatar_output.canonicalize()?, &comparison_folder)?;

    // Images comparison
    compare_images_folders(&avatar_snapshot_folder, &comparison_folder, 0.90)
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

fn test_scene_generation(
    with_build_envs: Option<HashMap<String, String>>,
) -> Result<(), anyhow::Error> {
    println!("=== running scene generation ===");
    let scene_output = Path::new("./godot/output/");
    if !scene_output.exists() {
        std::fs::create_dir_all(scene_output)?;
    }
    let scene_test_input = Path::new("./../tests/scene-renderer-test-input.json");
    let extra_args = [
        "--rendering-driver",
        "opengl3",
        "--rendering-method",
        "gl_compatibility",
        "--scene-renderer",
        "--scene-input-file",
        scene_test_input.to_str().unwrap(),
    ]
    .iter()
    .map(|it| it.to_string())
    .collect();

    run::build(false, false, vec![], with_build_envs, None)?;

    run::run(false, false, extra_args, false)?;

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
