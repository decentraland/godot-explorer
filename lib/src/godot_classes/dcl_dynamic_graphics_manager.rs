use godot::classes::RenderingServer;
use godot::prelude::*;

use crate::godot_classes::{dcl_android_plugin::DclAndroidPlugin, dcl_ios_plugin::DclIosPlugin};

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
const WARMUP_DURATION: f64 = 60.0; // 1 minute before monitoring
const DOWNGRADE_WINDOW: f64 = 60.0; // Window for downgrade evaluation
const UPGRADE_WINDOW: f64 = 120.0; // Window for upgrade evaluation
const COOLDOWN_AFTER_DOWNGRADE: f64 = 120.0; // 2 minutes after downgrade (want to stabilize quickly)
const COOLDOWN_AFTER_UPGRADE: f64 = 300.0; // 5 minutes after upgrade (more conservative)
const THERMAL_HIGH_DOWNGRADE_TIME: f64 = 30.0; // Seconds of HIGH thermal before downgrade
const SAMPLE_INTERVAL: f64 = 1.0; // Sample frame time every second

/// Thermal FPS cap timing constants (in seconds)
const THERMAL_FPS_NORMAL_DURATION_FOR_UPGRADE: f64 = 120.0; // 2 minutes of Normal thermal before upgrading FPS cap
const THERMAL_FPS_CHANGE_COOLDOWN: f64 = 60.0; // 1 minute cooldown between FPS cap changes

/// Thermal FPS cap values
/// These caps are applied based on thermal state and charging status
/// The cap is a hard limit on FPS, independent of graphics profile
const THERMAL_FPS_CAP_30: i32 = 30;
const THERMAL_FPS_CAP_45: i32 = 45;
const THERMAL_FPS_CAP_60: i32 = 60;
const THERMAL_FPS_NO_CAP: i32 = 0; // 0 means no thermal cap (use user setting)

/// Threshold constants based on frame time ratio to target
/// If target is 33.3ms (30 FPS), these ratios determine thresholds
const FRAME_TIME_DOWNGRADE_RATIO: f64 = 1.2; // Downgrade if frame time > target * 1.2
const FRAME_TIME_UPGRADE_RATIO: f64 = 0.5; // Upgrade if frame time < target * 0.5 (plenty of headroom)
const FRAME_SPIKE_THRESHOLD_MS: f64 = 50.0; // Frames > 50ms are considered spikes/hiccups
const FRAME_SPIKE_RATIO_THRESHOLD: f64 = 0.1; // 10% of frames with spikes triggers downgrade

/// Profile indices
const PROFILE_VERY_LOW: i32 = 0;
#[allow(dead_code)]
const PROFILE_LOW: i32 = 1;
#[allow(dead_code)]
const PROFILE_MEDIUM: i32 = 2;
const PROFILE_HIGH: i32 = 3;
const PROFILE_CUSTOM: i32 = 4;

/// Profile bounds for dynamic adjustment (Custom is excluded)
const MIN_PROFILE: i32 = PROFILE_VERY_LOW;
const MAX_PROFILE: i32 = PROFILE_HIGH;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclDynamicGraphicsManager {
    base: Base<Node>,

    /// Current state machine state
    state: ManagerState,

    /// Timer for current state
    state_timer: f64,

    /// Frame time samples (actual process time in ms) collected every second
    frame_time_samples: Vec<f64>,

    /// Timer for sampling
    sample_timer: f64,

    /// Accumulated frame times for averaging within sample interval
    accumulated_frame_time: f64,
    accumulated_frame_count: u32,

    /// Frame spike tracking (frames > 50ms)
    spike_count: u32,
    frame_count: u32,

    /// Current thermal state
    thermal_state: ThermalState,

    /// Timer for high thermal state
    thermal_high_timer: f64,

    /// Current active profile
    current_profile: i32,

    /// Target frame time based on FPS limit (e.g., 33.3ms for 30 FPS)
    target_frame_time_ms: f64,

    /// Is gameplay active (not loading)
    is_gameplay_active: bool,

    /// Is dynamic adjustment enabled
    enabled: bool,

    /// Cached mobile platform availability (checked once at init)
    has_ios_plugin: bool,
    has_android_plugin: bool,

    /// Viewport RID for render time measurement
    viewport_rid: Option<Rid>,

    /// Whether render time measurement is enabled
    render_time_enabled: bool,

    /// Current cooldown duration (varies based on whether last change was upgrade or downgrade)
    current_cooldown_duration: f64,

    // === Thermal FPS Cap State ===
    /// Current thermal FPS cap (0 = no cap, use user setting)
    thermal_fps_cap: i32,

    /// Timer tracking consecutive seconds of Normal thermal state
    thermal_fps_normal_timer: f64,

    /// Cooldown timer for FPS cap changes
    thermal_fps_cooldown_timer: f64,

    /// Whether device is currently charging
    is_charging: bool,

    /// Whether thermal FPS cap system is enabled (separate from profile management)
    thermal_fps_cap_enabled: bool,
}

