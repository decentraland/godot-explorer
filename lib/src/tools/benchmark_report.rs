//! # Benchmark Report Tool
//!
//! Comprehensive benchmark reporting system that collects memory, performance, and resource metrics
//! and generates markdown reports for analysis.
//!
//! ## Features
//! - Collects metrics from multiple sources:
//!   - Godot memory (static, peak)
//!   - GPU memory (video RAM, textures, buffers)
//!   - Rust heap (via memory_debugger allocator tracking)
//!   - Deno/V8 memory (JavaScript runtime)
//!   - Object counts (total objects, nodes, resources, orphan nodes)
//!   - Rendering metrics (FPS, draw calls, primitives, objects in frame)
//!   - Resource analysis (meshes, materials, RIDs, deduplication potential)
//!   - Mobile metrics (memory usage, temperature, battery - iOS/Android only)
//!
//! - Generates two types of reports:
//!   - Individual reports: Detailed metrics for each test
//!   - Summary reports: Comparison tables across all tests
//!
//! ## Usage
//!
//! ### From Command Line
//! ```bash
//! # Run benchmark with markdown reports
//! cargo run -- run -- --benchmark-report --realm https://realm-provider.decentraland.org/main --location 0,0
//!
//! # Run automated benchmark suite (Genesis Plaza + Goerli Plaza)
//! cargo run -- run -- --benchmark-report
//! ```
//!
//! ### Test Locations
//! The tool tests the following locations by default:
//! - Genesis Plaza: (0, 0)
//! - Goerli Plaza: (-9, -9)
//!
//! Additional locations can be enabled by uncommenting them in:
//! `godot/addons/dcl_dev_tools/dev_tools/resource_counter/tool.gd`
//!
//! ### Output
//! Reports are saved to:
//! - Individual: `[USER_DATA]/output/reports/[test-name]_[timestamp].md`
//! - Summary: `[USER_DATA]/output/reports/summary_[timestamp].md`
//!
//! Where USER_DATA is the Godot user data directory (varies by platform).
//!
//! ### Integration with GDScript
//! The BenchmarkReport node should be added as a child of Global and can be accessed via:
//! ```gdscript
//! var benchmark_report = Global.get_node("BenchmarkReport")
//! benchmark_report.collect_and_store_metrics(test_name, location, realm, resource_data)
//! benchmark_report.generate_individual_report()
//! benchmark_report.generate_summary_report()
//! ```
//!
//! ## Requirements
//! - `use_memory_debugger` feature for Rust heap tracking
//! - `use_deno` feature for JavaScript runtime memory tracking
//! - BenchmarkReport node must be instantiated in the scene tree

use godot::{
    engine::{performance::Monitor, Os, Performance},
    prelude::*,
};

use crate::godot_classes::{
    dcl_android_plugin::DclGodotAndroidPlugin, dcl_ios_plugin::DclIosPlugin,
};

use std::fs::{self, File};
use std::io::Write as IoWrite;
use std::path::PathBuf;

#[cfg(feature = "use_memory_debugger")]
use crate::tools::memory_debugger::{ALLOCATED, DEALLOCATED};
#[cfg(feature = "use_memory_debugger")]
use std::sync::atomic::Ordering;

/// Benchmark metrics collected for a single test
#[derive(Debug, Clone)]
pub struct BenchmarkMetrics {
    pub test_name: String,
    pub timestamp: String,
    pub location: String,
    pub realm: String,

    // Memory metrics
    pub godot_static_memory_mb: f64,
    pub godot_static_memory_peak_mb: f64,
    pub gpu_video_ram_mb: f64,
    pub gpu_texture_memory_mb: f64,
    pub gpu_buffer_memory_mb: f64,
    pub rust_heap_usage_mb: f64,
    pub rust_total_allocated_mb: f64,
    pub deno_total_memory_mb: f64,
    pub deno_scene_count: i32,
    pub deno_average_memory_mb: f64,

    // Object counts
    pub total_objects: i64,
    pub resource_count: i64,
    pub node_count: i64,
    pub orphan_node_count: i64,

    // Rendering
    pub fps: f64,
    pub draw_calls: i64,
    pub primitives_in_frame: i64,
    pub objects_in_frame: i64,

    // Resource analysis (from GDScript)
    pub total_meshes: i32,
    pub total_materials: i32,
    pub mesh_rid_count: i32,
    pub material_rid_count: i32,
    pub mesh_hash_count: i32,
    pub potential_dedup_count: i32,
    pub mesh_savings_percent: f64,

