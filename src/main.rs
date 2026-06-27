use std::{collections::HashMap, fs::create_dir_all, path::Path};

use anyhow::Context;
use clap::{AppSettings, Arg, Command};
use export::import_assets;
use image_comparison::compare_images_folders;
use tests::test_godot_tools;
use xtaskops::ops::{clean_files, cmd, confirm, remove_dir};

use crate::{consts::RUST_LIB_PROJECT_FOLDER, install_dependency::clear_cache_dir};

mod android_godot_lib;
mod avatar_impostor_benchmark;
mod build_config;
mod check_gdscript;
mod consts;
mod copy_files;
mod dependencies;
mod doctor;
mod download_file;
mod export;
mod fi_benchmark;
mod full_tests;
mod helpers;
mod image_comparison;
mod install_dependency;
mod ios_xcode;
mod keystore;
mod log_server;
mod path;
mod platform;
mod run;
mod sentry_metrics;
mod tests;
mod ui;
mod update_snapshots;
mod version;
mod version_check;

fn ensure_project_root() -> Result<(), anyhow::Error> {
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;

    // Check for key markers that should exist in the project root
    let markers = vec![
        "Cargo.toml",
        "godot",
        "lib",
        "src", // xtask source
    ];

    let mut missing_markers = Vec::new();

    for marker in &markers {
        let marker_path = current_dir.join(marker);
        if !marker_path.exists() {
            missing_markers.push(*marker);
        }
    }

    if !missing_markers.is_empty() {
        use ui::{print_message, MessageType};
        print_message(
            MessageType::Error,
            &format!(
                "This command must be run from the project root directory.\n\
                Missing: {}\n\
                Current directory: {}\n\
                Please cd to the project root and try again.",
                missing_markers.join(", "),
                current_dir.display()
            ),
        );
        anyhow::bail!("Not in project root directory");
    }

    Ok(())
}

