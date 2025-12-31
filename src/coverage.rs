use std::{
    collections::HashMap,
    fs::{self, create_dir_all},
    path::Path,
    process::{Child, Stdio},
};

use anyhow::Context;
use xtaskops::ops::{clean_files, cmd, confirm, remove_dir};

use crate::{consts::RUST_LIB_PROJECT_FOLDER, helpers, run, ui};

const SNAPSHOT_DIRS: &[&str] = &[
    "tests/snapshots/scenes",
    "tests/snapshots/avatar-image-generation",
    "tests/snapshots/client",
];

const SCENE_EXPLORER_TESTS_DIR: &str = "./tests/scene-explorer-tests";

/// Starts the scene-explorer-tests server for running scene tests.
/// Returns the server process handle that should be killed when tests complete.
fn start_scene_test_server() -> Result<Child, anyhow::Error> {
    ui::print_section("Setting up Scene Test Server");

    let scene_tests_path = Path::new(SCENE_EXPLORER_TESTS_DIR);
    if !scene_tests_path.exists() {
        anyhow::bail!(
            "Scene explorer tests directory not found at {}. Please ensure tests/scene-explorer-tests exists.",
            SCENE_EXPLORER_TESTS_DIR
        );
    }

    // Install npm dependencies
    ui::print_message(
        ui::MessageType::Step,
        "Installing scene-explorer-tests dependencies...",
    );
    let npm_install = std::process::Command::new("npm")
        .args(["install", "--legacy-peer-deps"])
        .current_dir(scene_tests_path)
        .status()
        .context("Failed to run npm install")?;

    if !npm_install.success() {
        anyhow::bail!("npm install failed for scene-explorer-tests");
    }

    // Build the scenes
    ui::print_message(ui::MessageType::Step, "Building scene-explorer-tests...");
    let npm_build = std::process::Command::new("npm")
        .args(["run", "build"])
        .current_dir(scene_tests_path)
        .status()
        .context("Failed to run npm build")?;

    if !npm_build.success() {
        anyhow::bail!("npm build failed for scene-explorer-tests");
    }

    // Export static files with local URL
    ui::print_message(
        ui::MessageType::Step,
        "Exporting static files for local testing...",
    );
    let npm_export = std::process::Command::new("npm")
        .args(["run", "export-static-local"])
        .current_dir(scene_tests_path)
        .status()
        .context("Failed to run npm export-static-local")?;

    if !npm_export.success() {
        anyhow::bail!("npm export-static-local failed for scene-explorer-tests");
    }

    // Start the http server in the background
    ui::print_message(
        ui::MessageType::Step,
        "Starting http-server on port 7666...",
    );
    let server_process = std::process::Command::new("npm")
        .args(["run", "serve"])
        .current_dir(scene_tests_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to start http-server")?;

    // Give the server a moment to start
    std::thread::sleep(std::time::Duration::from_secs(2));

    ui::print_message(
        ui::MessageType::Success,
        "Scene test server started on http://localhost:7666",
    );

    Ok(server_process)
}

/// Stops the scene test server by killing the process
fn stop_scene_test_server(mut server: Child) {
    ui::print_message(ui::MessageType::Step, "Stopping scene test server...");
    let _ = server.kill();
    let _ = server.wait();
    ui::print_message(ui::MessageType::Success, "Scene test server stopped");
}

#[derive(Debug)]
struct SnapshotDiff {
    category: String,
    test_name: String,
    is_new: bool,
}

/// Finds snapshot differences by looking for .diff.png files in comparison folders.
fn find_snapshot_diffs() -> Vec<SnapshotDiff> {
    let mut diffs = Vec::new();

    for snapshot_dir in SNAPSHOT_DIRS {
        let comparison_dir = Path::new(snapshot_dir).join("comparison");
        let original_dir = Path::new(snapshot_dir);

        if !comparison_dir.exists() {
            continue;
        }

        // Find all .diff.png files
        if let Ok(entries) = fs::read_dir(&comparison_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.ends_with(".diff.png") {
                        let test_name = name.trim_end_matches(".diff.png").to_string();
                        let original_file = original_dir.join(format!("{}.png", test_name));
                        let category = Path::new(snapshot_dir)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        diffs.push(SnapshotDiff {
                            category,
                            test_name,
                            is_new: !original_file.exists(),
                        });
                    }
                }
            }
        }
    }

    // Sort by category and test name
    diffs.sort_by(|a, b| (&a.category, &a.test_name).cmp(&(&b.category, &b.test_name)));
    diffs
}