    // Mobile metrics (optional)
    pub mobile_memory_usage_mb: Option<i32>,
    pub mobile_temperature_celsius: Option<f32>,
    pub mobile_battery_percent: Option<i32>,

    // Process info
    pub process_memory_usage_mb: f64,
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct BenchmarkReport {
    base: Base<Node>,
    metrics_history: Vec<BenchmarkMetrics>,
    output_dir: PathBuf,

    #[export]
    scene_manager_path: Option<Gd<Node>>,
}

#[godot_api]
impl INode for BenchmarkReport {
    fn init(base: Base<Node>) -> Self {
        // Create output directory
        let output_dir = PathBuf::from("res://output/reports");

        Self {
            base,
            metrics_history: Vec::new(),
            output_dir,
            scene_manager_path: None,
        }
    }

    fn ready(&mut self) {
        tracing::info!("BenchmarkReport initialized");

        // Create output directory if it doesn't exist
        let user_dir = Os::singleton().get_user_data_dir().to_string();
        let full_output_dir = if self.output_dir.starts_with("res://") {
            PathBuf::from(user_dir).join("output").join("reports")
        } else {
            self.output_dir.clone()
        };

        if let Err(e) = fs::create_dir_all(&full_output_dir) {
            tracing::error!("Failed to create output directory: {:?}", e);
        } else {
            tracing::info!("Output directory: {:?}", full_output_dir);
        }

        // Automatically find and set the scene_manager_path if not already set
        if self.scene_manager_path.is_none() {
            if let Some(parent) = self.base().get_parent() {
                let scene_runner_node = parent.get_node_or_null("scene_runner".into());
                if let Some(node) = scene_runner_node {
                    self.scene_manager_path = Some(node);
                    tracing::info!("BenchmarkReport: Automatically found scene_runner");
                }
            }
        }
    }
}

#[godot_api]
impl BenchmarkReport {
    /// Collect all current metrics
    #[func]
    pub fn collect_metrics(
        &self,
        test_name: GString,
        location: GString,
        realm: GString,
    ) -> Dictionary {
        let performance = Performance::singleton();

        // Memory metrics (in MiB)
        let godot_static_memory_mb =
            performance.get_monitor(Monitor::MEMORY_STATIC) as f64 / 1_048_576.0;
        let godot_static_memory_peak_mb =
            performance.get_monitor(Monitor::MEMORY_STATIC_MAX) as f64 / 1_048_576.0;
        let gpu_video_ram_mb =
            performance.get_monitor(Monitor::RENDER_VIDEO_MEM_USED) as f64 / 1_048_576.0;
        let gpu_texture_memory_mb =
            performance.get_monitor(Monitor::RENDER_TEXTURE_MEM_USED) as f64 / 1_048_576.0;
        let gpu_buffer_memory_mb =
            performance.get_monitor(Monitor::RENDER_BUFFER_MEM_USED) as f64 / 1_048_576.0;

        // Rust heap memory
        #[cfg(feature = "use_memory_debugger")]
        let (rust_heap_usage_mb, rust_total_allocated_mb) = {
            let allocated = ALLOCATED.load(Ordering::Relaxed);
            let deallocated = DEALLOCATED.load(Ordering::Relaxed);
            let current_usage = allocated.saturating_sub(deallocated);
            (
                current_usage as f64 / 1_048_576.0,
                allocated as f64 / 1_048_576.0,
            )
        };
        #[cfg(not(feature = "use_memory_debugger"))]
        let (rust_heap_usage_mb, rust_total_allocated_mb) = (0.0, 0.0);

        // Deno memory
        let (deno_total_memory_mb, deno_scene_count, deno_average_memory_mb) =
            self.get_deno_metrics();

        // Object counts
        let total_objects = performance.get_monitor(Monitor::OBJECT_COUNT) as i64;
        let resource_count = performance.get_monitor(Monitor::OBJECT_RESOURCE_COUNT) as i64;
        let node_count = performance.get_monitor(Monitor::OBJECT_NODE_COUNT) as i64;
        let orphan_node_count = performance.get_monitor(Monitor::OBJECT_ORPHAN_NODE_COUNT) as i64;

        // Rendering
        let fps = performance.get_monitor(Monitor::TIME_FPS) as f64;
        let draw_calls = performance.get_monitor(Monitor::RENDER_TOTAL_DRAW_CALLS_IN_FRAME) as i64;
        let primitives_in_frame =
            performance.get_monitor(Monitor::RENDER_TOTAL_PRIMITIVES_IN_FRAME) as i64;
        let objects_in_frame =
            performance.get_monitor(Monitor::RENDER_TOTAL_OBJECTS_IN_FRAME) as i64;

        // Mobile metrics
        let (mobile_memory_usage_mb, mobile_temperature_celsius, mobile_battery_percent) =
            self.get_mobile_metrics();

        // Create dictionary with metrics
        let mut dict = Dictionary::new();
        dict.set("test_name", test_name.to_variant());
        dict.set("timestamp", self.get_timestamp().to_variant());
        dict.set("location", location.to_variant());
        dict.set("realm", realm.to_variant());

        dict.set(
            "godot_static_memory_mb",
            godot_static_memory_mb.to_variant(),
        );
        dict.set(
            "godot_static_memory_peak_mb",
            godot_static_memory_peak_mb.to_variant(),
        );
        dict.set("gpu_video_ram_mb", gpu_video_ram_mb.to_variant());
        dict.set("gpu_texture_memory_mb", gpu_texture_memory_mb.to_variant());
        dict.set("gpu_buffer_memory_mb", gpu_buffer_memory_mb.to_variant());
        dict.set("rust_heap_usage_mb", rust_heap_usage_mb.to_variant());
        dict.set(
            "rust_total_allocated_mb",
            rust_total_allocated_mb.to_variant(),
        );
        dict.set("deno_total_memory_mb", deno_total_memory_mb.to_variant());
        dict.set("deno_scene_count", deno_scene_count.to_variant());
        dict.set(
            "deno_average_memory_mb",
            deno_average_memory_mb.to_variant(),
        );

        dict.set("total_objects", total_objects.to_variant());
        dict.set("resource_count", resource_count.to_variant());
        dict.set("node_count", node_count.to_variant());
        dict.set("orphan_node_count", orphan_node_count.to_variant());

        dict.set("fps", fps.to_variant());
        dict.set("draw_calls", draw_calls.to_variant());
        dict.set("primitives_in_frame", primitives_in_frame.to_variant());
        dict.set("objects_in_frame", objects_in_frame.to_variant());

        if let Some(mem) = mobile_memory_usage_mb {
            dict.set("mobile_memory_usage_mb", mem.to_variant());
        }
        if let Some(temp) = mobile_temperature_celsius {
            dict.set("mobile_temperature_celsius", temp.to_variant());
        }
        if let Some(battery) = mobile_battery_percent {
            dict.set("mobile_battery_percent", battery.to_variant());
        }

        // Process info
        let process_memory_usage_mb = self.get_process_memory_usage_mb();
        dict.set(
            "process_memory_usage_mb",
            process_memory_usage_mb.to_variant(),
        );

        dict
    }

