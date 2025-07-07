use crate::{helpers::BinPaths, install_dependency};

#[cfg(not(target_os = "windows"))]
pub fn adjust_canonicalization<P: AsRef<std::path::Path>>(p: P) -> String {
    p.as_ref().display().to_string()
}

#[cfg(target_os = "windows")]
pub fn adjust_canonicalization<P: AsRef<std::path::Path>>(p: P) -> String {
    const VERBATIM_PREFIX: &str = r#"\\?\"#;
    const VERBATIM_UNC_PREFIX: &str = r#"\\?\UNC\"#;
    let p = p.as_ref().display().to_string();

    if let Some(stripped) = p.strip_prefix(VERBATIM_UNC_PREFIX) {
        // Convert \\?\UNC\server\share to \\server\share
        format!(r#"\\{}"#, stripped)
    } else if let Some(stripped) = p.strip_prefix(VERBATIM_PREFIX) {
        stripped.to_string()
    } else {
        p
    }
}

pub fn get_godot_path() -> String {
    adjust_canonicalization(
        std::fs::canonicalize(
            BinPaths::godot().join(install_dependency::get_godot_executable_path().unwrap()),
        )
        .expect("Did you execute `cargo run -- install`?"),
    )
}
