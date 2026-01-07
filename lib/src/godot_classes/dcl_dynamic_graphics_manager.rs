use godot::prelude::*;

use crate::godot_classes::{
    dcl_android_plugin::DclGodotAndroidPlugin, dcl_ios_plugin::DclIosPlugin,
};

/// Thermal state levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThermalState {
    Normal,
    High,
    Critical,
}

/// Manager state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagerState {
    Disabled,
    WarmingUp,
    Monitoring,
    Cooldown,
}

/// Timing constants (in seconds)
const WARMUP_DURATION: f64 = 180.0; // 3 minutes before monitoring
const DOWNGRADE_WINDOW: f64 = 60.0; // Window for downgrade evaluation
const UPGRADE_WINDOW: f64 = 120.0; // Window for upgrade evaluation
const COOLDOWN_DURATION: f64 = 300.0; // 5 minutes after profile change
const THERMAL_HIGH_DOWNGRADE_TIME: f64 = 30.0; // Seconds of HIGH thermal before downgrade
const SAMPLE_INTERVAL: f64 = 1.0; // Sample FPS every second

/// Threshold constants
const FPS_DOWNGRADE_THRESHOLD: f64 = 30.0;
const FPS_UPGRADE_THRESHOLD: f64 = 55.0;
const FRAME_SPIKE_THRESHOLD_MS: f64 = 50.0;
const FRAME_SPIKE_RATIO_THRESHOLD: f64 = 0.1; // 10% of frames with spikes triggers downgrade

/// Profile bounds (0=Low, 1=Medium, 2=High, 3=Custom is excluded)
const MIN_PROFILE: i32 = 0;
const MAX_PROFILE: i32 = 2;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclDynamicGraphicsManager {
    base: Base<Node>,

    /// Current state machine state
    state: ManagerState,

    /// Timer for current state
    state_timer: f64,

    /// FPS samples collected every second
    fps_samples: Vec<f64>,

    /// Timer for FPS sampling
    sample_timer: f64,

    /// Frame spike tracking
    spike_count: u32,
    frame_count: u32,

    /// Current thermal state
    thermal_state: ThermalState,

    /// Timer for high thermal state
    thermal_high_timer: f64,

    /// Current active profile
    current_profile: i32,

    /// Is gameplay active (not loading)
    is_gameplay_active: bool,

    /// Is dynamic adjustment enabled
    enabled: bool,

    /// Cached mobile platform availability (checked once at init)
    has_ios_plugin: bool,
    has_android_plugin: bool,
}

#[godot_api]
impl INode for DclDynamicGraphicsManager {
    fn init(base: Base<Node>) -> Self {
        // Check plugin availability once at init
        let has_ios_plugin = DclIosPlugin::is_available();
        let has_android_plugin = DclGodotAndroidPlugin::is_available();

        Self {
            base,
            state: ManagerState::Disabled,
            state_timer: 0.0,
            fps_samples: Vec::with_capacity(150),
            sample_timer: 0.0,
            spike_count: 0,
            frame_count: 0,
            thermal_state: ThermalState::Normal,
            thermal_high_timer: 0.0,
            current_profile: 0,
            is_gameplay_active: false,
            enabled: true,
            has_ios_plugin,
            has_android_plugin,
        }
    }

    fn ready(&mut self) {
        // Note: DclGlobal is not available yet during init, so we start disabled.
        // GDScript should call initialize() after Global is ready.
        tracing::info!(
            "DclDynamicGraphicsManager ready (iOS={}, Android={})",
            self.has_ios_plugin,
            self.has_android_plugin
        );
    }

    fn process(&mut self, delta: f64) {
        if self.state == ManagerState::Disabled {
            return;
        }

        // Track frame spikes
        let frame_time_ms = delta * 1000.0;
        self.frame_count += 1;
        if frame_time_ms > FRAME_SPIKE_THRESHOLD_MS {
            self.spike_count += 1;
        }

        // Sample FPS periodically
        self.sample_timer += delta;
        if self.sample_timer >= SAMPLE_INTERVAL {
            self.sample_timer = 0.0;
            self.collect_fps_sample();
        }

        // Update thermal state
        self.update_thermal_state(delta);

        // Process state machine
        match self.state {
            ManagerState::Disabled => {}
            ManagerState::WarmingUp => self.process_warmup(delta),
            ManagerState::Monitoring => self.process_monitoring(),
            ManagerState::Cooldown => self.process_cooldown(delta),
        }
    }
}

#[godot_api]
impl DclDynamicGraphicsManager {
    /// Signal emitted when profile should change. GDScript handles the actual change.
    #[signal]
    fn profile_change_requested(new_profile: i32);

    /// Initialize the manager with config values. Call this from GDScript after Global is ready.
    #[func]
    pub fn initialize(&mut self, enabled: bool, current_profile: i32) {
        self.enabled = enabled;
        self.current_profile = current_profile;

        // Start warmup if enabled and not custom profile
        if self.should_enable() {
            self.start_warmup();
        }

        tracing::info!(
            "DclDynamicGraphicsManager initialized: enabled={}, profile={}",
            self.enabled,
            self.current_profile
        );
    }