    /// Collect metrics and add resource analysis data from GDScript
    #[func]
    pub fn collect_and_store_metrics(
        &mut self,
        test_name: GString,
        location: GString,
        realm: GString,
        resource_data: Dictionary,
    ) {
        let performance = Performance::singleton();

        let metrics = BenchmarkMetrics {
            test_name: test_name.to_string(),
            timestamp: self.get_timestamp(),
            location: location.to_string(),
            realm: realm.to_string(),

            // Memory metrics
            godot_static_memory_mb: performance.get_monitor(Monitor::MEMORY_STATIC) as f64
                / 1_048_576.0,
            godot_static_memory_peak_mb: performance.get_monitor(Monitor::MEMORY_STATIC_MAX) as f64
                / 1_048_576.0,
            gpu_video_ram_mb: performance.get_monitor(Monitor::RENDER_VIDEO_MEM_USED) as f64
                / 1_048_576.0,
            gpu_texture_memory_mb: performance.get_monitor(Monitor::RENDER_TEXTURE_MEM_USED) as f64
                / 1_048_576.0,
            gpu_buffer_memory_mb: performance.get_monitor(Monitor::RENDER_BUFFER_MEM_USED) as f64
                / 1_048_576.0,

            #[cfg(feature = "use_memory_debugger")]
            rust_heap_usage_mb: {
                let allocated = ALLOCATED.load(Ordering::Relaxed);
                let deallocated = DEALLOCATED.load(Ordering::Relaxed);
                allocated.saturating_sub(deallocated) as f64 / 1_048_576.0
            },
            #[cfg(not(feature = "use_memory_debugger"))]
            rust_heap_usage_mb: 0.0,

            #[cfg(feature = "use_memory_debugger")]
            rust_total_allocated_mb: ALLOCATED.load(Ordering::Relaxed) as f64 / 1_048_576.0,
            #[cfg(not(feature = "use_memory_debugger"))]
            rust_total_allocated_mb: 0.0,

            deno_total_memory_mb: self.get_deno_metrics().0,
            deno_scene_count: self.get_deno_metrics().1,
            deno_average_memory_mb: self.get_deno_metrics().2,

            // Object counts
            total_objects: performance.get_monitor(Monitor::OBJECT_COUNT) as i64,
            resource_count: performance.get_monitor(Monitor::OBJECT_RESOURCE_COUNT) as i64,
            node_count: performance.get_monitor(Monitor::OBJECT_NODE_COUNT) as i64,
            orphan_node_count: performance.get_monitor(Monitor::OBJECT_ORPHAN_NODE_COUNT) as i64,

            // Rendering
            fps: performance.get_monitor(Monitor::TIME_FPS) as f64,
            draw_calls: performance.get_monitor(Monitor::RENDER_TOTAL_DRAW_CALLS_IN_FRAME) as i64,
            primitives_in_frame: performance.get_monitor(Monitor::RENDER_TOTAL_PRIMITIVES_IN_FRAME)
                as i64,
            objects_in_frame: performance.get_monitor(Monitor::RENDER_TOTAL_OBJECTS_IN_FRAME)
                as i64,

            // Resource analysis from GDScript
            total_meshes: resource_data
                .get("total_meshes")
                .map(|v| v.to::<i32>())
                .unwrap_or(0),
            total_materials: resource_data
                .get("total_materials")
                .map(|v| v.to::<i32>())
                .unwrap_or(0),
            mesh_rid_count: resource_data
                .get("mesh_rid_count")
                .map(|v| v.to::<i32>())
                .unwrap_or(0),
            material_rid_count: resource_data
                .get("material_rid_count")
                .map(|v| v.to::<i32>())
                .unwrap_or(0),
            mesh_hash_count: resource_data
                .get("mesh_hash_count")
                .map(|v| v.to::<i32>())
                .unwrap_or(0),
            potential_dedup_count: resource_data
                .get("potential_dedup_count")
                .map(|v| v.to::<i32>())
                .unwrap_or(0),
            mesh_savings_percent: resource_data
                .get("mesh_savings_percent")
                .map(|v| v.to::<f64>())
                .unwrap_or(0.0),

            // Mobile metrics
            mobile_memory_usage_mb: self.get_mobile_metrics().0,
            mobile_temperature_celsius: self.get_mobile_metrics().1,
            mobile_battery_percent: self.get_mobile_metrics().2,

            // Process info
            process_memory_usage_mb: self.get_process_memory_usage_mb(),
        };

        self.metrics_history.push(metrics);
        tracing::info!("Stored metrics for test: {}", test_name);
    }

