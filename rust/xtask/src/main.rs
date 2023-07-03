use std::fs::create_dir_all;

use anyhow::Context;
use clap::{AppSettings, Arg, Command};
use xtaskops::ops::{clean_files, cmd, confirm, remove_dir};

mod install_dependency;

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
        .subcommand(Command::new("install"))
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
                        .help("run tests")
                        .takes_value(false),
                ),
        );
    let matches = cli.get_matches();

    let root = xtaskops::ops::root_dir();
    let res = match matches.subcommand() {
        Some(("install", _)) => match install_dependency::install() {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("install failed: {}", e)),
        },
        Some(("run", sm)) => {
            let program = format!(
                "./../.bin/godot/{}",
                install_dependency::get_godot_executable_path().unwrap()
            );
            let mut args = vec!["--path", "./../godot"];
            if sm.is_present("editor") {
                args.push("-e");
            }

            let release_mode = sm.is_present("release");
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
                xtaskops::ops::cmd!("cargo", "build", "--package", "decentraland-godot-lib")
                    .run()?;
            }

            match install_dependency::copy_library(!release_mode) {
                Ok(_) => Ok(()),
                Err(e) => Err(anyhow::anyhow!("copy the library failed: {}", e)),
            }?;

            if sm.is_present("itest") {
                args.push("--test");
                args.push("--headless");
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
        Some(("coverage", sm)) => coverage_with_itest(sm.is_present("dev")),
        Some(("vars", _)) => {
            println!("root: {root:?}");
            Ok(())
        }
        Some(("ci", _)) => xtaskops::tasks::ci(),
        Some(("docs", _)) => xtaskops::tasks::docs(),
        Some(("powerset", _)) => xtaskops::tasks::powerset(),
        Some(("bloat-deps", sm)) => xtaskops::tasks::bloat_deps(
            sm.get_one::<String>("package")
                .context("please provide a package with -p")?,
        ),
        Some(("bloat-time", sm)) => xtaskops::tasks::bloat_time(
            sm.get_one::<String>("package")
                .context("please provide a package with -p")?,
        ),
        _ => unreachable!("unreachable branch"),
    };
    res
    // xtaskops::tasks::main()
}

pub fn coverage_with_itest(devmode: bool) -> Result<(), anyhow::Error> {
    remove_dir("coverage")?;
    create_dir_all("coverage")?;

    println!("=== running coverage ===");
    cmd!("cargo", "test")
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTFLAGS", "-Cinstrument-coverage")
        .env("LLVM_PROFILE_FILE", "cargo-test-%p-%m.profraw")
        .run()?;

    cmd!("cargo", "xtask", "run", "--itest")
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTFLAGS", "-Cinstrument-coverage")
        .env("LLVM_PROFILE_FILE", "cargo-test-%p-%m.profraw")
        .run()?;

    let err = glob::glob("./../godot/*.profraw")?
        .filter_map(|entry| entry.ok())
        .map(|entry| cmd!("mv", entry, "./").run())
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
        "./target/debug/deps",
        "-s",
        ".",
        "-t",
        fmt,
        "--branch",
        "--ignore-not-existing",
        "--ignore",
        "../*",
        "--ignore",
        "/*",
        "--ignore",
        "xtask/*",
        "--ignore",
        "*/src/tests/*",
        "-o",
        file,
    )
    .run()?;
    println!("ok.");

    println!("=== cleaning up ===");
    clean_files("**/*.profraw")?;
    println!("ok.");
    if devmode {
        if confirm("open report folder?") {
            cmd!("open", file).run()?;
        } else {
            println!("report location: {file}");
        }
    }

    Ok(())
}
