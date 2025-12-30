use cargo_metadata::MetadataCommand;
use chrono::prelude::*;
use std::{env, path::PathBuf, process::Command};

fn get_lib_version() -> anyhow::Result<String> {
    // Get the workspace root (which is CARGO_MANIFEST_DIR for xtask) and look for lib/Cargo.toml
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("lib")
        .join("Cargo.toml");

    let metadata = MetadataCommand::new()
        .manifest_path(&manifest_path)
        .no_deps()
        .exec()?;

    let lib_package = metadata
        .packages
        .iter()
        .find(|p| p.name == "dclgodot")
        .ok_or_else(|| anyhow::anyhow!("Failed to find dclgodot package"))?;

    Ok(lib_package.version.to_string())
}

fn check_safe_repo() -> Result<(), String> {
    // Get the current working directory and navigate up two levels
    let mut repo_path = env::current_dir().map_err(|e| e.to_string())?;
    repo_path.pop(); // Go up one level
    repo_path.pop(); // Go up another level
    let repo_path_str = repo_path
        .to_str()
        .ok_or("Failed to convert repo path to string")?;

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8(output.stderr).map_err(|e| e.to_string())?;
    if stderr.contains("detected dubious ownership") {
        Command::new("git")
            .args([
                "config",
                "--global",
                "--add",
                "safe.directory",
                repo_path_str,
            ])
            .output()
            .map_err(|e| e.to_string())?;

        let output_retry = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .map_err(|e| e.to_string())?;
        if output_retry.status.success() {
            return Ok(());
        } else {
            let err_str = format!(
                "After retrying the git command, the error persisted: {}",
                String::from_utf8(output_retry.stderr)
                    .unwrap_or_else(|_| "Unknown error".to_string())
            );
            return Err(err_str);
        }
    }

    Err(stderr)
}

// This is duplicated of `lib/build.rs` for now
pub fn get_godot_explorer_version(verbose: bool) -> anyhow::Result<()> {
    // Always use git to get the actual checked-out commit (what GitHub checkout uses)
    let commit_hash = match check_safe_repo() {
        Ok(_) => {
            if let Ok(output) = Command::new("git")
                .args(["log", "-1", "--format=%H"])
                .output()
            {
                let long_hash = String::from_utf8(output.stdout).unwrap().trim().to_string();
                if verbose {
                    eprintln!(
                        "cargo:warning=Using commit hash: {} (from git log)",
                        long_hash.chars().take(7).collect::<String>()
                    );
                }
                Some(long_hash)
            } else {
                if verbose {
                    eprintln!(
                        "cargo:warning=After checking if the repo is safe, couldn't get the hash"
                    );
                }
                None
            }
        }
        Err(e) => {
            if verbose {
                eprintln!("cargo:warning=Check if the repo is safe: {}", e);
            }
            None
        }
    };

    // Get short hash (first 7 characters)
    let short_hash = commit_hash
        .as_ref()
        .map(|hash| hash.chars().take(7).collect::<String>());

    // Get the version from lib/Cargo.toml
    let version = get_lib_version().unwrap_or_else(|_| "0.0.0".to_string());

    // Check if this is a production build
    let is_prod_build = env::var("DECENTRALAND_PROD_BUILD").is_ok();

    // Check if debug or release build
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let is_debug = profile == "debug";

    // Determine environment suffix (dev or prod)
    let env_suffix = if is_prod_build { "prod" } else { "dev" };

    // Determine build mode suffix (debug for debug builds, none for release)
    let mode_suffix = if is_debug { "-debug" } else { "" };

    let full_version = match short_hash.clone() {
        // With git hash: {version}-{short_hash}-alpha{-debug}-{dev|prod}
        Some(hash) => format!("{}-{}-alpha{}-{}", version, hash, mode_suffix, env_suffix),
        // Fallback if no git hash available
        _ => {
            if verbose {
                eprintln!("cargo:warning=No commit hash available, using timestamp");
            }
            let timestamp = Utc::now()
                .to_rfc3339()
                .replace(|c: char| !c.is_ascii_digit(), "");
            format!(
                "{}-t{}-alpha{}-{}",
                version, timestamp, mode_suffix, env_suffix
            )
        }
    };

    println!("{}", full_version);

    Ok(())
}
