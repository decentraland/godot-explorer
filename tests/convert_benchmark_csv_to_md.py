#!/usr/bin/env python3
"""
Convert benchmark CSV report to Markdown format with optional baseline comparison.

This script reads a CSV file containing benchmark metrics and generates
a Markdown report. It can optionally compare against a baseline CSV.

Usage:
    python convert_benchmark_csv_to_md.py <input.csv> <output.md> [--baseline <baseline.csv>]
    python convert_benchmark_csv_to_md.py <input.csv> <output.md> [--baseline-url <url>]
"""

import argparse
import csv
import sys
import urllib.request
from datetime import datetime
from typing import List, Dict, Optional, Tuple


class BenchmarkMetrics:
    """Represents benchmark metrics for a single test."""

    def __init__(self, row: Dict[str, str]):
        # Metadata
        self.test_name = row['test_name']
        self.timestamp = row['timestamp']
        self.location = row['location']
        self.realm = row['realm']

        # Process info
        self.process_memory_usage_mb = float(row['process_memory_usage_mb'])

        # Memory metrics
        self.godot_static_memory_mb = float(row['godot_static_memory_mb'])
        self.godot_static_memory_peak_mb = float(row['godot_static_memory_peak_mb'])
        self.gpu_video_ram_mb = float(row['gpu_video_ram_mb'])
        self.gpu_texture_memory_mb = float(row['gpu_texture_memory_mb'])
        self.gpu_buffer_memory_mb = float(row['gpu_buffer_memory_mb'])
        self.rust_heap_usage_mb = float(row['rust_heap_usage_mb'])
        self.rust_total_allocated_mb = float(row['rust_total_allocated_mb'])
        self.deno_total_memory_mb = float(row['deno_total_memory_mb'])
        self.deno_scene_count = int(row['deno_scene_count'])
        self.deno_average_memory_mb = float(row['deno_average_memory_mb'])

        # Object counts
        self.total_objects = int(row['total_objects'])
        self.resource_count = int(row['resource_count'])
        self.node_count = int(row['node_count'])
        self.orphan_node_count = int(row['orphan_node_count'])

        # Rendering
        self.fps = float(row['fps'])
        self.draw_calls = int(row['draw_calls'])
        self.primitives_in_frame = int(row['primitives_in_frame'])
        self.objects_in_frame = int(row['objects_in_frame'])

        # Resource analysis
        self.total_meshes = int(row['total_meshes'])
        self.total_materials = int(row['total_materials'])
        self.mesh_rid_count = int(row['mesh_rid_count'])
        self.material_rid_count = int(row['material_rid_count'])
        self.mesh_hash_count = int(row['mesh_hash_count'])
        self.potential_dedup_count = int(row['potential_dedup_count'])
        self.mesh_savings_percent = float(row['mesh_savings_percent'])

        # Mobile metrics (optional)
        self.mobile_memory_usage_mb = self._parse_optional_int(row['mobile_memory_usage_mb'])
        self.mobile_temperature_celsius = self._parse_optional_float(row['mobile_temperature_celsius'])
        self.mobile_battery_percent = self._parse_optional_int(row['mobile_battery_percent'])

    @staticmethod
    def _parse_optional_int(value: str) -> Optional[int]:
        return int(value) if value.strip() else None

    @staticmethod
    def _parse_optional_float(value: str) -> Optional[float]:
        return float(value) if value.strip() else None


def format_change(current: float, baseline: float, lower_is_better: bool = True) -> str:
    """Format a metric change with color indicator."""
    if baseline == 0:
        return f"{current:.2f}"

    diff = current - baseline
    pct_change = (diff / baseline) * 100

    # Determine if this is good or bad
    if abs(pct_change) < 0.5:  # Less than 0.5% change is neutral
        indicator = "⚪"
        color = ""
    elif (diff > 0 and lower_is_better) or (diff < 0 and not lower_is_better):
        # Worse
        indicator = "🔴"
        color = ""
    else:
        # Better
        indicator = "🟢"
        color = ""

    if abs(pct_change) < 0.5:
        return f"{current:.2f}"
    else:
        sign = "+" if diff > 0 else ""
        return f"{current:.2f} {indicator} ({sign}{pct_change:.1f}%)"


