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
        print_message(MessageType::Success, "All GDScript files validated successfully!");
        Ok(())
    } else {
        print_message(
            MessageType::Error,
            "GDScript validation failed. See errors above.",
        );
        Err(anyhow::anyhow!("GDScript validation failed"))
    }
}
