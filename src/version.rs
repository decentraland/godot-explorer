use std::{fs, path::PathBuf};

/// Reads the version from .build.version file created during lib build.
/// Returns the version string or an error if the file doesn't exist.
pub fn read_version() -> anyhow::Result<String> {
    let checkpoint_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".build.version");

    let version = fs::read_to_string(&checkpoint_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to read version from {:?}: {}\n\nRun `cargo run -- build` first to generate the version file.",
            checkpoint_path,
            e
        )
    })?;

    Ok(version.trim().to_string())
}

/// Reads the Sentry release string (`{version}+{build}`) from the
/// .build.sentry_release checkpoint written by lib/build.rs. This is exactly
/// what the runtime reports as `release` (minus the `org.decentraland.godotexplorer@`
/// prefix added in project_main_loop.gd), so CI can register the same release.
pub fn read_sentry_release() -> anyhow::Result<String> {
    let checkpoint_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".build.sentry_release");

    let release = fs::read_to_string(&checkpoint_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to read sentry release from {:?}: {}\n\nRun `cargo run -- build` first to generate the checkpoint file.",
            checkpoint_path,
            e
        )
    })?;

    Ok(release.trim().to_string())
}

/// Reads the version from .build.version file created during lib build.
/// This is the single source of truth - version is computed in lib/build.rs
pub fn get_godot_explorer_version(verbose: bool, sentry_release: bool) -> anyhow::Result<()> {
    let version = if sentry_release {
        read_sentry_release()?
    } else {
        read_version()?
    };

    if verbose {
        eprintln!("Version from build checkpoint: {}", version);
    }

    println!("{}", version);

    Ok(())
}
