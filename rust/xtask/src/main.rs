use std::{collections::HashMap, fs::create_dir_all};

use anyhow::Context;
use clap::{AppSettings, Arg, Command};
use xtaskops::ops::{clean_files, cmd, confirm, remove_dir};

use crate::consts::RUST_LIB_PROJECT_FOLDER;

mod consts;
mod copy_files;
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
        .subcommand(Command::new("update-protocol"))
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
                        .help("run integration-tests")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("stest")
                        .long("stest")
                        .help("run scene-tests")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("only-build")
                        .long("only-build")
                        .help("skip the run")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("link-libs")
                        .short('l')
                        .help("link libs instead of copying (only linux)")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("extras")
                        .last(true)
                        .allow_hyphen_values(true)
                        .multiple(true),
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
        ("update-protocol", _) => install_dependency::install_dcl_protocol(),
        ("run", sm) => run::run(
            sm.is_present("editor"),
            sm.is_present("release"),
            sm.is_present("itest"),
            sm.is_present("only-build"),
            sm.is_present("link-libs"),
            sm.is_present("stest"),
            sm.values_of("extras")
                .map(|v| v.map(|it| it.into()).collect())
                .unwrap_or_default(),
            None,
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

    if res.is_err() {
        println!("Fail running subcommand `{:?}`", subcommand.0);
    }
    res
    // xtaskops::tasks::main()
}

pub fn coverage_with_itest(devmode: bool) -> Result<(), anyhow::Error> {
    remove_dir("../coverage")?;
    create_dir_all("../coverage")?;

    println!("=== running coverage ===");
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

    run::run(
        false,
        false,
        true,
        false,
        false,
        false,
        vec![],
        Some(build_envs.clone()),
    )?;

    let extra_args = [
        "--rendering-driver",
        "opengl3",
        "--scene-test",
        "[[52,-52],[52,-54],[52,-56],[52,-58],[52,-60],[52,-62],[52,-64],[52,-66],[52,-68],[54,-52],[54,-54],[54,-56],[54,-58],[54,-60]]",
        "--realm",
        "https://decentraland.github.io/scene-explorer-tests/scene-explorer-tests",
    ]
    .iter()
    .map(|it| it.to_string())
    .collect();

    run::run(
        false,
        false,
        false,
        false,
        false,
        true,
        extra_args,
        Some(build_envs.clone()),
    )?;

    let err = glob::glob("./../../godot/*.profraw")?
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            println!("moving {:?} to ./../decentraland-godot-lib/", entry);
            cmd!("mv", entry, "./../decentraland-godot-lib/").run()
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
