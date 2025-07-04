use std::{collections::HashMap, fs::create_dir_all, path::Path};

use anyhow::Context;
use clap::{AppSettings, Arg, Command};
use export::import_assets;
use image_comparison::compare_images_folders;
use tests::test_godot_tools;
use xtaskops::ops::{clean_files, cmd, confirm, remove_dir};

use crate::consts::RUST_LIB_PROJECT_FOLDER;

mod consts;
mod copy_files;
mod doctor;
mod download_file;
mod export;
mod image_comparison;
mod install_dependency;
mod keystore;
mod path;
mod platform;
mod run;
mod tests;
mod ui;

fn main() -> Result<(), anyhow::Error> {
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
        .subcommand(
            Command::new("install")
                .arg(
                    Arg::new("no-templates")
                        .long("no-templates")
                        .help("skip download templates")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("platforms")
                        .long("platforms")
                        .help("download platform, can use multiple platforms, use like `--platforms linux android`")
                        .takes_value(true)
                        .multiple_values(true),
                )
        )
        .subcommand(Command::new("update-protocol"))
        .subcommand(
            Command::new("generate-keystore")
                .about("Generate Android keystore for app signing")
                .arg(
                    Arg::new("type")
                        .short('t')
                        .long("type")
                        .help("Keystore type: debug or release")
                        .takes_value(true)
                        .possible_values(&["debug", "release"])
                        .default_value("release"),
                ),
        )
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
                        .help("target OS (android, ios, linux, win64, macos)")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::new("format")
                        .short('f')
                        .long("format")
                        .help("Export format for Android: apk or aab")
                        .takes_value(true)
                        .possible_values(&["apk", "aab"])
                        .default_value("apk"),
                )
                .arg(
                    Arg::new("release")
                        .short('r')
                        .long("release")
                        .help("Export in release mode (signed)")
                        .takes_value(false),
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
                        .help("target OS")
                        .takes_value(true),
                ).arg(
                    Arg::new("platform")
                        .short('p')
                        .long("platform")
                        .help("additional platform to build (android/ios)")
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
                    Arg::new("resource-tracking")
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
                .values_of("platforms")
                .map(|vals| vals.map(String::from).collect())
                .unwrap_or_default();

            let no_templates = sm.is_present("no-templates") || platforms.is_empty();
            // Call your install function and pass the templates
            install_dependency::install(no_templates, &platforms)
        }
        ("update-protocol", _) => install_dependency::install_dcl_protocol(),
        ("generate-keystore", sm) => {
            let keystore_type = sm.value_of("type").unwrap_or("release");
            keystore::generate_keystore(keystore_type)
        }
        ("compare-image-folders", sm) => {
            let snapshot_folder = Path::new(sm.value_of("snapshots").unwrap());
            let result_folder = Path::new(sm.value_of("result").unwrap());
            compare_images_folders(snapshot_folder, result_folder, 0.995)
                .map_err(|e| anyhow::anyhow!(e))
        }
        ("run", sm) => {
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

            // Build for host OS
            run::build(
                sm.is_present("release"),
                build_args.clone(),
                None,
                sm.value_of("target"),
            )?;

            // Build for additional platform if specified
            if let Some(platform) = sm.value_of("platform") {
                print_message(
                    MessageType::Step,
                    &format!("Building for additional platform: {}", platform),
                );
                run::build(sm.is_present("release"), build_args, None, Some(platform))?;
            }

            run::run(
                sm.is_present("editor"),
                sm.is_present("itest"),
                sm.values_of("extras")
                    .map(|v| v.map(|it| it.into()).collect())
                    .unwrap_or_default(),
                sm.is_present("stest"),
            )?;
            Ok(())
        }
        ("build", sm) => {
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

            run::build(
                sm.is_present("release"),
                build_args,
                None,
                sm.value_of("target"),
            )?;
            Ok(())
        }
        ("export", sm) => {
            let target = sm.value_of("target");
            let format = sm.value_of("format").unwrap_or("apk");
            let release = sm.is_present("release");
            export::export(target, format, release)
        }
        ("import-assets", _m) => {
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

    remove_dir("./coverage")?;
    create_dir_all("./coverage")?;

    ui::print_section("Running Coverage");
    cmd!("cargo", "test", "--", "--skip", "auth")
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTFLAGS", "-Cinstrument-coverage")
        .env("LLVM_PROFILE_FILE", "cargo-test-%p-%m.profraw")
        .dir(RUST_LIB_PROJECT_FOLDER)
        .run()?;

    let build_envs: HashMap<String, String> = [
        ("CARGO_INCREMENTAL", "0"),
        ("RUSTFLAGS", "-Cinstrument-coverage"),
        ("LLVM_PROFILE_FILE", "cargo-test-%p-%m.profraw"),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

    run::build(false, vec![], Some(build_envs.clone()), None)?;

    run::run(false, true, vec![], false)?;

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

    run::build(false, vec![], Some(build_envs.clone()), None)?;

    run::run(false, false, extra_args, true)?;

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
    cmd!("cargo", "build", "--no-default-features")
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTFLAGS", "-Cinstrument-coverage")
        .env("LLVM_PROFILE_FILE", "cargo-test-%p-%m.profraw")
        .dir(RUST_LIB_PROJECT_FOLDER)
        .run()?;

    Ok(())
}
