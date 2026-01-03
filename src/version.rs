use std::{fs, path::PathBuf};

/// Reads the version from .build.version file created during lib build.
/// This is the single source of truth - version is computed in lib/build.rs
pub fn get_godot_explorer_version(verbose: bool) -> anyhow::Result<()> {
    let checkpoint_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".build.version");

    let version = fs::read_to_string(&checkpoint_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to read version from {:?}: {}\n\nRun `cargo run -- build` first to generate the version file.",
            checkpoint_path,
            e
        )
    })?;

    let version = version.trim();

    if verbose {
        eprintln!("Version from build checkpoint: {}", version);
    }

    println!("{}", version);

    Ok(())
}