#[godot_api]
impl INode for DclDynamicGraphicsManager {
    fn init(base: Base<Node>) -> Self {
        // Check plugin availability once at init
        let has_ios_plugin = DclIosPlugin::is_available();
        let has_android_plugin = DclAndroidPlugin::is_available();

        Self {
            base,
            state: ManagerState::Disabled,
            state_timer: 0.0,
            frame_time_samples: Vec::with_capacity(150),
            sample_timer: 0.0,
            accumulated_frame_time: 0.0,
            accumulated_frame_count: 0,
            spike_count: 0,
            frame_count: 0,
            thermal_state: ThermalState::Normal,
            thermal_high_timer: 0.0,
            current_profile: 0,
            target_frame_time_ms: 33.3, // Default for 30 FPS
            is_gameplay_active: false,
            enabled: true,
            has_ios_plugin,
            has_android_plugin,
            viewport_rid: None,
            render_time_enabled: false,
            current_cooldown_duration: COOLDOWN_AFTER_DOWNGRADE,
            // Thermal FPS cap state
            thermal_fps_cap: THERMAL_FPS_NO_CAP,
            thermal_fps_normal_timer: 0.0,
            thermal_fps_cooldown_timer: 0.0,
            is_charging: false,
            thermal_fps_cap_enabled: true, // Enabled by default on mobile
        }
    }

    fn ready(&mut self) {
        // Note: DclGlobal is not available yet during init, so we start disabled.
        // GDScript should call initialize() after Global is ready.

        // Get viewport RID and enable render time measurement
        if let Some(viewport) = self.base().get_viewport() {
            let rid = viewport.get_viewport_rid();
            self.viewport_rid = Some(rid);

            // Enable render time measurement on the viewport
            RenderingServer::singleton().viewport_set_measure_render_time(rid, true);
            self.render_time_enabled = true;

            godot_print!(
                "[DynamicGraphics] ready: render time measurement enabled (iOS={}, Android={})",
                self.has_ios_plugin,
                self.has_android_plugin
            );
        } else {
            godot_print!(
                "[DynamicGraphics] WARNING: could not get viewport (iOS={}, Android={})",
                self.has_ios_plugin,
                self.has_android_plugin
            );
        }
    }

