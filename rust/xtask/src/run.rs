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
    let macos_universal = true;

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

    if std::env::consts::OS == "macos" && macos_universal {
        build_universal_macos(release_mode);
    } else {
        let build_args = if release_mode {
            vec!["build", "--release"]
        } else {
            vec!["build"]
        };

        let build_status = std::process::Command::new("cargo")
            .current_dir(std::fs::canonicalize(RUST_LIB_PROJECT_FOLDER).unwrap())
            .args(build_args)
            .status()
            .expect("Failed to run Godot");

        if !build_status.success() {
            return Err(anyhow::anyhow!(
                "cargo build exited with non-zero status: {}",
                build_status
            ));
        }
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

fn build_universal_macos(release_mode: bool) -> Result<(), anyhow::Error> {
    let target_library_file_name = "libdecentraland_godot_lib.dylib";
    let targets = vec!["x86_64-apple-darwin", "aarch64-apple-darwin"];

    let dest_folder = if release_mode {
        format!("{RUST_LIB_PROJECT_FOLDER}target/release/")
    } else {
        format!("{RUST_LIB_PROJECT_FOLDER}target/debug/")
    };

    std::fs::create_dir_all(dest_folder.clone())?;

    let mut lipo_args = vec!["-create".to_string()];
    for target in targets.iter() {
        let build_args = if release_mode {
            vec!["build", "--release", "--target", target]
        } else {
            vec!["build", "--target", target]
        };

        let build_status = std::process::Command::new("cargo")
            .current_dir(std::fs::canonicalize(RUST_LIB_PROJECT_FOLDER).unwrap())
            .args(build_args)
            .status()
            .expect("Failed to run Godot");

        if !build_status.success() {
            return Err(anyhow::anyhow!(
                "cargo build exited with non-zero status: {}",
                build_status
            ));
        }

        lipo_args.push(format!(
            "{RUST_LIB_PROJECT_FOLDER}target/{target}/{target_library_file_name}",
            target = target
        ));
    }

    lipo_args.push("-output".into());
    lipo_args.push(format!("{dest_folder}{target_library_file_name}"));
    let lipo_status = std::process::Command::new("lipo")
        .args(lipo_args)
        .status()
        .expect("Failed to run lipo command");
    if !lipo_status.success() {
        return Err(anyhow::anyhow!(
            "lipo exited with non-zero status: {}",
            lipo_status
        ));
    }

    Ok(())
}
