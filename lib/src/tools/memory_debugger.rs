use godot::{engine::{Os, Performance, performance::Monitor}, prelude::*};

use crate::godot_classes::{
    dcl_android_plugin::DclGodotAndroidPlugin, dcl_ios_plugin::DclIosPlugin,
};

#[cfg(feature = "use_memory_debugger")]
use std::sync::Mutex;

#[cfg(feature = "use_memory_debugger")]
static PROFILER: Mutex<Option<dhat::Profiler>> = Mutex::new(None);

#[derive(GodotClass)]
#[class(base=Node)]
pub struct MemoryDebugger {
    base: Base<Node>,
    metrics_print_timer: f64,
    is_enabled: bool,
    print_interval: f64,
}

#[godot_api]
impl INode for MemoryDebugger {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            metrics_print_timer: 0.0,
            is_enabled: false, // Will be set in ready()
            print_interval: 1.0, // Print metrics every second by default
        }
    }

    fn ready(&mut self) {
        // Check if this is a Godot debug build
        let is_debug_build = Os::singleton().is_debug_build();
        self.is_enabled = is_debug_build;

        if self.is_enabled {
            tracing::info!("MemoryDebugger enabled (Godot debug export)");

            #[cfg(feature = "use_memory_debugger")]
            {
                self.start_heap_profiling();
            }
        } else {
            tracing::info!("MemoryDebugger disabled (Godot release export)");
        }
    }

    fn process(&mut self, delta: f64) {
        if !self.is_enabled {
            return;
        }

        self.metrics_print_timer += delta;
        if self.metrics_print_timer >= self.print_interval {
            self.metrics_print_timer = 0.0;
            self.print_all_metrics();
        }
    }
}

#[godot_api]
impl MemoryDebugger {
    #[func]
    pub fn set_enabled(&mut self, enabled: bool) {
        self.is_enabled = enabled;

        #[cfg(feature = "use_memory_debugger")]
        {
            if enabled {
                self.start_heap_profiling();
            } else {
                self.stop_heap_profiling();
            }
        }
    }

    #[func]
    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    #[func]
    pub fn set_print_interval(&mut self, interval: f64) {
        self.print_interval = interval.max(0.1); // Minimum 0.1 seconds
    }

    #[func]
    pub fn get_print_interval(&self) -> f64 {
        self.print_interval
    }

    fn print_all_metrics(&self) {
        godot_print!("╔══════════════════════════════════════════════════════════════╗");
        godot_print!("║                      MEMORY DEBUGGER                         ║");
        godot_print!("╚══════════════════════════════════════════════════════════════╝");

        self.print_godot_memory_metrics();
        self.print_godot_object_metrics();
        self.print_godot_render_metrics();
        self.print_mobile_metrics();

        #[cfg(feature = "use_memory_debugger")]
        {
            self.print_rust_heap_info();
        }

        godot_print!("════════════════════════════════════════════════════════════════");
    }

    fn print_godot_memory_metrics(&self) {
        let performance = Performance::singleton();

        // Memory metrics (in MB)
        let static_memory = performance.get_monitor(Monitor::MEMORY_STATIC) as f64 / 1_048_576.0;
        let static_memory_max = performance.get_monitor(Monitor::MEMORY_STATIC_MAX) as f64 / 1_048_576.0;

        godot_print!("┌─ Godot Memory ─────────────────────────────────────────────┐");
        godot_print!("│  Static Memory:     {:.2} MB", static_memory);
        godot_print!("│  Peak Static:       {:.2} MB", static_memory_max);
        godot_print!("└────────────────────────────────────────────────────────────┘");
    }

    fn print_godot_object_metrics(&self) {
        let performance = Performance::singleton();

        let object_count = performance.get_monitor(Monitor::OBJECT_COUNT) as i64;
        let resource_count = performance.get_monitor(Monitor::OBJECT_RESOURCE_COUNT) as i64;
        let node_count = performance.get_monitor(Monitor::OBJECT_NODE_COUNT) as i64;
        let orphan_node_count = performance.get_monitor(Monitor::OBJECT_ORPHAN_NODE_COUNT) as i64;

        godot_print!("┌─ Godot Objects ────────────────────────────────────────────┐");
        godot_print!("│  Total Objects:     {}", object_count);
        godot_print!("│  Resources:         {}", resource_count);
        godot_print!("│  Nodes:             {}", node_count);
        godot_print!("│  Orphan Nodes:      {}", orphan_node_count);
        godot_print!("└────────────────────────────────────────────────────────────┘");
    }

