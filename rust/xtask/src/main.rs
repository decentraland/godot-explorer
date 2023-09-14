use std::fs::create_dir_all;

use anyhow::Context;
use clap::{AppSettings, Arg, Command};
use xtaskops::ops::{clean_files, cmd, confirm, remove_dir};

use crate::consts::RUST_LIB_PROJECT_FOLDER;

mod consts;
mod download_file;
mod export;
mod install_dependency;
mod path;
mod run;

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
        .subcommand(
            Command::new("install").arg(
                Arg::new("no-templates")
                    .long("no-templates")
                    .help("skip download templates")
                    .takes_value(false),
            ),
        )
        .subcommand(Command::new("export"))
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
                )
                .arg(
                    Arg::new("only-build")
                        .long("only-build")
                        .help("skip the run")
                        .takes_value(false),
                ),
        );
    let matches = cli.get_matches();

    let subcommand = if let Some(value) = matches.subcommand() {
        value
    } else {
        unreachable!("unreachable branch")
    };

    println!("Running subcommand `{:?}`", subcommand.0);

    let root = xtaskops::ops::root_dir();
    let res = match subcommand {
        ("install", sm) => install_dependency::install(sm.is_present("no-templates")),
        ("run", sm) => run::run(
            sm.is_present("editor"),
            sm.is_present("release"),
            sm.is_present("itest"),
            sm.is_present("only-build"),
        ),
        ("export", _m) => export::export(),
        ("coverage", sm) => coverage_with_itest(sm.is_present("dev")),
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
        _ => unreachable!("unreachable branch"),
    };
    res
    // xtaskops::tasks::main()
}

pub fn coverage_with_itest(devmode: bool) -> Result<(), anyhow::Error> {
    remove_dir("../coverage")?;
    create_dir_all("../coverage")?;

    println!("=== running coverage ===");
    cmd!("cargo", "test")
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTFLAGS", "-Cinstrument-coverage")
        .env("LLVM_PROFILE_FILE", "cargo-test-%p-%m.profraw")
        .dir(RUST_LIB_PROJECT_FOLDER)
        .run()?;

    cmd!("cargo", "run", "--", "run", "--itest")
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTFLAGS", "-Cinstrument-coverage")
        .env("LLVM_PROFILE_FILE", "cargo-test-%p-%m.profraw")
        .run()?;

    let err = glob::glob("./../../godot/*.profraw")?
        .filter_map(|entry| entry.ok())
        .map(|entry| cmd!("mv", entry, "./../decentraland-godot-lib/").run())
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
        "./decentraland-godot-lib/target/debug/deps",
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
    .dir("..")
    .run()?;
    println!("ok.");

    println!("=== cleaning up ===");
    clean_files("../**/*.profraw")?;
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
