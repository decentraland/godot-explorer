use crate::{consts::BIN_FOLDER, install_dependency};

#[cfg(not(target_os = "windows"))]
pub fn adjust_canonicalization<P: AsRef<std::path::Path>>(p: P) -> String {
    p.as_ref().display().to_string()
}

#[cfg(target_os = "windows")]
pub fn adjust_canonicalization<P: AsRef<std::path::Path>>(p: P) -> String {
    const VERBATIM_PREFIX: &str = r#"\\?\"#;
    let p = p.as_ref().display().to_string();
    if let Some(stripped) = p.strip_prefix(VERBATIM_PREFIX) {
        stripped.to_string()
    } else {
        p
    }
}

pub fn get_godot_path() -> String {
    adjust_canonicalization(
        std::fs::canonicalize(format!(
            "{}godot/{}",
            BIN_FOLDER,
            install_dependency::get_godot_executable_path().unwrap()
        ))
        .expect("Did you executed `cargo run -- install`?"),
    )
}
