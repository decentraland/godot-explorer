use godot::{engine::{Os, Performance, performance::Monitor}, prelude::*};

use crate::godot_classes::{
    dcl_android_plugin::DclGodotAndroidPlugin, dcl_ios_plugin::DclIosPlugin,
};

#[cfg(feature = "use_memory_debugger")]
use std::alloc::{GlobalAlloc, Layout, System};

#[cfg(feature = "use_memory_debugger")]
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Tracking Allocator - Live Rust Heap Memory Monitoring
// ============================================================================

#[cfg(feature = "use_memory_debugger")]
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "use_memory_debugger")]
static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "use_memory_debugger")]
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "use_memory_debugger")]
static DEALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "use_memory_debugger")]
pub struct TrackingAllocator;

#[cfg(feature = "use_memory_debugger")]
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
        self.print_gpu_memory_metrics();
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

    #[cfg(feature = "use_memory_debugger")]
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
}