/// Generates a markdown report of snapshot differences and saves it to a file.
/// Returns the number of differences found.
fn generate_snapshot_report() -> usize {
    let diffs = find_snapshot_diffs();
    let count = diffs.len();

    if diffs.is_empty() {
        ui::print_message(ui::MessageType::Success, "No snapshot differences found");
        // Remove report file if it exists (no differences)
        let _ = fs::remove_file("coverage/snapshot-report.md");
        return 0;
    }

    ui::print_message(
        ui::MessageType::Warning,
        &format!("Found {} snapshot(s) with differences", count),
    );

    let mut lines = vec![
        "## 📸 Snapshot Test Differences\n".to_string(),
        format!("Found **{}** snapshot(s) with differences.\n", count),
        String::new(),
        "| Category | Test Name | Status |".to_string(),
        "|----------|-----------|--------|".to_string(),
    ];

    for diff in &diffs {
        let status = if diff.is_new {
            "🆕 New"
        } else {
            "⚠️ Changed"
        };
        lines.push(format!(
            "| {} | `{}` | {} |",
            diff.category, diff.test_name, status
        ));
    }

    lines.push(String::new());
    lines.push("### How to review".to_string());
    lines.push("1. Download the `coverage-snapshots` artifact from this workflow run".to_string());
    lines.push("2. The comparison folder contains:".to_string());
    lines.push("   - `<name>.png` - Generated snapshot from this run".to_string());
    lines.push("   - `<name>.diff.png` - Visual difference highlighting changes".to_string());
    lines.push("3. Original snapshots are in the parent folder with the same name".to_string());
    lines.push(String::new());
    lines.push("### To update snapshots".to_string());
    lines.push(
        "If the changes are expected, copy the generated snapshots to replace the originals."
            .to_string(),
    );

    let report = lines.join("\n");

    // Ensure coverage directory exists
    let _ = create_dir_all("coverage");

    // Write report to file
    if let Err(e) = fs::write("coverage/snapshot-report.md", &report) {
        ui::print_message(
            ui::MessageType::Error,
            &format!("Failed to write snapshot report: {}", e),
        );
    } else {
        ui::print_message(
            ui::MessageType::Info,
            "Snapshot report saved to coverage/snapshot-report.md",
        );
    }

    count
}

/// Reads scene test coordinates from index.json in the scene-explorer-tests directory.
fn get_scene_test_coords() -> Result<Vec<[i32; 2]>, anyhow::Error> {
    let index_path = Path::new(SCENE_EXPLORER_TESTS_DIR).join("index.json");

    let content = std::fs::read_to_string(&index_path)
        .with_context(|| format!("Failed to read {:?}", index_path))?;

    let json: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {:?}", index_path))?;

    let scenes = json
        .get("scenes")
        .and_then(|s| s.as_array())
        .ok_or_else(|| anyhow::anyhow!("index.json missing 'scenes' array"))?;

    let mut coords: Vec<[i32; 2]> = Vec::new();

    for scene in scenes {
        if let Some(name) = scene.as_str() {
            // Parse folder name format: "X,Y-description" (e.g., "52,-52-raycast")
            // Find the position after coordinates by looking for a dash followed by a letter
            let coord_end = name
                .char_indices()
                .find(|(i, c)| {
                    *c == '-' && name.chars().nth(i + 1).map_or(false, |next| next.is_alphabetic())
                })
                .map(|(i, _)| i)
                .unwrap_or(name.len());

            let coord_part = &name[..coord_end];
            let parts: Vec<&str> = coord_part.split(',').collect();
            if parts.len() == 2 {
                if let (Ok(x), Ok(y)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                    coords.push([x, y]);
                }
            }
        }
    }

    if coords.is_empty() {
        anyhow::bail!("No valid scene coordinates found in {:?}", index_path);
    }

    Ok(coords)
}