def format_change_int(current: int, baseline: int, lower_is_better: bool = True) -> str:
    """Format an integer metric change with color indicator."""
    if baseline == 0:
        return f"{current}"

    diff = current - baseline
    pct_change = (diff / baseline) * 100

    # Determine if this is good or bad
    if abs(pct_change) < 0.5:
        indicator = "⚪"
    elif (diff > 0 and lower_is_better) or (diff < 0 and not lower_is_better):
        indicator = "🔴"
    else:
        indicator = "🟢"

    if abs(pct_change) < 0.5:
        return f"{current}"
    else:
        sign = "+" if diff > 0 else ""
        return f"{current} {indicator} ({sign}{pct_change:.1f}%)"


def find_baseline_for_test(test_name: str, baseline_list: List[BenchmarkMetrics]) -> Optional[BenchmarkMetrics]:
    """Find matching baseline metrics for a test."""
    for baseline in baseline_list:
        if baseline.test_name == test_name:
            return baseline
    return None


def format_individual_report(metrics: BenchmarkMetrics, baseline: Optional[BenchmarkMetrics] = None) -> str:
    """Format individual test report."""
    report = []

    report.append(f"# Benchmark Report: {metrics.test_name}\n")
    report.append(f"**Timestamp**: {metrics.timestamp}\n")
    report.append(f"**Location**: {metrics.location}\n")
    if metrics.realm:
        report.append(f"**Realm**: {metrics.realm}\n")

    report.append("\n---\n")

    # Memory Metrics
    report.append("\n## Memory Metrics\n")
    report.append("\n| Metric | Value |\n")
    report.append("|--------|-------|\n")

    if baseline:
        report.append(f"| **Process Memory Usage (RSS)** | **{format_change(metrics.process_memory_usage_mb, baseline.process_memory_usage_mb)} MiB** |\n")
        report.append(f"| Godot Static Memory | {format_change(metrics.godot_static_memory_mb, baseline.godot_static_memory_mb)} MiB |\n")
        report.append(f"| Godot Peak Memory | {format_change(metrics.godot_static_memory_peak_mb, baseline.godot_static_memory_peak_mb)} MiB |\n")
        report.append(f"| GPU Video RAM | {format_change(metrics.gpu_video_ram_mb, baseline.gpu_video_ram_mb)} MiB |\n")
        report.append(f"| GPU Texture Memory | {format_change(metrics.gpu_texture_memory_mb, baseline.gpu_texture_memory_mb)} MiB |\n")
        report.append(f"| GPU Buffer Memory | {format_change(metrics.gpu_buffer_memory_mb, baseline.gpu_buffer_memory_mb)} MiB |\n")
        report.append(f"| Rust Heap Usage | {format_change(metrics.rust_heap_usage_mb, baseline.rust_heap_usage_mb)} MiB |\n")
        report.append(f"| Rust Total Allocated | {format_change(metrics.rust_total_allocated_mb, baseline.rust_total_allocated_mb)} MiB |\n")
    else:
        report.append(f"| **Process Memory Usage (RSS)** | **{metrics.process_memory_usage_mb:.2f} MiB ({metrics.process_memory_usage_mb / 1024.0:.2f} GiB)** |\n")
        report.append(f"| Godot Static Memory | {metrics.godot_static_memory_mb:.2f} MiB |\n")
        report.append(f"| Godot Peak Memory | {metrics.godot_static_memory_peak_mb:.2f} MiB |\n")
        report.append(f"| GPU Video RAM | {metrics.gpu_video_ram_mb:.2f} MiB |\n")
        report.append(f"| GPU Texture Memory | {metrics.gpu_texture_memory_mb:.2f} MiB |\n")
        report.append(f"| GPU Buffer Memory | {metrics.gpu_buffer_memory_mb:.2f} MiB |\n")
        report.append(f"| Rust Heap Usage | {metrics.rust_heap_usage_mb:.2f} MiB |\n")
        report.append(f"| Rust Total Allocated | {metrics.rust_total_allocated_mb:.2f} MiB |\n")

    if metrics.deno_scene_count > 0:
        if baseline and baseline.deno_scene_count > 0:
            report.append(f"| Deno/V8 Total Memory | {format_change(metrics.deno_total_memory_mb, baseline.deno_total_memory_mb)} MiB |\n")
            report.append(f"| Deno Active Scenes | {metrics.deno_scene_count} |\n")
            report.append(f"| Deno Avg per Scene | {format_change(metrics.deno_average_memory_mb, baseline.deno_average_memory_mb)} MiB |\n")
        else:
            report.append(f"| Deno/V8 Total Memory | {metrics.deno_total_memory_mb:.2f} MiB |\n")
            report.append(f"| Deno Active Scenes | {metrics.deno_scene_count} |\n")
            report.append(f"| Deno Avg per Scene | {metrics.deno_average_memory_mb:.2f} MiB |\n")
    report.append("\n")

    # Object Counts
    report.append("## Object Counts\n")
    report.append("\n| Metric | Count |\n")
    report.append("|--------|-------|\n")
    if baseline:
        report.append(f"| Total Objects | {format_change_int(metrics.total_objects, baseline.total_objects)} |\n")
        report.append(f"| Resources | {format_change_int(metrics.resource_count, baseline.resource_count)} |\n")
        report.append(f"| Nodes | {format_change_int(metrics.node_count, baseline.node_count)} |\n")
        report.append(f"| Orphan Nodes | {format_change_int(metrics.orphan_node_count, baseline.orphan_node_count)} |\n")
    else:
        report.append(f"| Total Objects | {metrics.total_objects} |\n")
        report.append(f"| Resources | {metrics.resource_count} |\n")
        report.append(f"| Nodes | {metrics.node_count} |\n")
        report.append(f"| Orphan Nodes | {metrics.orphan_node_count} |\n")
    report.append("\n")

    # Rendering
    report.append("## Rendering Metrics\n")
    report.append("\n| Metric | Value |\n")
    report.append("|--------|-------|\n")
    if baseline:
        report.append(f"| FPS | {format_change(metrics.fps, baseline.fps, lower_is_better=False)} |\n")
        report.append(f"| Draw Calls per Frame | {format_change_int(metrics.draw_calls, baseline.draw_calls)} |\n")
        report.append(f"| Primitives per Frame | {format_change_int(metrics.primitives_in_frame, baseline.primitives_in_frame)} |\n")
        report.append(f"| Objects per Frame | {format_change_int(metrics.objects_in_frame, baseline.objects_in_frame)} |\n")
    else:
        report.append(f"| FPS | {metrics.fps:.1f} |\n")
        report.append(f"| Draw Calls per Frame | {metrics.draw_calls} |\n")
        report.append(f"| Primitives per Frame | {metrics.primitives_in_frame} |\n")
        report.append(f"| Objects per Frame | {metrics.objects_in_frame} |\n")
    report.append("\n")

    # Resource Analysis
    if metrics.total_meshes > 0:
        report.append("## Resource Analysis\n")
        report.append("\n| Metric | Value |\n")
        report.append("|--------|-------|\n")
        if baseline and baseline.total_meshes > 0:
            report.append(f"| Total Mesh References | {format_change_int(metrics.total_meshes, baseline.total_meshes)} |\n")
            report.append(f"| Total Material References | {format_change_int(metrics.total_materials, baseline.total_materials)} |\n")
            report.append(f"| Unique Mesh RIDs | {format_change_int(metrics.mesh_rid_count, baseline.mesh_rid_count)} |\n")
            report.append(f"| Unique Material RIDs | {format_change_int(metrics.material_rid_count, baseline.material_rid_count)} |\n")
            report.append(f"| Hashed Mesh Count | {format_change_int(metrics.mesh_hash_count, baseline.mesh_hash_count)} |\n")
            report.append(f"| Potential Deduplication | {format_change_int(metrics.potential_dedup_count, baseline.potential_dedup_count)} ({metrics.mesh_savings_percent:.1f}% savings) |\n")
        else:
            report.append(f"| Total Mesh References | {metrics.total_meshes} |\n")
            report.append(f"| Total Material References | {metrics.total_materials} |\n")
            report.append(f"| Unique Mesh RIDs | {metrics.mesh_rid_count} |\n")
            report.append(f"| Unique Material RIDs | {metrics.material_rid_count} |\n")
            report.append(f"| Hashed Mesh Count | {metrics.mesh_hash_count} |\n")
            report.append(f"| Potential Deduplication | {metrics.potential_dedup_count} ({metrics.mesh_savings_percent:.1f}% savings) |\n")
        report.append("\n")

    # Mobile Metrics
    if metrics.mobile_memory_usage_mb is not None:
        report.append("## Mobile Metrics\n")
        report.append("\n| Metric | Value |\n")
        report.append("|--------|-------|\n")
        if metrics.mobile_memory_usage_mb is not None:
            report.append(f"| Memory Usage | {metrics.mobile_memory_usage_mb} MiB |\n")
        if metrics.mobile_temperature_celsius is not None:
            report.append(f"| Temperature | {metrics.mobile_temperature_celsius:.1f}°C |\n")
        if metrics.mobile_battery_percent is not None:
            report.append(f"| Battery | {metrics.mobile_battery_percent}% |\n")
        report.append("\n")

    return ''.join(report)


