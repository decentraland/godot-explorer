use anyhow::Context;
use clap::{AppSettings, Arg, Command};

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
            Command::new("run-godot-lib")
                .arg(
                    Arg::new("editor")
                        .short('e')
                        .long("editor")
                        .help("open godot editor mode")
                        .takes_value(false),
                )
                .arg(
                    Arg::new("debug")
                        .short('d')
                        .long("debug")
                        .help("build debug mode")
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
        Some(("run-godot-lib", sm)) => {
            let program = format!(
                "./../.bin/godot/{}",
                install_dependency::get_godot_executable_path().unwrap()
            );
            let mut args = vec!["--path", "./../godot"];
            if sm.is_present("editor") {
                args.push("-e");
            }

            let debug_mode = sm.is_present("debug");
            if debug_mode {
                xtaskops::ops::cmd!("cargo", "build").run()?;
            } else {
                xtaskops::ops::cmd!("cargo", "build", "--release").run()?;
            }

            match install_dependency::copy_library(debug_mode) {
                Ok(_) => Ok(()),
                Err(e) => Err(anyhow::anyhow!("copy the library failed: {}", e)),
            }?;

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
        Some(("coverage", sm)) => xtaskops::tasks::coverage(sm.is_present("dev")),
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
