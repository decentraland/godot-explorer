use godot::{engine::Os, prelude::*};

use crate::godot_classes::{
    dcl_android_plugin::DclGodotAndroidPlugin, dcl_ios_plugin::DclIosPlugin,
};

#[derive(GodotClass)]
#[class(base=Node)]
pub struct PerformanceDebugger {
    base: Base<Node>,
    metrics_print_timer: f64,
    is_enabled: bool,
}

#[godot_api]
impl INode for PerformanceDebugger {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            metrics_print_timer: 0.0,
            is_enabled: false, // Will be set in ready()
        }
    }

    fn ready(&mut self) {
        // Check if this is a Godot debug build
        let is_debug_build = Os::singleton().is_debug_build();
        self.is_enabled = is_debug_build;

        if self.is_enabled {
            tracing::info!("PerformanceDebugger enabled (Godot debug export)");
        } else {
            tracing::info!("PerformanceDebugger disabled (Godot release export)");
        }
    }

    fn process(&mut self, delta: f64) {
        if !self.is_enabled {
            return;
        }

        self.metrics_print_timer += delta;
        if self.metrics_print_timer >= 1.0 {
            self.metrics_print_timer = 0.0;
            self.print_mobile_metrics();
        }
    }
}

#[godot_api]
impl PerformanceDebugger {
    #[func]
    pub fn set_enabled(&mut self, enabled: bool) {
        self.is_enabled = enabled;
    }

    #[func]
    pub fn is_enabled(&self) -> bool {
        self.is_enabled
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
            godot_print!("=== Mobile Metrics ===");
            godot_print!("  Memory Usage: {} MB", metrics.memory_usage);
            godot_print!(
                "  Device Temperature: {}°C",
                metrics.device_temperature_celsius
            );
            godot_print!("  Device Thermal State: {}", metrics.device_thermal_state);
            godot_print!("  Battery Percent: {}%", metrics.battery_percent);
            godot_print!("  Charging State: {}", metrics.charging_state);
            godot_print!("======================");
        } else {
            godot_print!("Mobile metrics data not available");
        }
    }
}
