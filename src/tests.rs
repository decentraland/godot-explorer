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
        std::fs::create_dir_all(&avatar_output)?;
    }

    let extra_args = [
        "--rendering-driver",
        "opengl3",
        "--rendering-method",
        "gl_compatibility",
        "--avatar-renderer",
        "--use-test-input",
    ]
    .iter()
    .map(|it| it.to_string())
    .collect();

    run::run(
        false,
        false,
        false,
        false,
        false,
        false,
        vec![],
        extra_args,
        with_build_envs.clone(),
    )?;

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
        std::fs::create_dir_all(&scene_output)?;
    }

    let extra_args = [
        "--rendering-driver",
        "opengl3",
        "--rendering-method",
        "gl_compatibility",
        "--scene-renderer",
        "--use-test-input",
    ]
    .iter()
    .map(|it| it.to_string())
    .collect();

    run::run(
        false,
        false,
        false,
        false,
        false,
        false,
        vec![],
        extra_args,
        with_build_envs,
    )?;

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

    test_avatar_generation(with_build_envs.clone())?;

    test_scene_generation(with_build_envs.clone())?;

    Ok(())
}
