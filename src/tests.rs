use std::{collections::HashMap, path::Path};

use anyhow::Ok;

use crate::{copy_files::move_dir_recursive, image_comparison::compare_images_folders, run};

pub fn test_godot_tools(
    with_build_envs: Option<HashMap<String, String>>,
) -> Result<(), anyhow::Error> {
    let avatar_snapshot_folder =
        Path::new("./tests/snapshots/avatar-image-generation").canonicalize()?;
    let comparison_folder = avatar_snapshot_folder.join("comparison");

    println!("=== running godot avatar generation ===");

    let extra_args = [
        "--rendering-driver",
        "opengl3",
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
    let avatar_output = Path::new("./godot/output/").canonicalize()?;
    move_dir_recursive(&avatar_output, &comparison_folder)?;

    // Images comparison
    compare_images_folders(&avatar_snapshot_folder, &comparison_folder, 0.995)
        .map_err(|e| anyhow::anyhow!(e))?;

    println!("=== running scene  generation ===");

    let extra_args = [
        "--rendering-driver",
        "opengl3",
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
    let avatar_output = Path::new("./godot/output/").canonicalize()?;
    move_dir_recursive(&avatar_output, &comparison_folder)?;

    Ok(())
}