    /// Called when loading starts (from GDScript signal)
    #[func]
    pub fn on_loading_started(&mut self) {
        self.is_gameplay_active = false;
        self.reset_samples();
        tracing::debug!("DynamicGraphicsManager: loading started, pausing monitoring");
    }

    /// Called when loading finishes (from GDScript signal)
    #[func]
    pub fn on_loading_finished(&mut self) {
        self.is_gameplay_active = true;
        // Restart warmup after loading
        if self.state == ManagerState::Monitoring {
            self.start_warmup();
        }
        tracing::debug!("DynamicGraphicsManager: loading finished, resuming");
    }

    /// Called when user manually changes profile in settings
    #[func]
    pub fn on_manual_profile_change(&mut self, new_profile: i32) {
        self.current_profile = new_profile;

        // If user selects Custom (3), disable dynamic adjustment
        if new_profile == 3 {
            self.state = ManagerState::Disabled;
            tracing::info!("DynamicGraphicsManager: disabled (custom profile selected)");
        } else if self.should_enable() && self.state == ManagerState::Disabled {
            self.start_warmup();
            tracing::info!("DynamicGraphicsManager: re-enabled after manual change");
        }
    }

    /// Enable or disable dynamic graphics adjustment
    #[func]
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;

        if enabled && self.should_enable() {
            self.start_warmup();
        } else {
            self.state = ManagerState::Disabled;
        }

        tracing::info!("DclDynamicGraphicsManager: set_enabled({})", enabled);
    }

    /// Check if dynamic adjustment is currently active
    #[func]
    pub fn is_active(&self) -> bool {
        self.state != ManagerState::Disabled
    }

    /// Get current state for debugging
    #[func]
    pub fn get_state_string(&self) -> GString {
        match self.state {
            ManagerState::Disabled => "disabled".into(),
            ManagerState::WarmingUp => format!("warming_up ({:.0}s)", self.state_timer).into(),
            ManagerState::Monitoring => "monitoring".into(),
            ManagerState::Cooldown => format!("cooldown ({:.0}s)", self.state_timer).into(),
        }
    }

    /// Get current profile
    #[func]
    pub fn get_current_profile(&self) -> i32 {
        self.current_profile
    }

    /// Get average FPS from recent samples
    #[func]
    pub fn get_average_fps(&self) -> f64 {
        if self.fps_samples.is_empty() {
            return 60.0;
        }
        let sum: f64 = self.fps_samples.iter().sum();
        sum / self.fps_samples.len() as f64
    }

    /// Get current thermal state as string
    #[func]
    pub fn get_thermal_state_string(&self) -> GString {
        match self.thermal_state {
            ThermalState::Normal => "normal".into(),
            ThermalState::High => "high".into(),
            ThermalState::Critical => "critical".into(),
        }
    }
}

impl DclDynamicGraphicsManager {
    fn should_enable(&self) -> bool {
        // Disabled if dynamic graphics is off or profile is Custom (3)
        if !self.enabled {
            return false;
        }
        if self.current_profile == 3 {
            return false;
        }
        true
    }

    fn start_warmup(&mut self) {
        self.state = ManagerState::WarmingUp;
        self.state_timer = 0.0;
        self.reset_samples();
        tracing::debug!("DynamicGraphicsManager: starting warmup");
    }

    fn process_warmup(&mut self, delta: f64) {
        if !self.is_gameplay_active {
            return;
        }

        self.state_timer += delta;
        if self.state_timer >= WARMUP_DURATION {
            self.state = ManagerState::Monitoring;
            self.state_timer = 0.0;
            self.reset_samples();
            tracing::info!("DynamicGraphicsManager: warmup complete, starting monitoring");
        }
    }

    fn process_monitoring(&mut self) {
        if !self.is_gameplay_active {
            return;
        }

        // Check for immediate critical thermal downgrade
        if self.thermal_state == ThermalState::Critical {
            self.try_downgrade("critical thermal state");
            return;
        }

        // Check downgrade conditions (need DOWNGRADE_WINDOW seconds of samples)
        if self.fps_samples.len() >= DOWNGRADE_WINDOW as usize {
            let avg_fps = self.calculate_average_fps(DOWNGRADE_WINDOW as usize);
            let spike_ratio = self.calculate_spike_ratio();

            // Downgrade if FPS too low
            if avg_fps < FPS_DOWNGRADE_THRESHOLD {
                self.try_downgrade(&format!("low FPS (avg: {:.1})", avg_fps));
                return;
            }

            // Downgrade if too many frame spikes
            if spike_ratio > FRAME_SPIKE_RATIO_THRESHOLD {
                self.try_downgrade(&format!("frame spikes ({:.1}%)", spike_ratio * 100.0));
                return;
            }
        }

        // Check thermal high downgrade
        if self.thermal_high_timer >= THERMAL_HIGH_DOWNGRADE_TIME {
            self.try_downgrade("high thermal state");
            return;
        }

        // Check upgrade conditions (need UPGRADE_WINDOW seconds of samples)
        if self.fps_samples.len() >= UPGRADE_WINDOW as usize {
            let avg_fps = self.calculate_average_fps(UPGRADE_WINDOW as usize);
            let spike_ratio = self.calculate_spike_ratio();

            // All conditions must be met for upgrade
            let fps_ok = avg_fps >= FPS_UPGRADE_THRESHOLD;
            let spikes_ok = spike_ratio < FRAME_SPIKE_RATIO_THRESHOLD * 0.5; // More strict for upgrade
            let thermal_ok = self.thermal_state == ThermalState::Normal;

            if fps_ok && spikes_ok && thermal_ok {
                self.try_upgrade(&format!("good performance (FPS: {:.1})", avg_fps));
            }
        }
    }

