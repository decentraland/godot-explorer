use anyhow::Context;
use clap::{AppSettings, Arg, Command};

mod proto_dependency;

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
        .subcommand(Command::new("install-protocol"));
    let matches = cli.get_matches();

    let root = xtaskops::ops::root_dir();
    let res = match matches.subcommand() {
        Some(("install-protocol", _)) => match proto_dependency::install_dependency() {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("install-protocol failed: {}", e)),
        },
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
