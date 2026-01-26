use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::consts::GODOT_PROJECT_FOLDER;
use crate::path::get_godot_path;
use crate::ui::{print_message, print_section, MessageType};

/// Benchmark configurations: (arm_length, title, description)
/// 0 = single parcel with its ring of empty parcels
/// Larger values = 4 parcels in cross, more empty parcels to fill the gaps
const BENCHMARK_CONFIGS: &[(u32, &str, &str)] = &[
    (0, "Single Parcel", "1 scene parcel with surrounding empty parcels"),
    (2, "Big Scene", "4 parcels in cross (2x2 equivalent), typical large scene"),
    (4, "Worst Case Real", "4 parcels spread apart, realistic worst case"),
    (6, "Extreme", "4 parcels very spread apart, unrealistic stress test"),
];

/// Run the floating islands benchmark with multiple client sessions
pub fn run_fi_benchmark(headless: bool) -> anyhow::Result<()> {
    print_section("Floating Islands Memory Benchmark");
    print_message(
        MessageType::Info,
        &format!(
            "Running {} benchmark sessions{}: {:?}",
            BENCHMARK_CONFIGS.len(),
            if headless { " (headless)" } else { " (with window + screenshots)" },
            BENCHMARK_CONFIGS.iter().map(|(s, t, _)| format!("{}={}", t, s)).collect::<Vec<_>>()
        ),
    );

    let program = get_godot_path();
    let output_dir = get_output_dir()?;
    let output_dir_abs = std::fs::canonicalize(&output_dir)?;
    let mut results: Vec<(u32, String, String, PathBuf)> = vec![];

    for (size, title, description) in BENCHMARK_CONFIGS {
        print_message(
            MessageType::Step,
            &format!("Running '{}' (arm_length={})...", title, size),
        );

        let output_file = output_dir_abs.join(format!("fi-benchmark-{}.json", size));
        let size_str = size.to_string();
        let output_str = output_file.to_str().unwrap();

        let status = if headless {
            std::process::Command::new(&program)
                .args([
                    "--path",
                    GODOT_PROJECT_FOLDER,
                    "--headless",
                    "--skip-lobby",
                    "--fi-benchmark-size",
                    &size_str,
                    "--fi-benchmark-output",
                    output_str,
                ])
                .status()?
        } else {
            std::process::Command::new(&program)
                .args([
                    "--path",
                    GODOT_PROJECT_FOLDER,
                    "--rendering-driver",
                    "vulkan",
                    "--skip-lobby",
                    "--fi-benchmark-size",
                    &size_str,
                    "--fi-benchmark-output",
                    output_str,
                ])
                .status()?
        };

        if !status.success() {
            print_message(
                MessageType::Warning,
                &format!("'{}' exited with status: {}", title, status),
            );
        } else if output_file.exists() {
            print_message(
                MessageType::Success,
                &format!("'{}' completed", title),
            );
            results.push((*size, title.to_string(), description.to_string(), output_file));
        } else {
            print_message(
                MessageType::Warning,
                &format!("'{}' did not produce output file", title),
            );
        }
    }

    // Generate reports
    if !results.is_empty() {
        generate_csv_report(&results, &output_dir)?;
        generate_markdown_report(&results, &output_dir)?;
    } else {
        print_message(MessageType::Error, "No benchmark results were generated");
        return Err(anyhow::anyhow!("No benchmark results generated"));
    }

    print_message(MessageType::Success, "Floating Islands Benchmark complete!");
    Ok(())
}

/// Get the output directory for benchmark results
fn get_output_dir() -> anyhow::Result<PathBuf> {
    let output_dir = PathBuf::from("output");
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)?;
    }
    Ok(output_dir)
}

/// Parse a benchmark JSON result file
fn parse_benchmark_result(path: &Path) -> Option<serde_json::Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
}

/// Generate CSV report
fn generate_csv_report(results: &[(u32, String, String, PathBuf)], output_dir: &Path) -> anyhow::Result<()> {
    print_message(MessageType::Step, "Generating CSV report...");

    let csv_path = output_dir.join("floating_islands_benchmark.csv");
    let file = File::create(&csv_path)?;
    let mut writer = BufWriter::new(file);

    writeln!(
        writer,
        "scenario,arm_length,empty_parcels,generation_time_ms,memory_delta_mb,node_count,vram_delta_mb"
    )?;

    for (size, title, _, result_path) in results {
        if let Some(json) = parse_benchmark_result(result_path) {
            let generation_time = json["generation_time_ms"].as_i64().unwrap_or(0);
            let delta = &json["delta"];
            let memory_delta = delta["memory_static_mb"].as_f64().unwrap_or(0.0);
            let node_count = delta["object_node_count"].as_f64().unwrap_or(0.0) as i64;
            let vram_delta = delta["video_mem_used_mb"].as_f64().unwrap_or(0.0);

            // Count empty parcels from node breakdown (terrain count = empty parcel count)
            let empty_parcels = json["node_breakdown"]["terrain"]
                .as_i64()
                .or_else(|| json["node_breakdown"]["terrain"].as_f64().map(|f| f as i64))
                .unwrap_or(0);

            writeln!(
                writer,
                "\"{}\",{},{},{},{:.1},{},{:.1}",
                title, size, empty_parcels, generation_time, memory_delta, node_count, vram_delta
            )?;
        }
    }

    writer.flush()?;
    print_message(MessageType::Success, &format!("CSV: {:?}", csv_path));
    Ok(())
}