    fn print_godot_render_metrics(&self) {
        let performance = Performance::singleton();

        let objects_in_frame = performance.get_monitor(Monitor::RENDER_TOTAL_OBJECTS_IN_FRAME) as i64;
        let primitives_in_frame = performance.get_monitor(Monitor::RENDER_TOTAL_PRIMITIVES_IN_FRAME) as i64;
        let draw_calls = performance.get_monitor(Monitor::RENDER_TOTAL_DRAW_CALLS_IN_FRAME) as i64;

        godot_print!("┌─ Rendering ────────────────────────────────────────────────┐");
        godot_print!("│  Objects/Frame:     {}", objects_in_frame);
        godot_print!("│  Primitives/Frame:  {}", primitives_in_frame);
        godot_print!("│  Draw Calls/Frame:  {}", draw_calls);
        godot_print!("└────────────────────────────────────────────────────────────┘");
    }

    fn print_mobile_metrics(&self) {
        // Get metrics from the appropriate mobile plugin
        let metrics_data = if DclIosPlugin::is_available() {
            DclIosPlugin::get_mobile_metrics_internal()
        } else if DclGodotAndroidPlugin::is_available() {
            DclGodotAndroidPlugin::get_mobile_metrics_internal()
        } else {
            None
        };

        if let Some(metrics) = metrics_data {
            godot_print!("┌─ Mobile Metrics ───────────────────────────────────────────┐");
            godot_print!("│  Memory Usage:      {} MB", metrics.memory_usage);
            godot_print!("│  Temperature:       {}°C", metrics.device_temperature_celsius);
            godot_print!("│  Thermal State:     {}", metrics.device_thermal_state);
            godot_print!("│  Battery:           {}%", metrics.battery_percent);
            godot_print!("│  Charging:          {}", metrics.charging_state);
            godot_print!("└────────────────────────────────────────────────────────────┘");
        }
    }

    #[cfg(feature = "use_memory_debugger")]
    fn print_rust_heap_info(&self) {
        godot_print!("┌─ Rust Heap Profiling ──────────────────────────────────────┐");
        godot_print!("│  dhat profiling is ACTIVE");
        godot_print!("│  Call stop_heap_profiling() to generate profile");
        godot_print!("└────────────────────────────────────────────────────────────┘");
    }

    #[cfg(feature = "use_memory_debugger")]
    #[func]
    pub fn start_heap_profiling(&mut self) {
        let mut profiler = PROFILER.lock().unwrap();
        if profiler.is_none() {
            *profiler = Some(dhat::Profiler::new_heap());
            tracing::info!("Started dhat heap profiling");
            godot_print!("✓ Started Rust heap profiling with dhat");
        } else {
            tracing::warn!("Heap profiling already active");
        }
    }

    #[cfg(feature = "use_memory_debugger")]
    #[func]
    pub fn stop_heap_profiling(&mut self) {
        let mut profiler = PROFILER.lock().unwrap();
        if let Some(p) = profiler.take() {
            drop(p);
            tracing::info!("Stopped dhat heap profiling - profile written to dhat-heap.json");
            godot_print!("✓ Stopped Rust heap profiling - output written to dhat-heap.json");
        } else {
            tracing::warn!("No active heap profiling to stop");
        }
    }

    #[cfg(feature = "use_memory_debugger")]
    #[func]
    pub fn is_heap_profiling_active(&self) -> bool {
        PROFILER.lock().unwrap().is_some()
    }

    // Stub functions when feature is disabled
    #[cfg(not(feature = "use_memory_debugger"))]
    #[func]
    pub fn start_heap_profiling(&mut self) {
        tracing::warn!("Heap profiling not available - rebuild with --features use_memory_debugger");
        godot_print!("⚠ Heap profiling not available - rebuild with --features use_memory_debugger");
    }

    #[cfg(not(feature = "use_memory_debugger"))]
    #[func]
    pub fn stop_heap_profiling(&mut self) {
        tracing::warn!("Heap profiling not available - rebuild with --features use_memory_debugger");
    }

    #[cfg(not(feature = "use_memory_debugger"))]
    #[func]
    pub fn is_heap_profiling_active(&self) -> bool {
        false
    }

    /// Get current Godot memory usage in MB
    #[func]
    pub fn get_godot_memory_mb(&self) -> f64 {
        let performance = Performance::singleton();
        performance.get_monitor(Monitor::MEMORY_STATIC) as f64 / 1_048_576.0
    }

    /// Get peak Godot memory usage in MB
    #[func]
    pub fn get_godot_memory_peak_mb(&self) -> f64 {
        let performance = Performance::singleton();
        performance.get_monitor(Monitor::MEMORY_STATIC_MAX) as f64 / 1_048_576.0
    }

    /// Get total object count
    #[func]
    pub fn get_object_count(&self) -> i64 {
        let performance = Performance::singleton();
        performance.get_monitor(Monitor::OBJECT_COUNT) as i64
    }

    /// Get orphan node count (potential memory leaks)
    #[func]
    pub fn get_orphan_node_count(&self) -> i64 {
        let performance = Performance::singleton();
        performance.get_monitor(Monitor::OBJECT_ORPHAN_NODE_COUNT) as i64
    }
}
