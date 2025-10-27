use godot::{engine::{Os, Performance, performance::Monitor}, prelude::*};

use crate::godot_classes::{
    dcl_android_plugin::DclGodotAndroidPlugin, dcl_ios_plugin::DclIosPlugin,
};

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Tracking Allocator - Live Rust Heap Memory Monitoring
// ============================================================================

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

pub struct TrackingAllocator;

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = System.alloc(layout);
        if !ret.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        DEALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        DEALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ret = System.alloc_zeroed(layout);
        if !ret.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        ret
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let ret = System.realloc(ptr, layout, new_size);
        if !ret.is_null() {
            DEALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
            ALLOCATED.fetch_add(new_size, Ordering::Relaxed);
        }
        ret
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct MemoryDebugger {
    base: Base<Node>,
    metrics_print_timer: f64,
    is_enabled: bool,
    print_interval: f64,

    #[export]
    scene_manager_path: Option<Gd<Node>>,
}

#[godot_api]
impl INode for MemoryDebugger {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            metrics_print_timer: 0.0,
            is_enabled: false, // Will be set in ready()
            print_interval: 1.0, // Print metrics every second by default
            scene_manager_path: None,
        }
    }

    fn ready(&mut self) {
        // Check if this is a Godot debug build
        let is_debug_build = Os::singleton().is_debug_build();
        self.is_enabled = is_debug_build;

        if self.is_enabled {
            tracing::info!("MemoryDebugger enabled (Godot debug export)");
        } else {
            tracing::info!("MemoryDebugger disabled (Godot release export)");
        }

        // Automatically find and set the scene_manager_path if not already set
        if self.scene_manager_path.is_none() {
            if let Some(parent) = self.base().get_parent() {
                // Try to find scene_runner as a sibling (both are children of Global)
                let scene_runner_node = parent.get_node_or_null("scene_runner".into());
                if let Some(node) = scene_runner_node {
                    self.scene_manager_path = Some(node);
                    tracing::info!("MemoryDebugger: Automatically found scene_runner");
                } else {
                    tracing::warn!("MemoryDebugger: Could not find scene_runner node");
                }
            }
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
        godot_print!("║                      MEMORY DEBUGGER 3                       ║");
        godot_print!("╚══════════════════════════════════════════════════════════════╝");

        self.print_godot_memory_metrics();
        self.print_gpu_memory_metrics();
        self.print_godot_object_metrics();
        self.print_godot_render_metrics();
        self.print_mobile_metrics();
        self.print_rust_heap_info();
        self.print_deno_memory_metrics();
        self.print_mobile_memory_breakdown();

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

    fn print_gpu_memory_metrics(&self) {
        let performance = Performance::singleton();

        // GPU memory metrics (in MB)
        let video_mem = performance.get_monitor(Monitor::RENDER_VIDEO_MEM_USED) as f64 / 1_048_576.0;
        let texture_mem = performance.get_monitor(Monitor::RENDER_TEXTURE_MEM_USED) as f64 / 1_048_576.0;
        let buffer_mem = performance.get_monitor(Monitor::RENDER_BUFFER_MEM_USED) as f64 / 1_048_576.0;

        godot_print!("┌─ GPU Memory ───────────────────────────────────────────────┐");
        godot_print!("│  Video RAM:         {:.2} MB", video_mem);
        godot_print!("│  Texture Memory:    {:.2} MB", texture_mem);
        godot_print!("│  Buffer Memory:     {:.2} MB", buffer_mem);
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

    #[cfg(feature = "use_deno")]
    fn print_deno_memory_metrics(&self) {
        use crate::scene_runner::scene_manager::SceneManager;

        godot_print!("┌─ Deno/V8 Memory (JS Runtimes) ─────────────────────────────┐");

        if let Some(scene_manager_node) = &self.scene_manager_path {
            if let Ok(scene_manager) = scene_manager_node.clone().try_cast::<SceneManager>() {
                let scene_manager = scene_manager.bind();
                let total_used_mb = scene_manager.get_total_deno_memory_mb();
                let total_heap_mb = scene_manager.get_total_deno_heap_size_mb();
                let scene_count = scene_manager.get_deno_scene_count();
                let average_mb = scene_manager.get_average_deno_memory_mb();

                if scene_count > 0 {
                    godot_print!("│  Active Scenes:     {}", scene_count);
                    godot_print!("│  Total Used:        {:.2} MB", total_used_mb);
                    godot_print!("│  Total Heap Size:   {:.2} MB", total_heap_mb);
                    godot_print!("│  Average/Scene:     {:.2} MB", average_mb);
                } else {
                    godot_print!("│  No active scenes");
                }
            } else {
                godot_print!("│  ⚠ Failed to cast scene_manager_path to SceneManager");
            }
        } else {
            godot_print!("│  ⚠ Scene manager path not set");
            godot_print!("│  Set the 'scene_manager_path' property to enable tracking");
        }

        godot_print!("└────────────────────────────────────────────────────────────┘");
    }

    #[cfg(not(feature = "use_deno"))]
    fn print_deno_memory_metrics(&self) {
        godot_print!("┌─ Deno/V8 Memory (JS Runtimes) ─────────────────────────────┐");
        godot_print!("│  Deno feature not enabled");
        godot_print!("│  Rebuild with --features use_deno to enable");
        godot_print!("└────────────────────────────────────────────────────────────┘");
    }

    fn print_rust_heap_info(&self) {
        let allocated = ALLOCATED.load(Ordering::Relaxed);
        let deallocated = DEALLOCATED.load(Ordering::Relaxed);
        let current_usage = allocated.saturating_sub(deallocated);
        let alloc_count = ALLOCATION_COUNT.load(Ordering::Relaxed);
        let dealloc_count = DEALLOCATION_COUNT.load(Ordering::Relaxed);

        let current_mb = current_usage as f64 / 1_048_576.0;
        let total_allocated_mb = allocated as f64 / 1_048_576.0;
        let total_deallocated_mb = deallocated as f64 / 1_048_576.0;

        godot_print!("┌─ Rust Heap (Live Tracking) ────────────────────────────────┐");
        godot_print!("│  Current Usage:     {:.2} MB ({} bytes)", current_mb, current_usage);
        godot_print!("│  Total Allocated:   {:.2} MB ({} bytes)", total_allocated_mb, allocated);
        godot_print!("│  Total Freed:       {:.2} MB ({} bytes)", total_deallocated_mb, deallocated);
        godot_print!("│  Allocations:       {}", alloc_count);
        godot_print!("│  Deallocations:     {}", dealloc_count);
        godot_print!("│  Active Allocs:     {}", alloc_count.saturating_sub(dealloc_count));
        godot_print!("└────────────────────────────────────────────────────────────┘");
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

    /// Get video RAM usage in MB
    #[func]
    pub fn get_video_mem_mb(&self) -> f64 {
        let performance = Performance::singleton();
        performance.get_monitor(Monitor::RENDER_VIDEO_MEM_USED) as f64 / 1_048_576.0
    }

    /// Get texture memory usage in MB
    #[func]
    pub fn get_texture_mem_mb(&self) -> f64 {
        let performance = Performance::singleton();
        performance.get_monitor(Monitor::RENDER_TEXTURE_MEM_USED) as f64 / 1_048_576.0
    }

    /// Get buffer memory usage in MB
    #[func]
    pub fn get_buffer_mem_mb(&self) -> f64 {
        let performance = Performance::singleton();
        performance.get_monitor(Monitor::RENDER_BUFFER_MEM_USED) as f64 / 1_048_576.0
    }

    /// Get current Rust heap memory usage in MB (live tracking)
    #[cfg(feature = "use_memory_debugger")]
    #[func]
    pub fn get_rust_heap_usage_mb(&self) -> f64 {
        let allocated = ALLOCATED.load(Ordering::Relaxed);
        let deallocated = DEALLOCATED.load(Ordering::Relaxed);
        let current_usage = allocated.saturating_sub(deallocated);
        current_usage as f64 / 1_048_576.0
    }

    /// Get total allocated Rust heap memory in MB
    #[cfg(feature = "use_memory_debugger")]
    #[func]
    pub fn get_rust_heap_total_allocated_mb(&self) -> f64 {
        let allocated = ALLOCATED.load(Ordering::Relaxed);
        allocated as f64 / 1_048_576.0
    }

    /// Get Rust allocation count
    #[cfg(feature = "use_memory_debugger")]
    #[func]
    pub fn get_rust_allocation_count(&self) -> i64 {
        ALLOCATION_COUNT.load(Ordering::Relaxed) as i64
    }

    /// Get Rust deallocation count
    #[cfg(feature = "use_memory_debugger")]
    #[func]
    pub fn get_rust_deallocation_count(&self) -> i64 {
        DEALLOCATION_COUNT.load(Ordering::Relaxed) as i64
    }

    /// Reset Rust heap statistics (useful for profiling specific sections)
    #[cfg(feature = "use_memory_debugger")]
    #[func]
    pub fn reset_rust_heap_stats(&self) {
        ALLOCATED.store(0, Ordering::Relaxed);
        DEALLOCATED.store(0, Ordering::Relaxed);
        ALLOCATION_COUNT.store(0, Ordering::Relaxed);
        DEALLOCATION_COUNT.store(0, Ordering::Relaxed);
        godot_print!("✓ Reset Rust heap statistics");
    }

    // Stub functions when feature is disabled
    #[cfg(not(feature = "use_memory_debugger"))]
    #[func]
    pub fn get_rust_heap_usage_mb(&self) -> f64 {
        0.0
    }

    #[cfg(not(feature = "use_memory_debugger"))]
    #[func]
    pub fn get_rust_heap_total_allocated_mb(&self) -> f64 {
        0.0
    }

    #[cfg(not(feature = "use_memory_debugger"))]
    #[func]
    pub fn get_rust_allocation_count(&self) -> i64 {
        0
    }

    #[cfg(not(feature = "use_memory_debugger"))]
    #[func]
    pub fn get_rust_deallocation_count(&self) -> i64 {
        0
    }

    #[cfg(not(feature = "use_memory_debugger"))]
    #[func]
    pub fn reset_rust_heap_stats(&self) {
        godot_print!("⚠ Rust heap tracking not available - rebuild with --features use_memory_debugger");
    }

    /// Get total Deno/V8 memory usage in MB (requires scene_manager_path to be set)
    #[cfg(feature = "use_deno")]
    #[func]
    pub fn get_deno_total_memory_mb(&self) -> f64 {
        use crate::scene_runner::scene_manager::SceneManager;

        if let Some(scene_manager_node) = &self.scene_manager_path {
            if let Ok(scene_manager) = scene_manager_node.clone().try_cast::<SceneManager>() {
                return scene_manager.bind().get_total_deno_memory_mb();
            }
        }
        0.0
    }

    /// Get count of active Deno runtimes
    #[cfg(feature = "use_deno")]
    #[func]
    pub fn get_deno_scene_count(&self) -> i32 {
        use crate::scene_runner::scene_manager::SceneManager;

        if let Some(scene_manager_node) = &self.scene_manager_path {
            if let Ok(scene_manager) = scene_manager_node.clone().try_cast::<SceneManager>() {
                return scene_manager.bind().get_deno_scene_count();
            }
        }
        0
    }

    /// Get average Deno memory per scene in MB
    #[cfg(feature = "use_deno")]
    #[func]
    pub fn get_deno_average_memory_mb(&self) -> f64 {
        use crate::scene_runner::scene_manager::SceneManager;

        if let Some(scene_manager_node) = &self.scene_manager_path {
            if let Ok(scene_manager) = scene_manager_node.clone().try_cast::<SceneManager>() {
                return scene_manager.bind().get_average_deno_memory_mb();
            }
        }
        0.0
    }

    // Stub functions when use_deno feature is disabled
    #[cfg(not(feature = "use_deno"))]
    #[func]
    pub fn get_deno_total_memory_mb(&self) -> f64 {
        0.0
    }

    #[cfg(not(feature = "use_deno"))]
    #[func]
    pub fn get_deno_scene_count(&self) -> i32 {
        0
    }

    #[cfg(not(feature = "use_deno"))]
    #[func]
    pub fn get_deno_average_memory_mb(&self) -> f64 {
        0.0
    }

    /// Print memory breakdown showing percentage of total memory used by each component
    fn print_mobile_memory_breakdown(&self) {
        // Get total memory from mobile metrics
        let total_memory_mb = if DclIosPlugin::is_available() {
            DclIosPlugin::get_mobile_metrics_internal()
                .map(|m| m.memory_usage as f64)
        } else if DclGodotAndroidPlugin::is_available() {
            DclGodotAndroidPlugin::get_mobile_metrics_internal()
                .map(|m| m.memory_usage as f64)
        } else {
            None
        };

        // Only show breakdown if we have total memory from mobile
        let Some(total_mb) = total_memory_mb else {
            return;
        };

        if total_mb <= 0.0 {
            return;
        }

        let performance = Performance::singleton();

        // Get component memory usage
        let video_mem_mb = performance.get_monitor(Monitor::RENDER_VIDEO_MEM_USED) as f64 / 1_048_576.0;
        let texture_mem_mb = performance.get_monitor(Monitor::RENDER_TEXTURE_MEM_USED) as f64 / 1_048_576.0;
        let buffer_mem_mb = performance.get_monitor(Monitor::RENDER_BUFFER_MEM_USED) as f64 / 1_048_576.0;
        let other_gpu_mb = (video_mem_mb - texture_mem_mb - buffer_mem_mb).max(0.0);
        let static_mem_mb = performance.get_monitor(Monitor::MEMORY_STATIC) as f64 / 1_048_576.0;

        // Rust heap memory
        let rust_heap_mb = {
            let allocated = ALLOCATED.load(Ordering::Relaxed);
            let deallocated = DEALLOCATED.load(Ordering::Relaxed);
            let current_usage = allocated.saturating_sub(deallocated);
            current_usage as f64 / 1_048_576.0
        };

        // Deno memory
        let deno_mem_mb = self.get_deno_total_memory_mb();

        // Calculate total tracked and unknown
        let total_tracked_mb = video_mem_mb + static_mem_mb + rust_heap_mb + deno_mem_mb;
        let unknown_mb = (total_mb - total_tracked_mb).max(0.0);

        // Calculate percentages
        let video_pct = (video_mem_mb / total_mb) * 100.0;
        let texture_pct = (texture_mem_mb / total_mb) * 100.0;
        let buffer_pct = (buffer_mem_mb / total_mb) * 100.0;
        let other_gpu_pct = (other_gpu_mb / total_mb) * 100.0;
        let static_pct = (static_mem_mb / total_mb) * 100.0;
        let rust_pct = (rust_heap_mb / total_mb) * 100.0;
        let deno_pct = (deno_mem_mb / total_mb) * 100.0;
        let unknown_pct = (unknown_mb / total_mb) * 100.0;

        godot_print!("┌─ Memory Breakdown (Mobile) ────────────────────────────────┐");
        godot_print!("│  Total Memory:      {:.2} MB", total_mb);
        godot_print!("│");
        godot_print!("│  Video RAM:         {:.1}% ({:.2} MB)", video_pct, video_mem_mb);
        godot_print!("│    ├─ Textures:     {:.1}% ({:.2} MB)", texture_pct, texture_mem_mb);
        godot_print!("│    ├─ Buffers:      {:.1}% ({:.2} MB)", buffer_pct, buffer_mem_mb);
        godot_print!("│    └─ Other GPU:    {:.1}% ({:.2} MB)", other_gpu_pct, other_gpu_mb);
        godot_print!("│  Static Memory:     {:.1}% ({:.2} MB)", static_pct, static_mem_mb);
        godot_print!("│  Rust Heap:         {:.1}% ({:.2} MB)", rust_pct, rust_heap_mb);
        godot_print!("│  Deno/V8:           {:.1}% ({:.2} MB)", deno_pct, deno_mem_mb);
        godot_print!("│  Unknown:           {:.1}% ({:.2} MB)", unknown_pct, unknown_mb);
        godot_print!("└────────────────────────────────────────────────────────────┘");
    }
}