    /// Generate individual markdown report for the most recent metrics
    #[func]
    pub fn generate_individual_report(&self) -> GString {
        if let Some(metrics) = self.metrics_history.last() {
            let markdown = self.format_individual_report(metrics);

            // Save to file
            if let Err(e) = self.save_report(&metrics.test_name, &metrics.timestamp, &markdown) {
                tracing::error!("Failed to save individual report: {:?}", e);
            }

            GString::from(markdown)
        } else {
            GString::from("No metrics collected yet")
        }
    }

    /// Generate comprehensive summary report comparing all tests
    #[func]
    pub fn generate_summary_report(&self) -> GString {
        if self.metrics_history.is_empty() {
            return GString::from("No metrics collected yet");
        }

        let markdown = self.format_summary_report();

        // Save to file
        let timestamp = self.get_timestamp();
        if let Err(e) = self.save_report("summary", &timestamp, &markdown) {
            tracing::error!("Failed to save summary report: {:?}", e);
        }

        GString::from(markdown)
    }

    /// Generate single consolidated report with all test results
    #[func]
    pub fn generate_consolidated_report(&self) -> GString {
        if self.metrics_history.is_empty() {
            return GString::from("No metrics collected yet");
        }

        let markdown = self.format_consolidated_report();

        // Save to file with fixed name (no timestamp)
        if let Err(e) = self.save_consolidated_report("benchmark_report", &markdown) {
            tracing::error!("Failed to save consolidated report: {:?}", e);
        }

        GString::from(markdown)
    }