fn main() -> Result<(), anyhow::Error> {
    // Ensure we're running from the project root
    ensure_project_root()?;
    let cli = Command::new("xtask")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            Command::new("coverage").arg(
                Arg::new("dev")
                    .short('d')
                    .long("dev")
                    .help("generate an html report")
                    .takes_value(false),
            ),
        )
        .subcommand(Command::new("test-tools"))
        .subcommand(Command::new("vars"))
        .subcommand(Command::new("ci"))
        .subcommand(Command::new("powerset"))
        .subcommand(
            Command::new("bloat-deps").arg(
                Arg::new("package")
                    .short('p')
                    .long("package")
                    .help("package to build")
                    .required(true)
                    .takes_value(true),
            ),
        )
        .subcommand(
            Command::new("bloat-time").arg(
                Arg::new("package")
                    .short('p')
                    .long("package")
                    .help("package to build")
                    .required(true)
                    .takes_value(true),
            ),
        )
        .subcommand(Command::new("docs"))
        .subcommand(Command::new("doctor").about("Check system health and dependencies"))
        .subcommand(Command::new("check-gdscript").about("Validate all GDScript files for syntax errors"))
        .subcommand(Command::new("version-check").about("Check version consistency across files"))
        .subcommand(
            Command::new("fi-benchmark")
                .about("Run floating islands memory benchmark with multiple client sessions")
                .arg(
                    Arg::new("headless")
                        .long("headless")
                        .help("Run in headless mode (faster, no screenshots)")
                        .action(clap::ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("avatar-impostor-benchmark")
                .about(
                    "Run avatar impostor benchmark (100 avatars, OFF then ON phases) and write results to benchmark-results/. Use --target ios to launch on a connected iPhone (assumes the app has been deployed via `cargo run -- run --target ios`).",
                )
                .arg(
                    Arg::new("headless")
                        .long("headless")
                        .help("Run headless (host only). Skinning still uses GPU; FPS may differ from windowed mode.")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("target")
                        .long("target")
                        .help("Target platform: host (default) or ios.")
                        .takes_value(true),
                ),
        )
        .subcommand(
            Command::new("full-tests")
                .about("Run ALL test workflows locally in sequence with timing information")
                .arg(
                    Arg::new("continue-on-failure")
                        .long("continue-on-failure")
                        .help("Continue running steps even if one fails")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("skip-visual")
                        .long("skip-visual")
                        .help("Skip visual/GPU tests (scene, client, test-tools)")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("update-snapshots")
                        .long("update-snapshots")
                        .help("After visual tests, accept new output as baseline snapshots")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("report")
                        .long("report")
                        .help("Generate an HTML report with snapshot diffs and open in browser")
                        .takes_value(false),
                ),
        )
        .subcommand(
            Command::new("explorer-version")
                .about("Get Godot Explorer version (reads from .build.version created during build)")
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .help("show detailed messages")
                        .takes_value(false),
                )
        )
        .subcommand(
            Command::new("install")
                .arg(
                    Arg::new("no-templates")
                        .long("no-templates")
                        .help("skip download templates")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("targets")
                        .long("targets")
                        .help("download platform, can use multiple platforms, use like `--targets linux android`")
                        .takes_value(true)
                        .multiple_values(true),
                )
                .arg(
                    Arg::new("cache")
                        .long("cache")
                        .help("use cached downloads instead of re-downloading (default: always download fresh)")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("strip-ios")
                        .long("strip-ios")
                        .help("strip debug symbols from iOS templates to save disk space (default: keep debug symbols)")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("branch")
                        .long("branch")
                        .help("download Godot editor and templates from a branch build (e.g. `fix-ios-screen-orientation-swiftui-host`) instead of the stable release")
                        .takes_value(true),
                )
        )
        .subcommand(Command::new("clean-cache").about("Clean the cache to re-download external files."))
        .subcommand(Command::new("strip-ios-templates").about("Strip debug symbols from installed iOS templates (macOS only)"))
        .subcommand(
            Command::new("compare-image-folders")
                .arg(
                    Arg::new("snapshots")
                        .short('s')
                        .long("snapshots")
                        .help("snapshots")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::new("result")
                        .short('r')
                        .long("result")
                        .help("results image folder for comparison")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("export")
                .arg(
                    Arg::new("target")
                        .short('t')
                        .long("target")
                        .help("target OS (android, ios, linux, win64, macos). Defaults to host platform if not specified")
                        .takes_value(true)
                        .required(false),
                )
                .arg(
                    Arg::new("format")
                        .short('f')
                        .long("format")
                        .help("Export format for Android: apk or aab")
                        .takes_value(true)
                        .possible_values(["apk", "aab"])
                        .default_value("apk"),
                )
                .arg(
                    Arg::new("release")
                        .short('r')
                        .long("release")
                        .help("Export in release mode (signed)")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("build-number")
                        .long("build-number")
                        .help("Override the store build number (versionCode / CFBundleVersion). Defaults to the DCL_BUILD_NUMBER env var; if unset, the committed placeholder is kept.")
                        .takes_value(true)
                        .required(false),
                ),
        )
        .subcommand(Command::new("import-assets"))
        .subcommand(
            Command::new("run")
                .arg(
                    Arg::new("editor")
                        .short('e')
                        .long("editor")
                        .help("open godot editor mode")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("release")
                        .short('r')
                        .long("release")
                        .help("build release mode (but it doesn't use godot release build")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("prod")
                        .long("prod")
                        .help("mark as production build (affects version string)")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("staging")
                        .long("staging")
                        .help("mark as staging build (affects version string)")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("itest")
                        .long("itest")
                        .help("run integration-tests"),
                )
                .arg(
                    Arg::new("stest")
                        .long("stest")
                        .help("run scene-tests")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("ctest")
                        .long("ctest")
                        .help("run client tests")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("resource-tracking")
                        .short('x')
                        .help("enables resource tracking feature")
                        .takes_value(false),
                )
                .arg(Arg::new("build-args").help("extra build args for rust"))
                .arg(
                    Arg::new("extras")
                        .last(true)
                        .allow_hyphen_values(true)
                        .multiple(true),
                ).arg(
                    Arg::new("target")
                        .short('t')
                        .long("target")
                        .help("Target platform to build for. For android/ios: without -e deploys to device, with -e just builds")
                        .takes_value(true),
                ).arg(
                    Arg::new("only-lib")
                        .long("only-lib")
                        .help("For Android: push .so file directly to device instead of full APK deployment")
                        .takes_value(false),
                ).arg(
                    Arg::new("skip-export")
                        .long("skip-export")
                        .help("Skip build and export steps, deploy existing APK/IPA directly to device")
                        .takes_value(false),
                ).arg(
                    Arg::new("launch-only")
                        .long("launch-only")
                        .help("Just relaunch the app already installed on the device — no build, export, or install (android/ios). Still starts the hub + wires the scene-inspector")
                        .takes_value(false),
                ).arg(
                    Arg::new("no-default-features")
                        .long("no-default-features")
                        .help("Do not activate default features")
                        .takes_value(false),
                ).arg(
                    Arg::new("features")
                        .long("features")
                        .short('F')
                        .help("Space-separated list of features to activate")
                        .takes_value(true)
                        .multiple_values(true),
                ).arg(
                    Arg::new("use-tuned-glibc")
                        .long("use-tuned-glibc")
                        .help("Tune glibc malloc for better memory release (Linux only)")
                        .takes_value(false),
                ).arg(
                    Arg::new("asset-server")
                        .long("asset-server")
                        .help("Start the asset optimization server instead of the normal client")
                        .takes_value(false),
                ).arg(
                    Arg::new("asset-server-port")
                        .long("asset-server-port")
                        .help("Port for asset optimization server (default: 8080)")
                        .takes_value(true)
                        .default_value("8080"),
                ).arg(
                    Arg::new("deeplink")
                        .long("deeplink")
                        .help("Bake a deeplink into the build (e.g. 'decentraland://open?location=10,20&rust-log=debug'); applied at startup with no CLI args needed (mobile/TestFlight)")
                        .takes_value(true),
                ).arg(
                    Arg::new("no-hub")
                        .long("no-hub")
                        .help("For android/ios device deploys: don't auto-start the debug-hub or wire the scene-inspector. Falls back to plain device-log streaming (adb logcat / iOS --console)")
                        .takes_value(false),
                ).arg(
                    Arg::new("hub-viewer")
                        .long("hub-viewer")
                        .help("For android/ios device deploys: make the debug-hub the on-screen log view on any platform and turn OFF the device log stream (adb logcat / iOS --console) so it isn't duplicated. Note: Android Kotlin logs only reach logcat, so they won't show")
                        .takes_value(false),
                ),
        ).subcommand(
            Command::new("debug-hub")
                .about("Rendezvous broker: one connect-out device <-> many local consumers (unified scene-inspector channel: logs + tree queries + eval)")
                .arg(
                    Arg::new("device-port")
                        .long("device-port")
                        .help("Port the device dials out to (default: 9231)")
                        .takes_value(true)
                        .default_value("9231"),
                )
                .arg(
                    Arg::new("consumer-port")
                        .long("consumer-port")
                        .help("Loopback port consumers (websocat/MCP/AI) connect to (default: 9230)")
                        .takes_value(true)
                        .default_value("9230"),
                )
                .arg(
                    Arg::new("bind")
                        .long("bind")
                        .help("Device-facing bind address (default: 0.0.0.0)")
                        .takes_value(true)
                        .default_value("0.0.0.0"),
                ),
        ).subcommand(
            Command::new("get-metrics")
                .about("(TEMPORARY) Get Sentry metrics for godot-explorer and auth-mobile")
                .arg(
                    Arg::new("from")
                        .help("Start date (YYYY-MM-DD)")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("to")
                        .help("End date (YYYY-MM-DD, inclusive)")
                        .required(true)
                        .index(2),
                ),
        ).subcommand(
            Command::new("push-metrics")
                .about("(TEMPORARY) Push Sentry metrics to Slack")
                .arg(
                    Arg::new("from")
                        .help("Start date (YYYY-MM-DD)")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("to")
                        .help("End date (YYYY-MM-DD, inclusive)")
                        .required(true)
                        .index(2),
                ),
        ).subcommand(
            Command::new("update-ios-xcode")
                .about("Update iOS Xcode project with latest builds (macOS only)")
                .arg(
                    Arg::new("godot")
                        .long("godot")
                        .help("Update Godot engine library")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("plugin")
                        .long("plugin")
                        .help("Update dcl-godot-ios plugin")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("lib")
                        .long("lib")
                        .help("Update Rust library (libdclgodot)")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("pck")
                        .long("pck")
                        .help("Re-export and update the PCK file")
                        .takes_value(false),
                ),
        ).subcommand(
            Command::new("update-docker-snapshots")
                .about("Update Docker avatar/scene test snapshots from CI artifacts")
                .arg(
                    Arg::new("run-id")
                        .long("run-id")
                        .help("GitHub Actions run ID (defaults to latest on current branch)")
                        .takes_value(true),
                )
                .arg(
                    Arg::new("branch")
                        .long("branch")
                        .help("Branch to find runs on (defaults to current branch)")
                        .takes_value(true),
                ),
        ).subcommand(
            Command::new("update-coverage-snapshots")
                .about("Update coverage test snapshots from CI artifacts")
                .arg(
                    Arg::new("run-id")
                        .long("run-id")
                        .help("GitHub Actions run ID (defaults to latest on current branch)")
                        .takes_value(true),
                )
                .arg(
                    Arg::new("branch")
                        .long("branch")
                        .help("Branch to find runs on (defaults to current branch)")
                        .takes_value(true),
                ),
        ).subcommand(
            Command::new("update-libgodot-android")
                .about("Update Godot Android library (libgodot_android.so) in the AAR template")
                .arg(
                    Arg::new("release")
                        .short('r')
                        .long("release")
                        .help("Update release build instead of debug")
                        .takes_value(false),
                ),
        ).subcommand(
            Command::new("build")
                .arg(
                    Arg::new("release")
                        .short('r')
                        .long("release")
                        .help("build release mode (but it doesn't use godot release build")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("prod")
                        .long("prod")
                        .help("mark as production build (affects version string)")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("staging")
                        .long("staging")
                        .help("mark as staging build (affects version string)")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("resource-tracking")
                        .short('x')
                        .help("enables resource tracking feature")
                        .takes_value(false),
                )
                .arg(Arg::new("build-args").help("extra build args for rust"))
                .arg(
                    Arg::new("target")
                        .short('t')
                        .long("target")
                        .help("target OS")
                        .takes_value(true),
                ).arg(
                    Arg::new("no-default-features")
                        .long("no-default-features")
                        .help("Do not activate default features")
                        .takes_value(false),
                ).arg(
                    Arg::new("features")
                        .long("features")
                        .short('F')
                        .help("Space-separated list of features to activate")
                        .takes_value(true)
                        .multiple_values(true),
                ).arg(
                    Arg::new("deeplink")
                        .long("deeplink")
                        .help("Bake a deeplink into the build (applied at startup with no CLI args needed)")
                        .takes_value(true),
                ),
        );
    let matches = cli.get_matches();

    let subcommand = if let Some(value) = matches.subcommand() {
        value
    } else {
        unreachable!("unreachable branch")
    };

    use ui::{print_message, MessageType};

    let root = xtaskops::ops::root_dir();

    let res = match subcommand {
        ("install", sm) => {
            let platforms: Vec<String> = sm
                .values_of("targets")
                .map(|vals| vals.map(String::from).collect())
                .unwrap_or_default();

            let no_templates = sm.is_present("no-templates") || platforms.is_empty();
            let use_cache = sm.is_present("cache");
            let strip_ios = sm.is_present("strip-ios");
            let branch = sm.value_of("branch").map(String::from);
            // Call your install function and pass the templates
            let result = install_dependency::install(
                no_templates,
                &platforms,
                use_cache,
                strip_ios,
                branch.as_deref(),
            );
            if result.is_ok() {
                dependencies::suggest_next_steps("install", None);
            }
            result
        }
        ("clean-cache", _) => clear_cache_dir().map_err(|e| anyhow::anyhow!(e)),
        ("strip-ios-templates", _) => export::strip_ios_templates(),
        ("compare-image-folders", sm) => {
            let snapshot_folder = Path::new(sm.value_of("snapshots").unwrap());
            let result_folder = Path::new(sm.value_of("result").unwrap());
            compare_images_folders(snapshot_folder, result_folder, 0.995)
                .map_err(|e| anyhow::anyhow!(e))
        }
        ("debug-hub", sm) => {
            let device_port: u16 = sm
                .value_of("device-port")
                .and_then(|p| p.parse().ok())
                .unwrap_or(9231);
            let consumer_port: u16 = sm
                .value_of("consumer-port")
                .and_then(|p| p.parse().ok())
                .unwrap_or(9230);
            let bind = sm.value_of("bind").unwrap_or("0.0.0.0");
            log_server::run_hub_blocking(bind, device_port, consumer_port)
        }
        ("run", sm) => {
            // Check dependencies first
            dependencies::check_command_dependencies("run", None)?;

            // Set environment variable based on --prod or --staging flag
            if sm.is_present("prod") {
                std::env::set_var("DECENTRALAND_PROD_BUILD", "1");
            } else if sm.is_present("staging") {
                std::env::set_var("DECENTRALAND_STAGING_BUILD", "1");
            }

            let mut build_args: Vec<&str> = sm
                .values_of("build-args")
                .map(|v| v.collect())
                .unwrap_or_default();

            if sm.is_present("resource-tracking") {
                build_args.extend(&["-F", "use_resource_tracking"]);
            }

            // Add asset_server feature if running in asset-server mode
            if sm.is_present("asset-server") {
                build_args.extend(&["-F", "asset_server"]);
            }

            // Handle feature flags
            if sm.is_present("no-default-features") {
                build_args.push("--no-default-features");
            }

            if let Some(features) = sm.values_of("features") {
                for feature in features {
                    build_args.push("-F");
                    build_args.push(feature);
                }
            }

            // Check if target is specified
            let target = sm.value_of("target");
            let is_only_lib = sm.is_present("only-lib");
            let is_launch_only = sm.is_present("launch-only");
            // launch-only is a stronger skip-export: it also skips the install
            // (android) / xcodebuild+install (ios) and just relaunches the app
            // already on the device.
            let is_skip_export = sm.is_present("skip-export") || is_launch_only;

            // Bake --deeplink into the generated GDScript so it also applies on
            // builds that can't receive CLI args (tap-to-open, TestFlight).
            // Overwritten on every run; empty when nothing was passed.
            let baked_deeplink = build_config::resolve_deeplink(sm.value_of("deeplink"));
            build_config::write_build_config(&baked_deeplink)?;
            if !baked_deeplink.is_empty() {
                print_message(
                    MessageType::Info,
                    &format!("Baked deeplink into build: {baked_deeplink}"),
                );
            }

            // For android/ios targets, check if we should deploy to device
            let should_deploy = target.is_some()
                && (target == Some("android") || target == Some("ios"))
                && !sm.is_present("editor");

            // Both --prod and --staging require release profile
            let production_or_staging = sm.is_present("prod") || sm.is_present("staging");

            // The unified debug-hub (default on for device deploys). One command
            // does build + deploy + hub: the device dials out to it (iOS bakes
            // the address via the export plugin; Android is wired below + via
            // `adb reverse`). `--no-hub` opts out → plain device-log streaming.
            let hub_enabled = should_deploy && !sm.is_present("no-hub");
            // `--hub-viewer`: force the hub to be the on-screen log view on ANY
            // platform, and suppress the device log stream (--console / logcat) so
            // it isn't duplicated. The viewer then blocks after deploy (below).
            let hub_viewer = hub_enabled && sm.is_present("hub-viewer");
            const HUB_DEVICE_PORT: u16 = 9231;
            const HUB_CONSUMER_PORT: u16 = 9230;

            if should_deploy {
                let platform = target.unwrap();

                if hub_enabled {
                    // Reuses an existing hub if one is already up; non-fatal. The
                    // hub itself stays quiet — a viewer (a consumer) does the
                    // on-screen printing.
                    log_server::spawn_hub_background(
                        "0.0.0.0",
                        HUB_DEVICE_PORT,
                        HUB_CONSUMER_PORT,
                        log_server::HubOutput::Quiet,
                    );
                    // Default (no --hub-viewer): on iOS attach a background log
                    // viewer so this terminal shows the GDScript/Rust logs os_log
                    // hides from `--console`; Android's logcat already shows
                    // everything. Under --hub-viewer the viewer runs blocking AFTER
                    // deploy (with native shown, device stream off) — see below.
                    if platform == "ios" && !hub_viewer {
                        log_server::spawn_log_viewer(HUB_CONSUMER_PORT, false);
                    }
                    if hub_viewer && platform == "android" {
                        print_message(
                            MessageType::Warning,
                            "--hub-viewer: Android Kotlin logs (dcl-godot-android) won't show — they reach only logcat, which is now off.",
                        );
                    }
                } else if platform == "ios" {
                    // No hub: tell the iOS export plugin to bake NO connect-out
                    // arg (sentinel), so the app doesn't idle-retry a dead hub —
                    // pure old-style `--console` log streaming.
                    std::env::set_var("DCL_IOS_GODOT_CMDLINE", "none");
                }

                // Android dials the hub through `adb reverse` (set up in
                // deploy_and_run_android when it sees a loopback scene-inspector
                // arg). Inject it unless the user already passed one.
                let inject_scene_inspector = |extras: &mut Vec<String>| {
                    if hub_enabled
                        && platform == "android"
                        && !extras.iter().any(|a| a.starts_with("--scene-inspector"))
                    {
                        extras.push(format!(
                            "--scene-inspector=ws://127.0.0.1:{HUB_DEVICE_PORT}"
                        ));
                    }
                };

                if is_only_lib && platform == "android" {
                    // Hotreload mode: build and push .so file only
                    print_message(
                        MessageType::Step,
                        "Building for Android (only lib, push .so only)",
                    );

                    // Build for Android
                    run::build(
                        sm.is_present("release"),
                        production_or_staging,
                        build_args.clone(),
                        None,
                        Some(platform),
                    )?;

                    // Get extras to pass to the app
                    let extras: Vec<String> = sm
                        .values_of("extras")
                        .map(|v| v.map(|it| it.into()).collect())
                        .unwrap_or_default();

                    // Push the .so file to device
                    run::hotreload_android(extras)?;

                    return Ok(());
                } else if is_skip_export {
                    print_message(
                        MessageType::Step,
                        &format!(
                            "{} on {} (no build/export{})",
                            if is_launch_only {
                                "Launching installed app"
                            } else {
                                "Deploying existing build"
                            },
                            platform,
                            if is_launch_only { "/install" } else { "" }
                        ),
                    );

                    // Just deploy to device (launch-only skips the install too)
                    let mut device_extras: Vec<String> = sm
                        .values_of("extras")
                        .map(|v| v.map(|it| it.into()).collect())
                        .unwrap_or_default();
                    inject_scene_inspector(&mut device_extras);
                    run::deploy_and_run_on_device(
                        platform,
                        sm.is_present("release"),
                        device_extras,
                        !hub_viewer,
                        !is_launch_only,
                    )?;
                    if hub_viewer {
                        log_server::run_log_viewer_blocking(HUB_CONSUMER_PORT, true);
                    }

                    return Ok(());
                } else {
                    // Normal deployment: build, export, install, and run
                    print_message(
                        MessageType::Step,
                        &format!("Building and deploying to {}", platform),
                    );

                    // 1. Build for host OS first
                    run::build(
                        sm.is_present("release"),
                        production_or_staging,
                        build_args.clone(),
                        None,
                        None,
                    )?;

                    // 2. Build for the platform
                    run::build(
                        sm.is_present("release"),
                        production_or_staging,
                        build_args.clone(),
                        None,
                        Some(platform),
                    )?;

                    // 3. Export APK/IPA
                    let format = if platform == "android" { "apk" } else { "ipa" };
                    let result =
                        export::export(Some(platform), format, sm.is_present("release"), None);

                    if result.is_ok() {
                        // 4. Install and run on device
                        let mut device_extras: Vec<String> = sm
                            .values_of("extras")
                            .map(|v| v.map(|it| it.into()).collect())
                            .unwrap_or_default();
                        inject_scene_inspector(&mut device_extras);
                        run::deploy_and_run_on_device(
                            platform,
                            sm.is_present("release"),
                            device_extras,
                            !hub_viewer,
                            true,
                        )?;
                        if hub_viewer {
                            log_server::run_log_viewer_blocking(HUB_CONSUMER_PORT, true);
                        }
                    }

                    return result;
                }
            } else {
                // Normal build (either host OS or just build for target without deploying)
                run::build(
                    sm.is_present("release"),
                    production_or_staging,
                    build_args,
                    None,
                    target,
                )?;
            }

            // Now run
            let mut extras: Vec<String> = sm
                .values_of("extras")
                .map(|v| v.map(|it| it.into()).collect())
                .unwrap_or_default();

            // Add asset-server arguments if present
            if sm.is_present("asset-server") {
                extras.push("--asset-server".to_string());
                if let Some(port) = sm.value_of("asset-server-port") {
                    extras.push("--asset-server-port".to_string());
                    extras.push(port.to_string());
                }
            }

            run::run(
                sm.is_present("editor"),
                sm.is_present("itest"),
                extras,
                sm.is_present("stest"),
                sm.is_present("ctest"),
                sm.is_present("use-tuned-glibc"),
            )?;
            Ok(())
        }
        ("build", sm) => {
            let target = sm.value_of("target");

            // Bake --deeplink into the generated GDScript (overwritten each build).
            {
                let baked = build_config::resolve_deeplink(sm.value_of("deeplink"));
                build_config::write_build_config(&baked)?;
                if !baked.is_empty() {
                    print_message(
                        MessageType::Info,
                        &format!("Baked deeplink into build: {baked}"),
                    );
                }
            }

            // Run version check first
            version_check::run_version_check()?;

            // Set environment variable based on --prod or --staging flag
            if sm.is_present("prod") {
                std::env::set_var("DECENTRALAND_PROD_BUILD", "1");
            } else if sm.is_present("staging") {
                std::env::set_var("DECENTRALAND_STAGING_BUILD", "1");
            }

            // Check dependencies first
            dependencies::check_command_dependencies("build", target)?;

            let mut build_args: Vec<&str> = sm
                .values_of("build-args")
                .map(|v| v.collect())
                .unwrap_or_default();

            if sm.is_present("resource-tracking") {
                build_args.extend(&["-F", "use_resource_tracking"]);
            }

            // Handle feature flags
            if sm.is_present("no-default-features") {
                build_args.push("--no-default-features");
            }

            if let Some(features) = sm.values_of("features") {
                for feature in features {
                    build_args.push("-F");
                    build_args.push(feature);
                }
            }

            // Both --prod and --staging require release profile
            let production_or_staging = sm.is_present("prod") || sm.is_present("staging");
            let result = run::build(
                sm.is_present("release"),
                production_or_staging,
                build_args,
                None,
                target,
            );

            if result.is_ok() {
                dependencies::suggest_next_steps("build", target);
            }

            result
        }
        ("export", sm) => {
            let target = sm.value_of("target");
            let format = sm.value_of("format").unwrap_or("apk");
            let release = sm.is_present("release");
            let build_number = sm.value_of("build-number").map(|v| {
                v.trim()
                    .parse::<u64>()
                    .expect("--build-number must be a positive integer")
            });

            // Check dependencies first
            dependencies::check_command_dependencies("export", target)?;

            let result = export::export(target, format, release, build_number);

            if result.is_ok() {
                dependencies::suggest_next_steps("export", target);
            }

            result
        }
        ("import-assets", _m) => {
            // Check dependencies first
            dependencies::check_command_dependencies("import-assets", None)?;

            // Only build if library doesn't exist (to avoid overwriting .build.version from CI builds)
            let lib_path = helpers::get_host_library_path();
            if !lib_path.exists() {
                run::build(false, false, vec![], None, None)?;
            }

            let status = import_assets();
            if !status.success() {
                println!("WARN: cargo build exited with non-zero status: {}", status);
            }
            Ok(())
        }
        ("coverage", sm) => coverage_with_itest(sm.is_present("dev")),
        ("test-tools", _) => test_godot_tools(None),
        ("vars", _) => {
            println!("root: {root:?}");
            Ok(())
        }
        ("ci", _) => xtaskops::tasks::ci(),
        ("docs", _) => xtaskops::tasks::docs(),
        ("powerset", _) => xtaskops::tasks::powerset(),
        ("bloat-deps", sm) => xtaskops::tasks::bloat_deps(
            sm.get_one::<String>("package")
                .context("please provide a package with -p")?,
        ),
        ("bloat-time", sm) => xtaskops::tasks::bloat_time(
            sm.get_one::<String>("package")
                .context("please provide a package with -p")?,
        ),
        ("doctor", _) => doctor::run_doctor(),
        ("check-gdscript", _) => check_gdscript::check_gdscript(),
        ("update-ios-xcode", sm) => ios_xcode::update_ios_xcode(
            sm.is_present("godot"),
            sm.is_present("plugin"),
            sm.is_present("lib"),
            sm.is_present("pck"),
        ),
        ("update-libgodot-android", sm) => {
            android_godot_lib::update_libgodot_android(sm.is_present("release"))
        }
        ("update-docker-snapshots", sm) => {
            update_snapshots::update_docker_snapshots(sm.value_of("run-id"), sm.value_of("branch"))
        }
        ("update-coverage-snapshots", sm) => update_snapshots::update_coverage_snapshots(
            sm.value_of("run-id"),
            sm.value_of("branch"),
        ),
        ("version-check", _) => version_check::run_version_check(),
        ("explorer-version", sm) => version::get_godot_explorer_version(sm.is_present("verbose")),
        ("get-metrics", sm) => {
            let from = sm.value_of("from").unwrap();
            let to = sm.value_of("to").unwrap();
            sentry_metrics::get_metrics(from, to)
        }
        ("push-metrics", sm) => {
            let from = sm.value_of("from").unwrap();
            let to = sm.value_of("to").unwrap();
            sentry_metrics::push_metrics(from, to)
        }
        ("fi-benchmark", sm) => fi_benchmark::run_fi_benchmark(sm.get_flag("headless")),
        ("avatar-impostor-benchmark", sm) => {
            let target: &str = sm
                .get_one::<String>("target")
                .map(|s| s.as_str())
                .unwrap_or("");
            avatar_impostor_benchmark::run_avatar_impostor_benchmark(
                sm.get_flag("headless"),
                target,
            )
        }
        ("full-tests", sm) => full_tests::run_full_tests(
            sm.is_present("continue-on-failure"),
            sm.is_present("skip-visual"),
            sm.is_present("update-snapshots"),
            sm.is_present("report"),
        ),
        _ => unreachable!("unreachable branch"),
    };

    if let Err(e) = &res {
        print_message(MessageType::Error, &format!("Failed: {}", e));
    }
    res
    // xtaskops::tasks::main()
}

pub fn coverage_with_itest(devmode: bool) -> Result<(), anyhow::Error> {
    let scene_snapshot_folder = Path::new("./tests/snapshots/scenes");
    let scene_snapshot_folder = scene_snapshot_folder.canonicalize()?;
    let client_snapshot_folder = Path::new("./tests/snapshots/client");
    let client_snapshot_folder = client_snapshot_folder.canonicalize()?;

    remove_dir("./coverage")?;
    create_dir_all("./coverage")?;

    ui::print_section("Running Coverage");
    let mut test_cmd = cmd!("cargo", "test", "--", "--skip", "auth")
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

    run::build(false, false, vec![], Some(build_envs.clone()), None)?;

    run::run(false, true, vec![], false, false, false)?;

    let scene_test_realm: &str = "http://localhost:7666/scene-explorer-tests";
    let scene_test_coords: Vec<[i32; 2]> = vec![
        [52, -52], // raycast
        [52, -54], // transform
        [52, -56], // billboard
        [52, -58], // camera-mode
        [52, -60], // engine-info
        [52, -62], // gltf-container
        [52, -64], // visibility
        [52, -66], // mesh-renderer
        [52, -68], // avatar-attach
        [54, -52], // material
        [54, -54], // text-shape
        // TODO: video events not working well
        // [54, -56], // video-player
        [54, -58], // ui-background
        [54, -60], // ui-text
    ];
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

    run::build(false, false, vec![], Some(build_envs.clone()), None)?;

    run::run(false, false, extra_args, true, false, false)?;

    ui::print_section("Running Client Tests");
    let client_extra_args = [
        "--snapshot-folder",
        client_snapshot_folder.to_str().unwrap(),
    ]
    .iter()
    .map(|it| it.to_string())
    .collect();

    run::build(false, false, vec![], Some(build_envs.clone()), None)?;
    run::run(false, false, client_extra_args, false, true, false)?;

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
