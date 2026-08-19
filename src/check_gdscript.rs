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

pub fn test_avatar() -> Result<()> {
    print_section("Avatar Regression Tests");

    let godot_bin = get_godot_path();
    for test in [
        "test_avatar_locomotion_grounded",
        "test_avatar_state_machine_graph",
        "test_avatar_autoplay_stomp",
        "test_avatar_anim_throttle",
    ] {
        print_message(MessageType::Info, &format!("Running {test}..."));
        let output = cmd!(
            godot_bin.clone(),
            "--headless",
            "--path",
            GODOT_PROJECT_FOLDER,
            "--script",
            &format!("res://src/test/avatar/{test}.gd")
        )
        .run()?;
        if !output.status.success() {
            print_message(MessageType::Error, &format!("{test} FAILED"));
            return Err(anyhow::anyhow!("avatar regression test failed: {test}"));
        }
    }
    print_message(MessageType::Success, "All avatar regression tests passed!");
    Ok(())
}