pub fn coverage_with_itest(devmode: bool) -> Result<(), anyhow::Error> {
    let scene_snapshot_folder = Path::new("./tests/snapshots/scenes");
    let scene_snapshot_folder = scene_snapshot_folder.canonicalize()?;
    let client_snapshot_folder = Path::new("./tests/snapshots/client");
    let client_snapshot_folder = client_snapshot_folder.canonicalize()?;

    remove_dir("./coverage")?;
    create_dir_all("./coverage")?;

    ui::print_section("Running Coverage");
    // Run tests without livekit (use_deno only)
    let mut test_cmd = cmd!(
        "cargo",
        "test",
        "--no-default-features",
        "--features",
        "use_deno",
        "--",
        "--skip",
        "auth"
    )
    .env("CARGO_INCREMENTAL", "0")
    .env("RUSTFLAGS", "-Cinstrument-coverage")
    .env("LLVM_PROFILE_FILE", "cargo-test-%p-%m.profraw")
    .dir(RUST_LIB_PROJECT_FOLDER);

    // Set PROTOC environment variable to use locally installed protoc
    let protoc_path = helpers::BinPaths::protoc_bin();
    if protoc_path.exists() {
        if let Ok(canonical_path) = std::fs::canonicalize(&protoc_path) {
            test_cmd = test_cmd.env("PROTOC", canonical_path.to_string_lossy().to_string());
        }
    }

    test_cmd.run()?;

    let build_envs: HashMap<String, String> = [
        ("CARGO_INCREMENTAL", "0"),
        ("RUSTFLAGS", "-Cinstrument-coverage"),
        ("LLVM_PROFILE_FILE", "cargo-test-%p-%m.profraw"),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

    // Build without livekit
    let coverage_build_args = vec!["--no-default-features", "--features", "use_deno"];

    run::build(
        false,
        false,
        coverage_build_args.clone(),
        Some(build_envs.clone()),
        None,
    )?;

    run::run(false, true, vec![], false, false)?;

    // Start the scene test server
    let scene_test_server = start_scene_test_server()?;

    // Run scene tests (wrap in a closure to ensure server cleanup on any error)
    let scene_test_result = (|| -> Result<(), anyhow::Error> {
        let scene_test_realm: &str = "http://localhost:7666/scene-explorer-tests";
        let scene_test_coords = get_scene_test_coords()?;
        ui::print_message(
            ui::MessageType::Info,
            &format!("Loaded {} scene tests from index.json", scene_test_coords.len()),
        );
        let scene_test_coords_str = serde_json::ser::to_string(&scene_test_coords)
            .expect("failed to serialize scene_test_coords");

        let extra_args = [
            "--scene-test",
            scene_test_coords_str.as_str(),
            "--realm",
            scene_test_realm,
            "--snapshot-folder",
            scene_snapshot_folder.to_str().unwrap(),
        ]
        .iter()
        .map(|it| it.to_string())
        .collect();

        run::build(
            false,
            false,
            coverage_build_args.clone(),
            Some(build_envs.clone()),
            None,
        )?;

        run::run(false, false, extra_args, true, false)?;
        Ok(())
    })();

    // Stop the scene test server
    stop_scene_test_server(scene_test_server);

    // Propagate any error from scene tests
    scene_test_result?;

    ui::print_section("Running Client Tests");
    let client_extra_args = [
        "--snapshot-folder",
        client_snapshot_folder.to_str().unwrap(),
    ]
    .iter()
    .map(|it| it.to_string())
    .collect();

    run::build(
        false,
        false,
        coverage_build_args.clone(),
        Some(build_envs.clone()),
        None,
    )?;
    run::run(false, false, client_extra_args, false, true)?;

    // Generate snapshot difference report
    ui::print_section("Checking Snapshot Differences");
    generate_snapshot_report();

    let err = glob::glob("./godot/*.profraw")?
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            println!("moving {:?} to ./lib", entry);
            cmd!("mv", entry, "./lib").run()
        })
        .any(|res| res.is_err());

    if err {
        return Err(anyhow::anyhow!("failed to move profraw files"));
    }

    println!("ok.");

    println!("=== generating report ===");
    let (fmt, file) = if devmode {
        ("html", "coverage/html")
    } else {
        ("lcov", "coverage/tests.lcov")
    };
    cmd!(
        "grcov",
        ".",
        "--binary-path",
        "./lib/target/debug/deps",
        "-s",
        ".",
        "-t",
        fmt,
        "--branch",
        "--ignore-not-existing",
        "--ignore",
        "/*",
        "--ignore",
        "./*",
        "--ignore",
        "*/src/tests/*",
        "-o",
        file,
    )
    .run()?;
    println!("ok.");

    println!("=== cleaning up ===");
    clean_files("./**/*.profraw")?;
    println!("ok.");
    if devmode {
        if confirm("open report folder?") {
            cmd!("open", file).run()?;
        } else {
            println!("report location: {file}");
        }
    }

    println!("=== test build without default features ===");
    let mut no_default_cmd = cmd!("cargo", "build", "--no-default-features")
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTFLAGS", "-Cinstrument-coverage")
        .env("LLVM_PROFILE_FILE", "cargo-test-%p-%m.profraw")
        .dir(RUST_LIB_PROJECT_FOLDER);

    // Set PROTOC environment variable to use locally installed protoc
    let protoc_path = helpers::BinPaths::protoc_bin();
    if protoc_path.exists() {
        if let Ok(canonical_path) = std::fs::canonicalize(&protoc_path) {
            no_default_cmd =
                no_default_cmd.env("PROTOC", canonical_path.to_string_lossy().to_string());
        }
    }

    no_default_cmd.run()?;

    Ok(())
}