    /// Clear all stored metrics
    #[func]
    pub fn clear_metrics(&mut self) {
        self.metrics_history.clear();
        tracing::info!("Cleared all benchmark metrics");
    }

    // Helper methods

    fn get_timestamp(&self) -> String {
        let time_dict = godot::engine::Time::singleton().get_datetime_dict_from_system();
        format!(
            "{:04}-{:02}-{:02}_{:02}-{:02}-{:02}",
            time_dict.get("year").unwrap().to::<i32>(),
            time_dict.get("month").unwrap().to::<i32>(),
            time_dict.get("day").unwrap().to::<i32>(),
            time_dict.get("hour").unwrap().to::<i32>(),
            time_dict.get("minute").unwrap().to::<i32>(),
            time_dict.get("second").unwrap().to::<i32>()
        )
    }

    #[cfg(feature = "use_deno")]
    fn get_deno_metrics(&self) -> (f64, i32, f64) {
        use crate::scene_runner::scene_manager::SceneManager;

        if let Some(scene_manager_node) = &self.scene_manager_path {
            if let Ok(scene_manager) = scene_manager_node.clone().try_cast::<SceneManager>() {
                let scene_manager = scene_manager.bind();
                return (
                    scene_manager.get_total_deno_memory_mb(),
                    scene_manager.get_deno_scene_count(),
                    scene_manager.get_average_deno_memory_mb(),
                );
            }
        }
        (0.0, 0, 0.0)
    }

    #[cfg(not(feature = "use_deno"))]
    fn get_deno_metrics(&self) -> (f64, i32, f64) {
        (0.0, 0, 0.0)
    }

    fn get_mobile_metrics(&self) -> (Option<i32>, Option<f32>, Option<i32>) {
        let metrics_data = if DclIosPlugin::is_available() {
            DclIosPlugin::get_mobile_metrics_internal()
        } else if DclGodotAndroidPlugin::is_available() {
            DclGodotAndroidPlugin::get_mobile_metrics_internal()
        } else {
            None
        };

        if let Some(metrics) = metrics_data {
            (
                Some(metrics.memory_usage),
                Some(metrics.device_temperature_celsius),
                Some(metrics.battery_percent.round() as i32),
            )
        } else {
            (None, None, None)
        }
    }