    fn process(&mut self, delta: f64) {
        // Update thermal state and charging state (always, even when profile management is disabled)
        self.update_thermal_state(delta);
        self.update_charging_state();

        // Process thermal FPS cap (independent of profile management state)
        if self.thermal_fps_cap_enabled {
            self.process_thermal_fps_cap(delta);
        }

        // Profile management is disabled, skip the rest
        if self.state == ManagerState::Disabled {
            return;
        }

        // Get actual render time (CPU + GPU) from RenderingServer
        // This gives us the real work time, independent of FPS cap
        let render_time_ms = self.get_total_render_time_ms();

        // Track frame spikes (using actual render time, not delta which includes wait time)
        self.frame_count += 1;
        if render_time_ms > FRAME_SPIKE_THRESHOLD_MS {
            self.spike_count += 1;
        }

        // Accumulate frame times for averaging
        self.accumulated_frame_time += render_time_ms;
        self.accumulated_frame_count += 1;

        // Sample frame time periodically (average over the interval)
        self.sample_timer += delta;
        if self.sample_timer >= SAMPLE_INTERVAL {
            self.sample_timer = 0.0;
            self.collect_frame_time_sample();
        }

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

    /// Signal emitted when thermal FPS cap changes. GDScript handles applying the cap.
    /// fps_cap: The new FPS cap value (30, 45, 60) or 0 for no cap (use user setting)
    #[signal]
    fn thermal_fps_cap_changed(fps_cap: i32);

    /// Initialize the manager with config values. Call this from GDScript after Global is ready.
    /// fps_limit: The FPS limit mode (0=VSYNC, 1=NO_LIMIT, 2=18fps, 3=30fps, 4=60fps, 5=120fps)
    #[func]
    pub fn initialize(&mut self, enabled: bool, current_profile: i32, fps_limit: i32) {
        self.enabled = enabled;
        self.current_profile = current_profile;
        self.target_frame_time_ms = Self::fps_limit_to_target_ms(fps_limit);

        // Start warmup if enabled and not custom profile
        if self.should_enable() {
            self.start_warmup();
        }

        godot_print!(
            "[DynamicGraphics] initialized: enabled={}, profile={}, fps_limit={}, target_frame_time={}ms",
            self.enabled,
            self.current_profile,
            fps_limit,
            self.target_frame_time_ms
        );
    }

    /// Update target frame time when FPS limit changes
    #[func]
    pub fn on_fps_limit_changed(&mut self, fps_limit: i32) {
        self.target_frame_time_ms = Self::fps_limit_to_target_ms(fps_limit);
        godot_print!(
            "[DynamicGraphics] FPS limit changed, new target_frame_time={}ms",
            self.target_frame_time_ms
        );
    }

    /// Called when loading starts (from GDScript signal)
    #[func]
    pub fn on_loading_started(&mut self) {
        self.is_gameplay_active = false;
        self.reset_samples();
        godot_print!("[DynamicGraphics] loading started, pausing monitoring");
    }

    /// Called when loading finishes (from GDScript signal)
    #[func]
    pub fn on_loading_finished(&mut self) {
        self.is_gameplay_active = true;
        // Restart warmup after loading
        if self.state == ManagerState::Monitoring {
            self.start_warmup();
        }
        godot_print!(
            "[DynamicGraphics] loading finished, resuming (state={:?})",
            self.state
        );
    }

    /// Check if gameplay is currently active (not loading)
    #[func]
    pub fn is_gameplay_active(&self) -> bool {
        self.is_gameplay_active
    }

    /// Get debug info string for troubleshooting
    #[func]
    pub fn get_debug_info(&self) -> GString {
        GString::from(
            format!(
                "state={:?}, timer={:.1}s, gameplay_active={}, samples={}",
                self.state,
                self.state_timer,
                self.is_gameplay_active,
                self.frame_time_samples.len()
            )
            .as_str(),
        )
    }

    /// Called when user manually changes profile in settings
    #[func]
    pub fn on_manual_profile_change(&mut self, new_profile: i32) {
        self.current_profile = new_profile;

        // If user selects Custom, disable dynamic adjustment
        if new_profile == PROFILE_CUSTOM {
            self.state = ManagerState::Disabled;
            godot_print!("[DynamicGraphics] disabled (custom profile selected)");
        } else if self.should_enable() && self.state == ManagerState::Disabled {
            self.start_warmup();
            godot_print!("[DynamicGraphics] re-enabled after manual change");
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

        godot_print!("[DynamicGraphics] set_enabled({})", enabled);
    }

    /// Check if dynamic adjustment is currently active
    #[func]
    pub fn is_active(&self) -> bool {
        self.state != ManagerState::Disabled
    }

    /// Check if dynamic adjustment is enabled (regardless of state)
    #[func]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get current state name (without timer info)
    #[func]
    pub fn get_state_name(&self) -> GString {
        match self.state {
            ManagerState::Disabled => "Disabled".into(),
            ManagerState::WarmingUp => "WarmingUp".into(),
            ManagerState::Monitoring => "Monitoring".into(),
            ManagerState::Cooldown => "Cooldown".into(),
        }
    }

    /// Get current state for debugging (with timer info)
    #[func]
    pub fn get_state_string(&self) -> GString {
        match self.state {
            ManagerState::Disabled => "disabled".into(),
            ManagerState::WarmingUp => {
                GString::from(format!("warming_up ({:.0}s)", self.state_timer).as_str())
            }
            ManagerState::Monitoring => "monitoring".into(),
            ManagerState::Cooldown => {
                GString::from(format!("cooldown ({:.0}s)", self.state_timer).as_str())
            }
        }
    }

    /// Get remaining warmup time in seconds
    #[func]
    pub fn get_warmup_remaining(&self) -> f64 {
        if self.state == ManagerState::WarmingUp {
            (WARMUP_DURATION - self.state_timer).max(0.0)
        } else {
            0.0
        }
    }

    /// Get remaining cooldown time in seconds
    #[func]
    pub fn get_cooldown_remaining(&self) -> f64 {
        if self.state == ManagerState::Cooldown {
            (self.current_cooldown_duration - self.state_timer).max(0.0)
        } else {
            0.0
        }
    }

    /// Get current profile
    #[func]
    pub fn get_current_profile(&self) -> i32 {
        self.current_profile
    }

    /// Get average frame time from recent samples (in ms)
    #[func]
    pub fn get_average_frame_time(&self) -> f64 {
        if self.frame_time_samples.is_empty() {
            return self.target_frame_time_ms;
        }
        let sum: f64 = self.frame_time_samples.iter().sum();
        sum / self.frame_time_samples.len() as f64
    }

    /// Get target frame time (in ms)
    #[func]
    pub fn get_target_frame_time(&self) -> f64 {
        self.target_frame_time_ms
    }

    /// Get frame time ratio (actual / target). < 1.0 means headroom, > 1.0 means struggling
    #[func]
    pub fn get_frame_time_ratio(&self) -> f64 {
        self.get_average_frame_time() / self.target_frame_time_ms
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

    // === Thermal FPS Cap Public API ===

    /// Enable or disable thermal FPS cap system
    #[func]
    pub fn set_thermal_fps_cap_enabled(&mut self, enabled: bool) {
        let was_enabled = self.thermal_fps_cap_enabled;
        self.thermal_fps_cap_enabled = enabled;

        if !enabled && was_enabled && self.thermal_fps_cap != THERMAL_FPS_NO_CAP {
            // Reset to no cap when disabled
            self.apply_thermal_fps_cap(THERMAL_FPS_NO_CAP, "disabled by user");
        }

        godot_print!(
            "[DynamicGraphics] thermal FPS cap enabled={}",
            self.thermal_fps_cap_enabled
        );
    }

    /// Check if thermal FPS cap system is enabled
    #[func]
    pub fn is_thermal_fps_cap_enabled(&self) -> bool {
        self.thermal_fps_cap_enabled
    }

    /// Get current thermal FPS cap value (0 = no cap)
    #[func]
    pub fn get_thermal_fps_cap(&self) -> i32 {
        self.thermal_fps_cap
    }

    /// Get current charging state
    #[func]
    pub fn is_device_charging(&self) -> bool {
        self.is_charging
    }

    /// Get thermal FPS cap debug info
    #[func]
    pub fn get_thermal_fps_cap_debug_info(&self) -> GString {
        GString::from(
            format!(
                "thermal_fps_cap={}, normal_timer={:.1}s, cooldown={:.1}s, charging={}",
                if self.thermal_fps_cap == THERMAL_FPS_NO_CAP {
                    "none".to_string()
                } else {
                    format!("{}fps", self.thermal_fps_cap)
                },
                self.thermal_fps_normal_timer,
                self.thermal_fps_cooldown_timer,
                self.is_charging
            )
            .as_str(),
        )
    }
}

impl DclDynamicGraphicsManager {
    fn should_enable(&self) -> bool {
        // Disabled if dynamic graphics is off or profile is Custom
        if !self.enabled {
            return false;
        }
        if self.current_profile == PROFILE_CUSTOM {
            return false;
        }
        true
    }

    fn start_warmup(&mut self) {
        self.state = ManagerState::WarmingUp;
        self.state_timer = 0.0;
        self.reset_samples();
        godot_print!("[DynamicGraphics] starting warmup ({}s)", WARMUP_DURATION);
    }

    fn process_warmup(&mut self, delta: f64) {
        if !self.is_gameplay_active {
            return;
        }

        let old_timer = self.state_timer;
        self.state_timer += delta;

        // Log progress every 30 seconds
        let old_30s = (old_timer / 30.0) as i32;
        let new_30s = (self.state_timer / 30.0) as i32;
        if new_30s > old_30s {
            godot_print!(
                "[DynamicGraphics] warmup progress {:.0}s / {:.0}s",
                self.state_timer,
                WARMUP_DURATION
            );
        }

        if self.state_timer >= WARMUP_DURATION {
            self.state = ManagerState::Monitoring;
            self.state_timer = 0.0;
            self.reset_samples();
            godot_print!("[DynamicGraphics] warmup complete, starting monitoring");
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
        if self.frame_time_samples.len() >= DOWNGRADE_WINDOW as usize {
            // Use P95 for downgrade evaluation (better at catching stutters than average)
            let p95_frame_time = self.calculate_p95_frame_time(DOWNGRADE_WINDOW as usize);
            let frame_time_ratio = p95_frame_time / self.target_frame_time_ms;
            let spike_ratio = self.calculate_spike_ratio();

            // Downgrade if P95 frame time too high (ratio > 1.2 means 20% over budget)
            if frame_time_ratio > FRAME_TIME_DOWNGRADE_RATIO {
                self.try_downgrade(&format!(
                    "high P95 frame time ({:.1}ms, {:.0}% of budget)",
                    p95_frame_time,
                    frame_time_ratio * 100.0
                ));
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
        if self.frame_time_samples.len() >= UPGRADE_WINDOW as usize {
            let avg_frame_time = self.calculate_average_frame_time(UPGRADE_WINDOW as usize);
            let frame_time_ratio = avg_frame_time / self.target_frame_time_ms;
            let spike_ratio = self.calculate_spike_ratio();

            // All conditions must be met for upgrade
            // Ratio < 0.5 means using less than 50% of frame budget (plenty of headroom)
            let frame_time_ok = frame_time_ratio < FRAME_TIME_UPGRADE_RATIO;
            let spikes_ok = spike_ratio < FRAME_SPIKE_RATIO_THRESHOLD * 0.5; // More strict for upgrade
            let thermal_ok = self.thermal_state == ThermalState::Normal;

            if frame_time_ok && spikes_ok && thermal_ok {
                self.try_upgrade(&format!(
                    "good performance ({:.1}ms, {:.0}% of budget)",
                    avg_frame_time,
                    frame_time_ratio * 100.0
                ));
            }
        }
    }

    fn process_cooldown(&mut self, delta: f64) {
        self.state_timer += delta;
        if self.state_timer >= self.current_cooldown_duration {
            self.state = ManagerState::Monitoring;
            self.state_timer = 0.0;
            self.reset_samples();
            godot_print!("[DynamicGraphics] cooldown complete, resuming monitoring");
        }
    }

    fn collect_frame_time_sample(&mut self) {
        // Calculate average frame time from accumulated values
        if self.accumulated_frame_count > 0 {
            let avg_frame_time = self.accumulated_frame_time / self.accumulated_frame_count as f64;
            self.frame_time_samples.push(avg_frame_time);

            // Reset accumulators
            self.accumulated_frame_time = 0.0;
            self.accumulated_frame_count = 0;
        }

        // Keep only samples needed for upgrade window (larger window)
        let max_samples = UPGRADE_WINDOW as usize + 10;
        while self.frame_time_samples.len() > max_samples {
            self.frame_time_samples.remove(0);
        }
    }

    fn calculate_average_frame_time(&self, sample_count: usize) -> f64 {
        if self.frame_time_samples.is_empty() {
            return self.target_frame_time_ms;
        }

        let count = sample_count.min(self.frame_time_samples.len());
        let start_index = self.frame_time_samples.len() - count;
        let sum: f64 = self.frame_time_samples[start_index..].iter().sum();
        sum / count as f64
    }

    fn calculate_spike_ratio(&self) -> f64 {
        if self.frame_count == 0 {
            return 0.0;
        }
        self.spike_count as f64 / self.frame_count as f64
    }

    /// Calculate the 95th percentile frame time (better at catching spikes than average)
    fn calculate_p95_frame_time(&self, sample_count: usize) -> f64 {
        if self.frame_time_samples.is_empty() {
            return self.target_frame_time_ms;
        }

        let count = sample_count.min(self.frame_time_samples.len());
        let start_index = self.frame_time_samples.len() - count;
        let mut sorted: Vec<f64> = self.frame_time_samples[start_index..].to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Get the 95th percentile index
        let p95_index = ((sorted.len() as f64) * 0.95) as usize;
        let p95_index = p95_index.min(sorted.len().saturating_sub(1));

        sorted
            .get(p95_index)
            .copied()
            .unwrap_or(self.target_frame_time_ms)
    }

    /// Convert FPS limit mode to target frame time in milliseconds
    fn fps_limit_to_target_ms(fps_limit: i32) -> f64 {
        match fps_limit {
            0 | 1 => 16.6, // VSYNC or NO_LIMIT - assume 60 FPS target
            2 => 55.6,     // 18 FPS (Very Low profile)
            3 => 33.3,     // 30 FPS
            4 => 16.6,     // 60 FPS
            5 => 8.3,      // 120 FPS
            _ => 33.3,     // Default to 30 FPS
        }
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
            return DclAndroidPlugin::get_thermal_state().to_string();
        }
        "nominal".to_string()
    }

    /// Get total render time (CPU + GPU) in milliseconds
    /// This is the actual work time, independent of FPS cap
    fn get_total_render_time_ms(&self) -> f64 {
        if !self.render_time_enabled {
            // Fallback to delta-based estimation if measurement not available
            return self.target_frame_time_ms;
        }

        if let Some(rid) = self.viewport_rid {
            let rs = RenderingServer::singleton();

            // Get CPU and GPU render times (in milliseconds)
            let cpu_time = rs.viewport_get_measured_render_time_cpu(rid);
            let gpu_time = rs.viewport_get_measured_render_time_gpu(rid);

            // Return the larger of the two (bottleneck determines frame time)
            // In practice, we want to know if either CPU or GPU is struggling
            cpu_time.max(gpu_time)
        } else {
            self.target_frame_time_ms
        }
    }

    fn try_downgrade(&mut self, reason: &str) {
        if self.current_profile <= MIN_PROFILE {
            // Already at lowest, can't downgrade further
            return;
        }

        let new_profile = self.current_profile - 1;
        self.current_cooldown_duration = COOLDOWN_AFTER_DOWNGRADE;
        self.apply_profile_change(new_profile, "downgrade", reason);
    }

    fn try_upgrade(&mut self, reason: &str) {
        if self.current_profile >= MAX_PROFILE {
            // Already at highest, can't upgrade further
            return;
        }

        let new_profile = self.current_profile + 1;
        self.current_cooldown_duration = COOLDOWN_AFTER_UPGRADE;
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

        godot_print!(
            "[DynamicGraphics] Profile {}: {} -> {} (reason: {})",
            action,
            old_profile,
            new_profile,
            reason
        );
    }

    fn call_apply_graphic_profile(&mut self, profile_index: i32) {
        // Emit signal for GDScript to handle the actual profile change
        self.base_mut()
            .emit_signal("profile_change_requested", &[profile_index.to_variant()]);
    }

    fn reset_samples(&mut self) {
        self.frame_time_samples.clear();
        self.accumulated_frame_time = 0.0;
        self.accumulated_frame_count = 0;
        self.spike_count = 0;
        self.frame_count = 0;
        self.thermal_high_timer = 0.0;
        self.sample_timer = 0.0;
    }

    // === Thermal FPS Cap Internal Functions ===

    /// Update charging state from platform
    fn update_charging_state(&mut self) {
        let charging_string = self.get_charging_state_from_platform();
        self.is_charging = charging_string == "charging";
    }

    /// Get charging state from platform plugins
    fn get_charging_state_from_platform(&self) -> String {
        if self.has_ios_plugin {
            return DclIosPlugin::get_charging_state().to_string();
        }
        if self.has_android_plugin {
            return DclAndroidPlugin::get_charging_state().to_string();
        }
        "unknown".to_string()
    }

    /// Process thermal FPS cap logic
    /// This runs independently of the profile management state machine
    fn process_thermal_fps_cap(&mut self, delta: f64) {
        // Update cooldown timer
        if self.thermal_fps_cooldown_timer > 0.0 {
            self.thermal_fps_cooldown_timer -= delta;
            if self.thermal_fps_cooldown_timer < 0.0 {
                self.thermal_fps_cooldown_timer = 0.0;
            }
        }

        // Calculate the target FPS cap based on current thermal state and charging
        let target_cap = self.calculate_target_fps_cap();

        // Check if we need to change the cap
        if target_cap != self.thermal_fps_cap {
            // Determine if this is a downgrade (more restrictive) or upgrade (less restrictive)
            let is_downgrade = self.is_fps_cap_downgrade(self.thermal_fps_cap, target_cap);

            if is_downgrade {
                // Downgrade immediately (no cooldown check for downgrade)
                self.apply_thermal_fps_cap(target_cap, "thermal downgrade");
                self.thermal_fps_cooldown_timer = THERMAL_FPS_CHANGE_COOLDOWN;
                self.thermal_fps_normal_timer = 0.0; // Reset normal timer on downgrade
            } else {
                // Upgrade requires sustained Normal thermal and cooldown to be clear
                if self.thermal_fps_cooldown_timer <= 0.0 {
                    // Check if we've had enough time at Normal thermal to upgrade
                    if self.thermal_state == ThermalState::Normal {
                        self.thermal_fps_normal_timer += delta;

                        if self.thermal_fps_normal_timer >= THERMAL_FPS_NORMAL_DURATION_FOR_UPGRADE
                        {
                            self.apply_thermal_fps_cap(
                                target_cap,
                                "thermal upgrade after stable period",
                            );
                            self.thermal_fps_cooldown_timer = THERMAL_FPS_CHANGE_COOLDOWN;
                            self.thermal_fps_normal_timer = 0.0;
                        }
                    } else {
                        // Not Normal thermal, reset the timer
                        self.thermal_fps_normal_timer = 0.0;
                    }
                }
            }
        } else {
            // Already at target cap
            // Update normal timer if at Normal thermal
            if self.thermal_state == ThermalState::Normal {
                self.thermal_fps_normal_timer += delta;
            } else {
                self.thermal_fps_normal_timer = 0.0;
            }
        }
    }

    /// Calculate the target FPS cap based on thermal state and charging
    /// Returns the FPS cap value, or THERMAL_FPS_NO_CAP (0) for no cap
    ///
    /// FPS Cap Rules:
    /// | Thermal State | Charging | Max FPS Cap    |
    /// |---------------|----------|----------------|
    /// | Normal        | No       | No Limit       |
    /// | Normal        | Yes      | 60 FPS         |
    /// | High          | No       | 45 FPS         |
    /// | High          | Yes      | 30 FPS         |
    /// | Critical      | Any      | 30 FPS         |
    fn calculate_target_fps_cap(&self) -> i32 {
        match self.thermal_state {
            ThermalState::Normal => {
                if self.is_charging {
                    THERMAL_FPS_CAP_60
                } else {
                    THERMAL_FPS_NO_CAP // No limit
                }
            }
            ThermalState::High => {
                if self.is_charging {
                    THERMAL_FPS_CAP_30
                } else {
                    THERMAL_FPS_CAP_45
                }
            }
            ThermalState::Critical => THERMAL_FPS_CAP_30,
        }
    }

    /// Check if changing from current_cap to new_cap is a downgrade (more restrictive)
    /// A lower FPS cap or going from no cap to any cap is a downgrade
    fn is_fps_cap_downgrade(&self, current_cap: i32, new_cap: i32) -> bool {
        // No cap (0) is the least restrictive
        if current_cap == THERMAL_FPS_NO_CAP {
            // Going from no cap to any cap is a downgrade
            return new_cap != THERMAL_FPS_NO_CAP;
        }
        if new_cap == THERMAL_FPS_NO_CAP {
            // Going from any cap to no cap is an upgrade
            return false;
        }
        // Lower FPS cap value = more restrictive = downgrade
        new_cap < current_cap
    }

    /// Apply a new thermal FPS cap
    fn apply_thermal_fps_cap(&mut self, new_cap: i32, reason: &str) {
        let old_cap = self.thermal_fps_cap;
        self.thermal_fps_cap = new_cap;

        // Emit signal for GDScript to apply the cap
        self.base_mut()
            .emit_signal("thermal_fps_cap_changed", &[new_cap.to_variant()]);

        godot_print!(
            "[DynamicGraphics] Thermal FPS cap: {} -> {} (reason: {}, charging: {}, thermal: {:?})",
            if old_cap == THERMAL_FPS_NO_CAP {
                "none".to_string()
            } else {
                format!("{}fps", old_cap)
            },
            if new_cap == THERMAL_FPS_NO_CAP {
                "none".to_string()
            } else {
                format!("{}fps", new_cap)
            },
            reason,
            self.is_charging,
            self.thermal_state
        );
    }
}
