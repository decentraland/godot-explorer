use anyhow::Result;
use xtaskops::ops::cmd;

use crate::{
    consts::GODOT_PROJECT_FOLDER,
    path::get_godot_path,
    ui::{print_message, print_section, MessageType},
};

pub fn check_gdscript() -> Result<()> {
    print_section("GDScript Validation");

    let godot_bin = get_godot_path();
    print_message(MessageType::Info, &format!("Using Godot: {}", godot_bin));
    print_message(
        MessageType::Info,
        "Running script validation on all .gd files...",
    );

    let output = cmd!(
        godot_bin,
        "--headless",
        "--path",
        GODOT_PROJECT_FOLDER,
        "res://src/test/validate_all_scripts.tscn",
        "--quit"
    )
    .run()?;

    if output.status.success() {
        print_message(
            MessageType::Success,
            "All GDScript files validated successfully!",
        );
        Ok(())
    } else {
        print_message(
            MessageType::Error,
            "GDScript validation failed. See errors above.",
        );
        Err(anyhow::anyhow!("GDScript validation failed"))
    }
}

/// Runs a set of headless GDScript unit tests, each a `SceneTree` script that
/// exits non-zero on failure. `scripts` are paths under `godot/`, given in full so
/// tests are not confined to one directory.
fn run_script_tests(section: &str, kind: &str, scripts: &[&str]) -> Result<()> {
    print_section(section);

    let godot_bin = get_godot_path();
    for script in scripts {
        let name = script.rsplit('/').next().unwrap_or(script);
        print_message(MessageType::Info, &format!("Running {name}..."));
        let output = cmd!(
            godot_bin.clone(),
            "--headless",
            "--path",
            GODOT_PROJECT_FOLDER,
            "--script",
            &format!("res://{script}")
        )
        .run()?;
        if !output.status.success() {
            print_message(MessageType::Error, &format!("{name} FAILED"));
            return Err(anyhow::anyhow!("{kind} test failed: {name}"));
        }
    }
    print_message(MessageType::Success, &format!("All {kind} tests passed!"));
    Ok(())
}

pub fn test_avatar() -> Result<()> {
    run_script_tests(
        "Avatar Regression Tests",
        "avatar regression",
        &[
            "src/test/avatar/test_avatar_locomotion_grounded.gd",
            "src/test/avatar/test_avatar_state_machine_graph.gd",
            "src/test/avatar/test_avatar_autoplay_stomp.gd",
            "src/test/avatar/test_avatar_anim_throttle.gd",
        ],
    )
}

pub fn test_i18n() -> Result<()> {
    run_script_tests(
        "Localization Tests",
        "localization",
        &["src/test/i18n/test_translation_key.gd"],
    )
}