def format_consolidated_report(metrics_list: List[BenchmarkMetrics], baseline_list: Optional[List[BenchmarkMetrics]] = None, baseline_failed: bool = False) -> str:
    """Format consolidated report with all tests."""
    report = []

    report.append("# Decentraland Godot Explorer - Benchmark Report\n")
    report.append(f"\n**Generated**: {datetime.now().strftime('%Y-%m-%d_%H-%M-%S')}\n")
    report.append(f"\n**Total Tests**: {len(metrics_list)}\n")

    if baseline_list:
        report.append("\n**📊 Comparison**: vs main branch baseline\n")
        report.append("- 🟢 = Improvement (better performance)\n")
        report.append("- 🔴 = Regression (worse performance)\n")
        report.append("- ⚪ = No significant change (<0.5%)\n")
    elif baseline_failed:
        report.append("\n> ⚠️ **No Baseline Available**: Baseline benchmark not found. This is likely the first run for this branch, or the baseline branch hasn't been benchmarked yet. Showing absolute values only.\n")

    report.append("\n---\n")

    # Table of Contents
    report.append("\n## Table of Contents\n")
    for i, metrics in enumerate(metrics_list, 1):
        anchor = metrics.test_name.lower().replace(' ', '-').replace('_', '-')
        report.append(f"\n{i}. [{metrics.test_name}](#test-{i}-{anchor})")
    report.append("\n\n---\n")

    # Summary Overview
    report.append("\n## Summary Overview\n")

    # Memory Metrics
    report.append("\n### Memory Metrics\n")
    report.append("\n| Test | Process RSS (MiB) | Godot Static (MiB) | GPU VRAM (MiB) | Rust Heap (MiB) | Deno Total (MiB) |\n")
    report.append("|------|-------------------|-------------------|----------------|-----------------|------------------|\n")
    for metrics in metrics_list:
        baseline = find_baseline_for_test(metrics.test_name, baseline_list) if baseline_list else None
        if baseline:
            report.append(f"| {metrics.test_name} | {format_change(metrics.process_memory_usage_mb, baseline.process_memory_usage_mb)} | {format_change(metrics.godot_static_memory_mb, baseline.godot_static_memory_mb)} | {format_change(metrics.gpu_video_ram_mb, baseline.gpu_video_ram_mb)} | {format_change(metrics.rust_heap_usage_mb, baseline.rust_heap_usage_mb)} | {format_change(metrics.deno_total_memory_mb, baseline.deno_total_memory_mb)} |\n")
        else:
            report.append(f"| {metrics.test_name} | {metrics.process_memory_usage_mb:.2f} | {metrics.godot_static_memory_mb:.2f} | {metrics.gpu_video_ram_mb:.2f} | {metrics.rust_heap_usage_mb:.2f} | {metrics.deno_total_memory_mb:.2f} |\n")
    report.append("\n")

    # Objects Summary
    report.append("### Object Counts\n")
    report.append("\n| Test | Total Objects | Nodes | Resources | Orphan Nodes |\n")
    report.append("|------|---------------|-------|-----------|---------------|\n")
    for metrics in metrics_list:
        baseline = find_baseline_for_test(metrics.test_name, baseline_list) if baseline_list else None
        if baseline:
            report.append(f"| {metrics.test_name} | {format_change_int(metrics.total_objects, baseline.total_objects)} | {format_change_int(metrics.node_count, baseline.node_count)} | {format_change_int(metrics.resource_count, baseline.resource_count)} | {format_change_int(metrics.orphan_node_count, baseline.orphan_node_count)} |\n")
        else:
            report.append(f"| {metrics.test_name} | {metrics.total_objects} | {metrics.node_count} | {metrics.resource_count} | {metrics.orphan_node_count} |\n")
    report.append("\n")

    # Rendering Summary
    report.append("### Rendering Metrics\n")
    report.append("\n| Test | FPS | Draw Calls | Primitives | Objects in Frame |\n")
    report.append("|------|-----|------------|------------|------------------|\n")
    for metrics in metrics_list:
        baseline = find_baseline_for_test(metrics.test_name, baseline_list) if baseline_list else None
        if baseline:
            report.append(f"| {metrics.test_name} | {format_change(metrics.fps, baseline.fps, lower_is_better=False)} | {format_change_int(metrics.draw_calls, baseline.draw_calls)} | {format_change_int(metrics.primitives_in_frame, baseline.primitives_in_frame)} | {format_change_int(metrics.objects_in_frame, baseline.objects_in_frame)} |\n")
        else:
            report.append(f"| {metrics.test_name} | {metrics.fps:.1f} | {metrics.draw_calls} | {metrics.primitives_in_frame} | {metrics.objects_in_frame} |\n")
    report.append("\n")

    # Resource Analysis Summary
    report.append("### Resource Analysis\n")
    report.append("\n| Test | Meshes | Materials | Mesh RIDs | Material RIDs | Dedup Potential |\n")
    report.append("|------|--------|-----------|-----------|---------------|------------------|\n")
    for metrics in metrics_list:
        baseline = find_baseline_for_test(metrics.test_name, baseline_list) if baseline_list else None
        if baseline:
            report.append(f"| {metrics.test_name} | {format_change_int(metrics.total_meshes, baseline.total_meshes)} | {format_change_int(metrics.total_materials, baseline.total_materials)} | {format_change_int(metrics.mesh_rid_count, baseline.mesh_rid_count)} | {format_change_int(metrics.material_rid_count, baseline.material_rid_count)} | {format_change_int(metrics.potential_dedup_count, baseline.potential_dedup_count)} |\n")
        else:
            report.append(f"| {metrics.test_name} | {metrics.total_meshes} | {metrics.total_materials} | {metrics.mesh_rid_count} | {metrics.material_rid_count} | {metrics.potential_dedup_count} |\n")
    report.append("\n---\n")

    # Detailed Test Results
    report.append("\n## Detailed Test Results\n")
    for i, metrics in enumerate(metrics_list, 1):
        baseline = find_baseline_for_test(metrics.test_name, baseline_list) if baseline_list else None
        report.append(f"\n### Test {i}: {metrics.test_name}\n")
        report.append(format_individual_report(metrics, baseline))
        report.append("\n---\n")

    return ''.join(report)


