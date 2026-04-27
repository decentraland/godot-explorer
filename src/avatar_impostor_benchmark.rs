use std::fs;
use std::path::PathBuf;

use crate::consts::GODOT_PROJECT_FOLDER;
use crate::path::get_godot_path;
use crate::ui::{print_message, print_section, MessageType};

pub fn run_avatar_impostor_benchmark(headless: bool) -> anyhow::Result<()> {
    print_section("Avatar Impostor Benchmark");
    print_message(
        MessageType::Info,
        if headless {
            "Running headless"
        } else {
            "Running with window"
        },
    );

    let output_dir = PathBuf::from("benchmark-results");
    fs::create_dir_all(&output_dir)?;
    let output_dir_abs = fs::canonicalize(&output_dir)?;
    let output_file = output_dir_abs.join("avatar-impostor-benchmark.txt");
    if output_file.exists() {
        let _ = fs::remove_file(&output_file);
    }
    let output_str = output_file.to_str().unwrap();

    let program = get_godot_path();
    let mut args: Vec<&str> = vec![
        "--path",
        GODOT_PROJECT_FOLDER,
        "--avatar-impostor-benchmark",
        "--avatar-impostor-benchmark-output",
        output_str,
    ];
    if headless {
        args.insert(2, "--headless");
    } else {
        args.insert(2, "--rendering-driver");
        args.insert(3, "vulkan");
    }

    print_message(
        MessageType::Step,
        &format!("Launching Godot: {} {}", program, args.join(" ")),
    );

    let status = std::process::Command::new(&program).args(&args).status()?;

    if !status.success() {
        print_message(
            MessageType::Warning,
            &format!("Godot exited with status: {}", status),
        );
    }

    if !output_file.exists() {
        anyhow::bail!(
            "Benchmark did not produce results at {}",
            output_file.display()
        );
    }

    print_message(
        MessageType::Success,
        &format!("Results written to {}", output_file.display()),
    );
    println!();
    let contents = fs::read_to_string(&output_file)?;
    println!("{}", contents);

    Ok(())
}