/// Generate Markdown report with screenshots
fn generate_markdown_report(results: &[(u32, String, String, PathBuf)], output_dir: &Path) -> anyhow::Result<()> {
    print_message(MessageType::Step, "Generating Markdown report...");

    let md_path = output_dir.join("floating_islands_benchmark.md");
    let file = File::create(&md_path)?;
    let mut w = BufWriter::new(file);

    writeln!(w, "# Floating Islands Memory Benchmark")?;
    writeln!(w)?;
    writeln!(w, "This benchmark measures memory and performance impact of floating islands generation.")?;
    writeln!(w)?;
    writeln!(w, "**Test pattern:** 4 scene parcels arranged in a cross pattern with increasing separation.")?;
    writeln!(w, "This represents the worst-case scenario where empty parcels must fill large gaps.")?;
    writeln!(w)?;

    // Summary table
    writeln!(w, "## Summary")?;
    writeln!(w)?;
    writeln!(w, "| Scenario | Empty Parcels | Nodes | Memory | VRAM | Gen Time |")?;
    writeln!(w, "|----------|---------------|-------|--------|------|----------|")?;

    for (size, title, _, result_path) in results {
        if let Some(json) = parse_benchmark_result(result_path) {
            let generation_time = json["generation_time_ms"].as_i64().unwrap_or(0);
            let delta = &json["delta"];
            let memory_delta = delta["memory_static_mb"].as_f64().unwrap_or(0.0);
            let node_count = delta["object_node_count"].as_f64().unwrap_or(0.0) as i64;
            let vram_delta = delta["video_mem_used_mb"].as_f64().unwrap_or(0.0);
            let empty_parcels = json["node_breakdown"]["terrain"]
                .as_i64()
                .or_else(|| json["node_breakdown"]["terrain"].as_f64().map(|f| f as i64))
                .unwrap_or(0);

            writeln!(
                w,
                "| **{}** (arm={}) | {} | {} | {:.0} MB | {:.0} MB | {:.1}s |",
                title, size, empty_parcels, node_count, memory_delta, vram_delta,
                generation_time as f64 / 1000.0
            )?;
        }
    }
    writeln!(w)?;

    // Detailed results with screenshots
    writeln!(w, "## Detailed Results")?;
    writeln!(w)?;

    for (size, title, description, result_path) in results {
        writeln!(w, "### {}", title)?;
        writeln!(w)?;
        writeln!(w, "{}", description)?;
        writeln!(w)?;

        // Screenshot
        let screenshot_name = format!("fi-benchmark-{}.png", size);
        let screenshot_path = output_dir.join(&screenshot_name);
        if screenshot_path.exists() {
            writeln!(w, "![{}]({})", title, screenshot_name)?;
            writeln!(w)?;
        }

        if let Some(json) = parse_benchmark_result(result_path) {
            let generation_time = json["generation_time_ms"].as_i64().unwrap_or(0);
            let delta = &json["delta"];
            let memory_delta = delta["memory_static_mb"].as_f64().unwrap_or(0.0);
            let node_count = delta["object_node_count"].as_f64().unwrap_or(0.0) as i64;
            let vram_delta = delta["video_mem_used_mb"].as_f64().unwrap_or(0.0);

            let breakdown = &json["node_breakdown"];
            let terrain = breakdown["terrain"].as_i64().or_else(|| breakdown["terrain"].as_f64().map(|f| f as i64)).unwrap_or(0);
            let cliff = breakdown["cliff"].as_i64().or_else(|| breakdown["cliff"].as_f64().map(|f| f as i64)).unwrap_or(0);
            let grass = breakdown["grass"].as_i64().or_else(|| breakdown["grass"].as_f64().map(|f| f as i64)).unwrap_or(0);
            let tree = breakdown["tree"].as_i64().or_else(|| breakdown["tree"].as_f64().map(|f| f as i64)).unwrap_or(0);

            writeln!(w, "| Metric | Value |")?;
            writeln!(w, "|--------|-------|")?;
            writeln!(w, "| Empty Parcels | {} |", terrain)?;
            writeln!(w, "| Total Nodes | {} |", node_count)?;
            writeln!(w, "| Memory Delta | {:.1} MB |", memory_delta)?;
            writeln!(w, "| VRAM Delta | {:.1} MB |", vram_delta)?;
            writeln!(w, "| Generation Time | {:.2}s |", generation_time as f64 / 1000.0)?;
            writeln!(w)?;
            writeln!(w, "**Node breakdown:** {} terrain, {} cliffs, {} grass, {} trees", terrain, cliff, grass, tree)?;
            writeln!(w)?;
        }
    }

    w.flush()?;
    print_message(MessageType::Success, &format!("Markdown: {:?}", md_path));
    Ok(())
}
