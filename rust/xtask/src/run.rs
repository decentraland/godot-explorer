use crate::install_dependency;

pub fn run(
    editor: bool,
    release_mode: bool,
    itest: bool,
    only_build: bool,
) -> Result<(), anyhow::Error> {
    let program = std::fs::canonicalize(format!(
        "./../.bin/godot/{}",
        install_dependency::get_godot_executable_path().unwrap()
    ))
    .unwrap()
    .to_str()
    .unwrap()
    .to_string();

    std::env::set_var("GODOT4_BIN", program.clone());

    let mut args = vec!["--path", "./../godot"];
    if editor {
        args.push("-e");
    }

    if release_mode {
        xtaskops::ops::cmd!(
            "cargo",
            "build",
            "--package",
            "decentraland-godot-lib",
            "--release"
        )
        .run()?;
    } else {
        xtaskops::ops::cmd!("cargo", "build", "--package", "decentraland-godot-lib").run()?;
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