    fn process_cooldown(&mut self, delta: f64) {
        self.state_timer += delta;
        if self.state_timer >= COOLDOWN_DURATION {
            self.state = ManagerState::Monitoring;
            self.state_timer = 0.0;
            self.reset_samples();
            tracing::debug!("DynamicGraphicsManager: cooldown complete, resuming monitoring");
        }
    }

    fn collect_fps_sample(&mut self) {
        let fps = godot::engine::Engine::singleton().get_frames_per_second() as f64;
        self.fps_samples.push(fps);

        // Keep only samples needed for upgrade window (larger window)
        let max_samples = UPGRADE_WINDOW as usize + 10;
        while self.fps_samples.len() > max_samples {
            self.fps_samples.remove(0);
        }
    }

    fn calculate_average_fps(&self, sample_count: usize) -> f64 {
        if self.fps_samples.is_empty() {
            return 60.0;
        }

        let count = sample_count.min(self.fps_samples.len());
        let start_index = self.fps_samples.len() - count;
        let sum: f64 = self.fps_samples[start_index..].iter().sum();
        sum / count as f64
    }

    fn calculate_spike_ratio(&self) -> f64 {
        if self.frame_count == 0 {
            return 0.0;
        }
        self.spike_count as f64 / self.frame_count as f64
    }

    fn update_thermal_state(&mut self, delta: f64) {
        let thermal_string = self.get_thermal_state_from_platform();

        match thermal_string.as_str() {
            "nominal" | "fair" => {
                self.thermal_state = ThermalState::Normal;
                self.thermal_high_timer = 0.0;
            }
            "serious" => {
                self.thermal_state = ThermalState::High;
                self.thermal_high_timer += delta;
            }
            "critical" => {
                self.thermal_state = ThermalState::Critical;
                self.thermal_high_timer = 0.0;
            }
            _ => {
                // Unknown or unavailable - assume normal
                self.thermal_state = ThermalState::Normal;
                self.thermal_high_timer = 0.0;
            }
        }
    }

    fn get_thermal_state_from_platform(&self) -> String {
        // Use cached availability flags to avoid per-frame singleton lookups
        if self.has_ios_plugin {
            return DclIosPlugin::get_thermal_state().to_string();
        }
        if self.has_android_plugin {
            return DclGodotAndroidPlugin::get_thermal_state().to_string();
        }
        "nominal".to_string()
    }

    fn try_downgrade(&mut self, reason: &str) {
        if self.current_profile <= MIN_PROFILE {
            // Already at lowest, can't downgrade further
            return;
        }

        let new_profile = self.current_profile - 1;
        self.apply_profile_change(new_profile, "downgrade", reason);
    }

    fn try_upgrade(&mut self, reason: &str) {
        if self.current_profile >= MAX_PROFILE {
            // Already at highest, can't upgrade further
            return;
        }

        let new_profile = self.current_profile + 1;
        self.apply_profile_change(new_profile, "upgrade", reason);
    }

    fn apply_profile_change(&mut self, new_profile: i32, action: &str, reason: &str) {
        let old_profile = self.current_profile;
        self.current_profile = new_profile;

        // Apply the profile via GDScript GraphicSettings
        self.call_apply_graphic_profile(new_profile);

        // Enter cooldown
        self.state = ManagerState::Cooldown;
        self.state_timer = 0.0;
        self.reset_samples();

        tracing::info!(
            "DynamicGraphicsManager: Profile {}: {} -> {} (reason: {})",
            action,
            old_profile,
            new_profile,
            reason
        );
    }

    fn call_apply_graphic_profile(&mut self, profile_index: i32) {
        // Emit signal for GDScript to handle the actual profile change
        self.base_mut().emit_signal(
            "profile_change_requested".into(),
            &[profile_index.to_variant()],
        );
    }

    fn reset_samples(&mut self) {
        self.fps_samples.clear();
        self.spike_count = 0;
        self.frame_count = 0;
        self.thermal_high_timer = 0.0;
        self.sample_timer = 0.0;
    }
}