def load_csv(file_path: str) -> List[BenchmarkMetrics]:
    """Load benchmark metrics from CSV file."""
    metrics_list = []
    with open(file_path, 'r', encoding='utf-8') as f:
        reader = csv.DictReader(f)
        for row in reader:
            metrics = BenchmarkMetrics(row)
            metrics_list.append(metrics)
    return metrics_list


def load_csv_from_url(url: str) -> Optional[List[BenchmarkMetrics]]:
    """Load benchmark metrics from URL."""
    try:
        print(f"📥 Downloading baseline from: {url}")
        with urllib.request.urlopen(url) as response:
            content = response.read().decode('utf-8')
            lines = content.strip().split('\n')
            reader = csv.DictReader(lines)
            metrics_list = []
            for row in reader:
                metrics = BenchmarkMetrics(row)
                metrics_list.append(metrics)
            print(f"✓ Loaded {len(metrics_list)} baseline test(s)")
            return metrics_list
    except Exception as e:
        print(f"⚠️ Could not load baseline from URL: {e}")
        return None


def main():
    parser = argparse.ArgumentParser(description='Convert benchmark CSV to Markdown with optional comparison')
    parser.add_argument('input_csv', help='Input CSV file')
    parser.add_argument('output_md', help='Output Markdown file')
    parser.add_argument('--baseline', help='Baseline CSV file for comparison', default=None)
    parser.add_argument('--baseline-url', help='Baseline CSV URL for comparison', default=None)

    args = parser.parse_args()

    # Read current metrics
    try:
        metrics_list = load_csv(args.input_csv)
    except FileNotFoundError:
        print(f"Error: File '{args.input_csv}' not found")
        sys.exit(1)
    except Exception as e:
        print(f"Error reading CSV: {e}")
        sys.exit(1)

    if not metrics_list:
        print("Error: No metrics found in CSV")
        sys.exit(1)

    # Load baseline if provided
    baseline_list = None
    baseline_failed = False
    if args.baseline:
        try:
            baseline_list = load_csv(args.baseline)
            print(f"✓ Loaded {len(baseline_list)} baseline test(s) from file")
        except Exception as e:
            print(f"⚠️ Could not load baseline file: {e}")
            baseline_failed = True
    elif args.baseline_url:
        baseline_list = load_csv_from_url(args.baseline_url)
        if baseline_list is None:
            baseline_failed = True

    # Generate markdown
    markdown = format_consolidated_report(metrics_list, baseline_list, baseline_failed)

    # Write markdown
    try:
        with open(args.output_md, 'w', encoding='utf-8') as f:
            f.write(markdown)
        print(f"✓ Converted {len(metrics_list)} test(s) to Markdown: {args.output_md}")
        if baseline_list:
            print(f"✓ Comparison with baseline included")
    except Exception as e:
        print(f"Error writing Markdown: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
