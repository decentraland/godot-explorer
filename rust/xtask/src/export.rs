use crate::install_dependency;

pub fn export() -> Result<(), anyhow::Error> {
    let program = format!(
        "./../.bin/godot/{}",
        install_dependency::get_godot_executable_path().unwrap()
    );

    let mut args = vec!["-e", "--path", "./../godot", "--headless", "--quit"];

    let status = std::process::Command::new(program.as_str())
        .args(&args)
        .status()
        .expect("Failed to run Godot");

    let mut args = vec!["-e", "--path", "./../godot", "--headless", "--quit"];

    if !status.success() {
        return Err(anyhow::anyhow!(
            "(pre-import) Godot exited with non-zero status: {}",
            status
        ));
    }
    
    let mut args = vec!["-e", "--path", "./../godot", "--headless", "--quit"];

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
