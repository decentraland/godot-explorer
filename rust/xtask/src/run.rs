use crate::{
    consts::{BIN_FOLDER, GODOT_PROJECT_FOLDER, RUST_LIB_PROJECT_FOLDER},
    install_dependency,
};

pub fn run(
    editor: bool,
    release_mode: bool,
    itest: bool,
    only_build: bool,
) -> Result<(), anyhow::Error> {
    let program = std::fs::canonicalize(format!(
        "{}godot/{}",
        BIN_FOLDER,
        install_dependency::get_godot_executable_path().unwrap()
    ))
    .unwrap()
    .to_str()
    .unwrap()
    .to_string();

    std::env::set_var("GODOT4_BIN", program.clone());

    let mut args = vec!["--path", GODOT_PROJECT_FOLDER];
    if editor {
        args.push("-e");
    }

    let build_args = if release_mode {
        vec!["build", "--release"]
    } else {
        vec!["build"]
    };
    
    let build_status = std::process::Command::new("cargo")
        .current_dir(RUST_LIB_PROJECT_FOLDER)
        .args(build_args)
        .status()
        .expect("Failed to run Godot");

    if !build_status.success() {
        return Err(anyhow::anyhow!(
            "cargo build exited with non-zero status: {}",
            build_status
        ));
    }

    match install_dependency::copy_library(!release_mode) {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!("copy the library failed: {}", e)),
    }?;

    if itest {
        args.push("--test");
        args.push("--headless");
    }

    if only_build {
        return Ok(());
    }

    let status = std::process::Command::new(program.as_str())
        .args(&args)
        .status()
        .expect("Failed to run Godot");

    if !status.success() {
        Err(anyhow::anyhow!(
            "Godot exited with non-zero status: {}",
            status
        ))
    } else {
        Ok(())
    }
}