    fn get_process_memory_usage_mb(&self) -> f64 {
        // Get process memory from Linux /proc/self/status
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        // VmRSS:    1234567 kB (Resident Set Size - actual physical memory used)
                        if let Some(value_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = value_str.parse::<f64>() {
                                return kb / 1024.0; // Convert kB to MiB
                            }
                        }
                    }
                }
            }
        }

        // Return 0.0 if not available
        0.0
    }

    fn format_individual_report(&self, metrics: &BenchmarkMetrics) -> String {
        let mut report = String::new();

        report.push_str(&format!("# Benchmark Report: {}\n\n", metrics.test_name));
        report.push_str(&format!("**Timestamp**: {}\n\n", metrics.timestamp));
        report.push_str(&format!("**Location**: {}\n\n", metrics.location));
        if !metrics.realm.is_empty() {
            report.push_str(&format!("**Realm**: {}\n\n", metrics.realm));
        }

        report.push_str("---\n\n");

        // Memory Metrics
        report.push_str("## Memory Metrics\n\n");
        report.push_str("| Metric | Value |\n");
        report.push_str("|--------|-------|\n");
        report.push_str(&format!(
            "| **Process Memory Usage (RSS)** | **{:.2} MiB ({:.2} GiB)** |\n",
            metrics.process_memory_usage_mb,
            metrics.process_memory_usage_mb / 1024.0
        ));
        report.push_str(&format!(
            "| Godot Static Memory | {:.2} MiB |\n",
            metrics.godot_static_memory_mb
        ));
        report.push_str(&format!(
            "| Godot Peak Memory | {:.2} MiB |\n",
            metrics.godot_static_memory_peak_mb
        ));
        report.push_str(&format!(
            "| GPU Video RAM | {:.2} MiB |\n",
            metrics.gpu_video_ram_mb
        ));
        report.push_str(&format!(
            "| GPU Texture Memory | {:.2} MiB |\n",
            metrics.gpu_texture_memory_mb
        ));
        report.push_str(&format!(
            "| GPU Buffer Memory | {:.2} MiB |\n",
            metrics.gpu_buffer_memory_mb
        ));
        report.push_str(&format!(
            "| Rust Heap Usage | {:.2} MiB |\n",
            metrics.rust_heap_usage_mb
        ));
        report.push_str(&format!(
            "| Rust Total Allocated | {:.2} MiB |\n",
            metrics.rust_total_allocated_mb
        ));
        if metrics.deno_scene_count > 0 {
            report.push_str(&format!(
                "| Deno/V8 Total Memory | {:.2} MiB |\n",
                metrics.deno_total_memory_mb
            ));
            report.push_str(&format!(
                "| Deno Active Scenes | {} |\n",
                metrics.deno_scene_count
            ));
            report.push_str(&format!(
                "| Deno Avg per Scene | {:.2} MiB |\n",
                metrics.deno_average_memory_mb
            ));
        }
        report.push_str("\n");

        // Object Counts
        report.push_str("## Object Counts\n\n");
        report.push_str("| Metric | Count |\n");
        report.push_str("|--------|-------|\n");
        report.push_str(&format!("| Total Objects | {} |\n", metrics.total_objects));
        report.push_str(&format!("| Resources | {} |\n", metrics.resource_count));
        report.push_str(&format!("| Nodes | {} |\n", metrics.node_count));
        report.push_str(&format!(
            "| Orphan Nodes | {} |\n",
            metrics.orphan_node_count
        ));
        report.push_str("\n");

        // Rendering
        report.push_str("## Rendering Metrics\n\n");
        report.push_str("| Metric | Value |\n");
        report.push_str("|--------|-------|\n");
        report.push_str(&format!("| FPS | {:.1} |\n", metrics.fps));
        report.push_str(&format!(
            "| Draw Calls per Frame | {} |\n",
            metrics.draw_calls
        ));
        report.push_str(&format!(
            "| Primitives per Frame | {} |\n",
            metrics.primitives_in_frame
        ));
        report.push_str(&format!(
            "| Objects per Frame | {} |\n",
            metrics.objects_in_frame
        ));
        report.push_str("\n");

        // Resource Analysis
        if metrics.total_meshes > 0 {
            report.push_str("## Resource Analysis\n\n");
            report.push_str("| Metric | Value |\n");
            report.push_str("|--------|-------|\n");
            report.push_str(&format!(
                "| Total Mesh References | {} |\n",
                metrics.total_meshes
            ));
            report.push_str(&format!(
                "| Total Material References | {} |\n",
                metrics.total_materials
            ));
            report.push_str(&format!(
                "| Unique Mesh RIDs | {} |\n",
                metrics.mesh_rid_count
            ));
            report.push_str(&format!(
                "| Unique Material RIDs | {} |\n",
                metrics.material_rid_count
            ));
            report.push_str(&format!(
                "| Hashed Mesh Count | {} |\n",
                metrics.mesh_hash_count
            ));
            report.push_str(&format!(
                "| Potential Deduplication | {} ({:.1}% savings) |\n",
                metrics.potential_dedup_count, metrics.mesh_savings_percent
            ));
            report.push_str("\n");
        }

        // Mobile Metrics
        if metrics.mobile_memory_usage_mb.is_some() {
            report.push_str("## Mobile Metrics\n\n");
            report.push_str("| Metric | Value |\n");
            report.push_str("|--------|-------|\n");
            if let Some(mem) = metrics.mobile_memory_usage_mb {
                report.push_str(&format!("| Memory Usage | {} MiB |\n", mem));
            }
            if let Some(temp) = metrics.mobile_temperature_celsius {
                report.push_str(&format!("| Temperature | {:.1}°C |\n", temp));
            }
            if let Some(battery) = metrics.mobile_battery_percent {
                report.push_str(&format!("| Battery | {}% |\n", battery));
            }
            report.push_str("\n");
        }

        report
    }

    fn format_summary_report(&self) -> String {
        let mut report = String::new();

        report.push_str("# Benchmark Summary Report\n\n");
        report.push_str(&format!("**Generated**: {}\n\n", self.get_timestamp()));
        report.push_str(&format!(
            "**Total Tests**: {}\n\n",
            self.metrics_history.len()
        ));
        report.push_str("---\n\n");

        // Memory comparison table
        report.push_str("## Memory Usage Comparison\n\n");
        report.push_str(
            "| Test | Godot (MiB) | GPU (MiB) | Rust (MiB) | Deno (MiB) | Total (MiB) |\n",
        );
        report.push_str(
            "|------|-------------|-----------|------------|------------|-------------|\n",
        );
        for metrics in &self.metrics_history {
            let total = metrics.godot_static_memory_mb
                + metrics.gpu_video_ram_mb
                + metrics.rust_heap_usage_mb
                + metrics.deno_total_memory_mb;
            report.push_str(&format!(
                "| {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} |\n",
                metrics.test_name,
                metrics.godot_static_memory_mb,
                metrics.gpu_video_ram_mb,
                metrics.rust_heap_usage_mb,
                metrics.deno_total_memory_mb,
                total
            ));
        }
        report.push_str("\n");

        // Performance comparison table
        report.push_str("## Performance Comparison\n\n");
        report.push_str("| Test | FPS | Draw Calls | Primitives | Objects |\n");
        report.push_str("|------|-----|------------|------------|---------|\n");
        for metrics in &self.metrics_history {
            report.push_str(&format!(
                "| {} | {:.1} | {} | {} | {} |\n",
                metrics.test_name,
                metrics.fps,
                metrics.draw_calls,
                metrics.primitives_in_frame,
                metrics.objects_in_frame
            ));
        }
        report.push_str("\n");

        // Resource analysis comparison (if available)
        if self.metrics_history.iter().any(|m| m.total_meshes > 0) {
            report.push_str("## Resource Analysis Comparison\n\n");
            report.push_str("| Test | Meshes | Materials | Unique Meshes | Dedup Potential |\n");
            report.push_str("|------|--------|-----------|---------------|------------------|\n");
            for metrics in &self.metrics_history {
                if metrics.total_meshes > 0 {
                    report.push_str(&format!(
                        "| {} | {} | {} | {} | {} ({:.1}%) |\n",
                        metrics.test_name,
                        metrics.total_meshes,
                        metrics.total_materials,
                        metrics.mesh_rid_count,
                        metrics.potential_dedup_count,
                        metrics.mesh_savings_percent
                    ));
                }
            }
            report.push_str("\n");
        }

        // Object counts comparison
        report.push_str("## Object Counts Comparison\n\n");
        report.push_str("| Test | Total Objects | Nodes | Resources | Orphan Nodes |\n");
        report.push_str("|------|---------------|-------|-----------|---------------|\n");
        for metrics in &self.metrics_history {
            report.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                metrics.test_name,
                metrics.total_objects,
                metrics.node_count,
                metrics.resource_count,
                metrics.orphan_node_count
            ));
        }
        report.push_str("\n");

        // Individual test details
        report.push_str("---\n\n");
        report.push_str("## Individual Test Details\n\n");
        for metrics in &self.metrics_history {
            report.push_str(&self.format_individual_report(metrics));
            report.push_str("\n---\n\n");
        }

        report
    }

    fn format_consolidated_report(&self) -> String {
        let mut report = String::new();

        report.push_str("# Decentraland Godot Explorer - Benchmark Report\n\n");
        report.push_str(&format!("**Generated**: {}\n\n", self.get_timestamp()));
        report.push_str(&format!(
            "**Total Tests**: {}\n\n",
            self.metrics_history.len()
        ));

        // Add process info
        if let Some(first_metric) = self.metrics_history.first() {
            report.push_str(&format!(
                "**Process Memory Usage (RSS)**: {:.2} MiB ({:.2} GiB)\n\n",
                first_metric.process_memory_usage_mb,
                first_metric.process_memory_usage_mb / 1024.0
            ));
        }

        report.push_str("---\n\n");

        // Table of Contents
        report.push_str("## Table of Contents\n\n");
        for (i, metrics) in self.metrics_history.iter().enumerate() {
            report.push_str(&format!(
                "{}. [{}](#test-{}-{})\n",
                i + 1,
                metrics.test_name,
                i + 1,
                metrics
                    .test_name
                    .to_lowercase()
                    .replace(' ', "-")
                    .replace('_', "-")
            ));
        }
        report.push_str("\n---\n\n");

        // Summary Tables
        report.push_str("## Summary Overview\n\n");

        // Memory Summary
        report.push_str("### Memory Metrics\n\n");
        report.push_str(
            "| Test | Godot Static (MiB) | GPU VRAM (MiB) | Rust Heap (MiB) | Deno Total (MiB) |\n",
        );
        report.push_str(
            "|------|-------------------|----------------|-----------------|------------------|\n",
        );
        for metrics in &self.metrics_history {
            report.push_str(&format!(
                "| {} | {:.2} | {:.2} | {:.2} | {:.2} |\n",
                metrics.test_name,
                metrics.godot_static_memory_mb,
                metrics.gpu_video_ram_mb,
                metrics.rust_heap_usage_mb,
                metrics.deno_total_memory_mb
            ));
        }
        report.push_str("\n");

        // Objects Summary
        report.push_str("### Object Counts\n\n");
        report.push_str("| Test | Total Objects | Nodes | Resources | Orphan Nodes |\n");
        report.push_str("|------|---------------|-------|-----------|---------------|\n");
        for metrics in &self.metrics_history {
            report.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                metrics.test_name,
                metrics.total_objects,
                metrics.node_count,
                metrics.resource_count,
                metrics.orphan_node_count
            ));
        }
        report.push_str("\n");

        // Rendering Summary
        report.push_str("### Rendering Metrics\n\n");
        report.push_str("| Test | FPS | Draw Calls | Primitives | Objects in Frame |\n");
        report.push_str("|------|-----|------------|------------|------------------|\n");
        for metrics in &self.metrics_history {
            report.push_str(&format!(
                "| {} | {:.1} | {} | {} | {} |\n",
                metrics.test_name,
                metrics.fps,
                metrics.draw_calls,
                metrics.primitives_in_frame,
                metrics.objects_in_frame
            ));
        }
        report.push_str("\n");

        // Resource Analysis Summary
        report.push_str("### Resource Analysis\n\n");
        report.push_str(
            "| Test | Meshes | Materials | Mesh RIDs | Material RIDs | Dedup Potential |\n",
        );
        report.push_str(
            "|------|--------|-----------|-----------|---------------|------------------|\n",
        );
        for metrics in &self.metrics_history {
            report.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                metrics.test_name,
                metrics.total_meshes,
                metrics.total_materials,
                metrics.mesh_rid_count,
                metrics.material_rid_count,
                metrics.potential_dedup_count
            ));
        }
        report.push_str("\n---\n\n");

        // Detailed Results for Each Test
        report.push_str("## Detailed Test Results\n\n");
        for (i, metrics) in self.metrics_history.iter().enumerate() {
            report.push_str(&format!("### Test {}: {}\n\n", i + 1, metrics.test_name));
            report.push_str(&self.format_individual_report(metrics));
            report.push_str("\n---\n\n");
        }

        report
    }

    fn save_report(&self, test_name: &str, timestamp: &str, content: &str) -> std::io::Result<()> {
        // Get the user data directory from Godot
        let user_dir = Os::singleton().get_user_data_dir().to_string();
        let output_dir = PathBuf::from(user_dir).join("output").join("reports");

        // Create directory if it doesn't exist
        fs::create_dir_all(&output_dir)?;

        // Create filename
        let filename = format!("{}_{}.md", test_name.replace(' ', "_"), timestamp);
        let filepath = output_dir.join(filename);

        // Write file
        let mut file = File::create(&filepath)?;
        file.write_all(content.as_bytes())?;

        tracing::info!("Saved report to: {:?}", filepath);
        godot_print!("✓ Benchmark report saved: {:?}", filepath);

        Ok(())
    }

    fn save_consolidated_report(&self, filename: &str, content: &str) -> std::io::Result<()> {
        // Get the user data directory from Godot
        let user_dir = Os::singleton().get_user_data_dir().to_string();
        let output_dir = PathBuf::from(user_dir).join("output");

        // Create directory if it doesn't exist
        fs::create_dir_all(&output_dir)?;

        // Create filename without timestamp
        let filename = format!("{}.md", filename);
        let filepath = output_dir.join(filename);

        // Write file
        let mut file = File::create(&filepath)?;
        file.write_all(content.as_bytes())?;

        tracing::info!("Saved consolidated report to: {:?}", filepath);
        godot_print!("✓ Consolidated benchmark report saved: {:?}", filepath);

        Ok(())
    }
}
