use godot::classes::RenderingServer;
use godot::prelude::*;

use crate::godot_classes::{dcl_android_plugin::DclAndroidPlugin, dcl_ios_plugin::DclIosPlugin};

/// Thermal state levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalState {
    Normal,
    High,
    Critical,
}

/// Manager state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagerState {
    Disabled,
    WarmingUp,
    Monitoring,
    Cooldown,
}

/// Action returned by state machine when profile should change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileAction {
    Downgrade(i32),
    Upgrade(i32),
}

/// Timing constants (in seconds)
const WARMUP_DURATION: f64 = 180.0; // 3 minutes before monitoring (spec: 3-5 min)
const DOWNGRADE_WINDOW: f64 = 60.0; // Window for downgrade evaluation
const UPGRADE_WINDOW: f64 = 120.0; // Window for upgrade evaluation
const COOLDOWN_AFTER_DOWNGRADE: f64 = 120.0; // 2 minutes after downgrade
const COOLDOWN_AFTER_UPGRADE: f64 = 300.0; // 5 minutes after upgrade
const THERMAL_HIGH_DOWNGRADE_TIME: f64 = 30.0; // Seconds of HIGH thermal before downgrade
const SAMPLE_INTERVAL: f64 = 1.0; // Sample frame time every second
const PLATFORM_POLL_INTERVAL: f64 = 5.0; // Poll thermal/charging state every 5 seconds

/// Threshold constants
const FRAME_TIME_DOWNGRADE_RATIO: f64 = 1.2;
const FRAME_TIME_UPGRADE_RATIO: f64 = 0.5;
const FRAME_SPIKE_THRESHOLD_MS: f64 = 50.0;
const FRAME_SPIKE_RATIO_THRESHOLD: f64 = 0.1;

/// Thermal FPS cap timing constants (in seconds)
const THERMAL_FPS_NORMAL_DURATION_FOR_UPGRADE: f64 = 120.0; // 2 minutes of Normal thermal before upgrading
const THERMAL_FPS_CHANGE_COOLDOWN: f64 = 60.0; // 1 minute cooldown between changes

/// Thermal FPS cap values (0 = no cap)
const THERMAL_FPS_CAP_30: i32 = 30;
const THERMAL_FPS_CAP_45: i32 = 45;
const THERMAL_FPS_CAP_60: i32 = 60;
const THERMAL_FPS_NO_CAP: i32 = 0;

/// Profile indices
const PROFILE_VERY_LOW: i32 = 0;
const PROFILE_HIGH: i32 = 3;
const PROFILE_CUSTOM: i32 = 4;

const MIN_PROFILE: i32 = PROFILE_VERY_LOW;
const MAX_PROFILE: i32 = PROFILE_HIGH;

// ============================================================================
// DynamicGraphicsState - Pure state machine, no Godot dependencies
// ============================================================================

/// Pure state machine for dynamic graphics adjustment.
/// Can be tested without Godot runtime.
#[derive(Debug)]
pub struct DynamicGraphicsState {
    pub state: ManagerState,
    pub state_timer: f64,
    pub frame_time_samples: Vec<f64>,
    pub sample_timer: f64,
    pub accumulated_frame_time: f64,
    pub accumulated_frame_count: u32,
    pub spike_count: u32,
    pub frame_count: u32,
    pub thermal_state: ThermalState,
    pub thermal_high_timer: f64,
    pub current_profile: i32,
    pub target_frame_time_ms: f64,
    pub is_gameplay_active: bool,
    pub enabled: bool,
    pub current_cooldown_duration: f64,
}

impl Default for DynamicGraphicsState {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicGraphicsState {
    pub fn new() -> Self {
        Self {
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
            target_frame_time_ms: 33.3,
            is_gameplay_active: false,
            enabled: true,
            current_cooldown_duration: COOLDOWN_AFTER_DOWNGRADE,
        }
    }

    /// Initialize with config values
    pub fn initialize(&mut self, enabled: bool, current_profile: i32, fps_limit: i32) {
        self.enabled = enabled;
        self.current_profile = current_profile;
        self.target_frame_time_ms = Self::fps_limit_to_target_ms(fps_limit);

        if self.should_enable() {
            self.start_warmup();
        }
    }

    /// Process one frame. Returns Some(ProfileAction) if profile should change.
    pub fn process_frame(
        &mut self,
        delta: f64,
        render_time_ms: f64,
        thermal_str: &str,
    ) -> Option<ProfileAction> {
        if self.state == ManagerState::Disabled {
            return None;
        }

        // Track frame spikes
        self.frame_count += 1;
        if render_time_ms > FRAME_SPIKE_THRESHOLD_MS {
            self.spike_count += 1;
        }

        // Accumulate frame times
        self.accumulated_frame_time += render_time_ms;
        self.accumulated_frame_count += 1;

        // Sample periodically
        self.sample_timer += delta;
        if self.sample_timer >= SAMPLE_INTERVAL {
            self.sample_timer = 0.0;
            self.collect_frame_time_sample();
        }

        // Update thermal state
        self.update_thermal_state(delta, thermal_str);

        // Process state machine
        match self.state {
            ManagerState::Disabled => None,
            ManagerState::WarmingUp => {
                self.process_warmup(delta);
                None
            }
            ManagerState::Monitoring => self.process_monitoring(),
            ManagerState::Cooldown => {
                self.process_cooldown(delta);
                None
            }
        }
    }

    pub fn on_loading_started(&mut self) {
        self.is_gameplay_active = false;
        self.reset_samples();
    }

    pub fn on_loading_finished(&mut self) {
        self.is_gameplay_active = true;
        if self.state == ManagerState::Monitoring {
            self.start_warmup();
        }
    }

    pub fn on_manual_profile_change(&mut self, new_profile: i32) {
        self.current_profile = new_profile;

        if new_profile == PROFILE_CUSTOM {
            self.state = ManagerState::Disabled;
        } else if self.should_enable() && self.state == ManagerState::Disabled {
            self.start_warmup();
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;

        if enabled && self.should_enable() {
            self.start_warmup();
        } else {
            self.state = ManagerState::Disabled;
        }
    }

    pub fn on_fps_limit_changed(&mut self, fps_limit: i32) {
        self.target_frame_time_ms = Self::fps_limit_to_target_ms(fps_limit);
    }

    // Getters
    pub fn is_active(&self) -> bool {
        self.state != ManagerState::Disabled
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn get_warmup_remaining(&self) -> f64 {
        if self.state == ManagerState::WarmingUp {
            (WARMUP_DURATION - self.state_timer).max(0.0)
        } else {
            0.0
        }
    }

    pub fn get_cooldown_remaining(&self) -> f64 {
        if self.state == ManagerState::Cooldown {
            (self.current_cooldown_duration - self.state_timer).max(0.0)
        } else {
            0.0
        }
    }

    pub fn get_average_frame_time(&self) -> f64 {
        if self.frame_time_samples.is_empty() {
            return self.target_frame_time_ms;
        }
        let sum: f64 = self.frame_time_samples.iter().sum();
        sum / self.frame_time_samples.len() as f64
    }

    pub fn get_frame_time_ratio(&self) -> f64 {
        self.get_average_frame_time() / self.target_frame_time_ms
    }

    // Internal methods
    fn should_enable(&self) -> bool {
        self.enabled && self.current_profile != PROFILE_CUSTOM
    }

    fn start_warmup(&mut self) {
        self.state = ManagerState::WarmingUp;
        self.state_timer = 0.0;
        self.reset_samples();
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
        }
    }

    fn process_monitoring(&mut self) -> Option<ProfileAction> {
        if !self.is_gameplay_active {
            return None;
        }

        // Critical thermal - immediate downgrade
        if self.thermal_state == ThermalState::Critical {
            return self.try_downgrade();
        }

        // Check downgrade conditions
        if self.frame_time_samples.len() >= DOWNGRADE_WINDOW as usize {
            let p95_frame_time = self.calculate_p95_frame_time(DOWNGRADE_WINDOW as usize);
            let frame_time_ratio = p95_frame_time / self.target_frame_time_ms;
            let spike_ratio = self.calculate_spike_ratio();

            if frame_time_ratio > FRAME_TIME_DOWNGRADE_RATIO {
                return self.try_downgrade();
            }

            if spike_ratio > FRAME_SPIKE_RATIO_THRESHOLD {
                return self.try_downgrade();
            }
        }

        // Thermal high downgrade
        if self.thermal_high_timer >= THERMAL_HIGH_DOWNGRADE_TIME {
            return self.try_downgrade();
        }

        // Check upgrade conditions
        if self.frame_time_samples.len() >= UPGRADE_WINDOW as usize {
            let avg_frame_time = self.calculate_average_frame_time(UPGRADE_WINDOW as usize);
            let frame_time_ratio = avg_frame_time / self.target_frame_time_ms;
            let spike_ratio = self.calculate_spike_ratio();

            let frame_time_ok = frame_time_ratio < FRAME_TIME_UPGRADE_RATIO;
            let spikes_ok = spike_ratio < FRAME_SPIKE_RATIO_THRESHOLD * 0.5;
            let thermal_ok = self.thermal_state == ThermalState::Normal;

            if frame_time_ok && spikes_ok && thermal_ok {
                return self.try_upgrade();
            }
        }

        None
    }

    fn process_cooldown(&mut self, delta: f64) {
        self.state_timer += delta;
        if self.state_timer >= self.current_cooldown_duration {
            self.state = ManagerState::Monitoring;
            self.state_timer = 0.0;
            self.reset_samples();
        }
    }

    fn try_downgrade(&mut self) -> Option<ProfileAction> {
        if self.current_profile <= MIN_PROFILE {
            return None;
        }

        let new_profile = self.current_profile - 1;
        self.current_cooldown_duration = COOLDOWN_AFTER_DOWNGRADE;
        self.apply_profile_change(new_profile);
        Some(ProfileAction::Downgrade(new_profile))
    }

    fn try_upgrade(&mut self) -> Option<ProfileAction> {
        if self.current_profile >= MAX_PROFILE {
            return None;
        }

        let new_profile = self.current_profile + 1;
        self.current_cooldown_duration = COOLDOWN_AFTER_UPGRADE;
        self.apply_profile_change(new_profile);
        Some(ProfileAction::Upgrade(new_profile))
    }

    fn apply_profile_change(&mut self, new_profile: i32) {
        self.current_profile = new_profile;
        self.state = ManagerState::Cooldown;
        self.state_timer = 0.0;
        self.reset_samples();
    }

    fn collect_frame_time_sample(&mut self) {
        if self.accumulated_frame_count > 0 {
            let avg = self.accumulated_frame_time / self.accumulated_frame_count as f64;
            self.frame_time_samples.push(avg);
            self.accumulated_frame_time = 0.0;
            self.accumulated_frame_count = 0;
        }

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
        let start = self.frame_time_samples.len() - count;
        let sum: f64 = self.frame_time_samples[start..].iter().sum();
        sum / count as f64
    }

    fn calculate_spike_ratio(&self) -> f64 {
        if self.frame_count == 0 {
            return 0.0;
        }
        self.spike_count as f64 / self.frame_count as f64
    }

    fn calculate_p95_frame_time(&self, sample_count: usize) -> f64 {
        if self.frame_time_samples.is_empty() {
            return self.target_frame_time_ms;
        }

        let count = sample_count.min(self.frame_time_samples.len());
        let start = self.frame_time_samples.len() - count;
        let mut sorted: Vec<f64> = self.frame_time_samples[start..].to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let idx = ((sorted.len() as f64) * 0.95) as usize;
        let idx = idx.min(sorted.len().saturating_sub(1));

        sorted
            .get(idx)
            .copied()
            .unwrap_or(self.target_frame_time_ms)
    }

    fn update_thermal_state(&mut self, delta: f64, thermal_str: &str) {
        match thermal_str {
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
                self.thermal_state = ThermalState::Normal;
                self.thermal_high_timer = 0.0;
            }
        }
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

    pub fn fps_limit_to_target_ms(fps_limit: i32) -> f64 {
        match fps_limit {
            0 | 1 => 16.6,
            2 => 55.6,
            3 => 33.3,
            4 => 16.6,
            5 => 8.3,
            _ => 33.3,
        }
    }
}

// ============================================================================
// DclDynamicGraphicsManager - Godot wrapper
// ============================================================================

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclDynamicGraphicsManager {
    base: Base<Node>,

    /// Pure state machine
    state: DynamicGraphicsState,

    /// Godot-specific fields
    has_ios_plugin: bool,
    has_android_plugin: bool,
    viewport_rid: Option<Rid>,
    render_time_enabled: bool,

    // === Thermal FPS Cap State ===
    /// Current thermal FPS cap (0 = no cap)
    thermal_fps_cap: i32,
    /// Timer tracking consecutive seconds of Normal thermal state
    thermal_fps_normal_timer: f64,
    /// Cooldown timer for FPS cap changes
    thermal_fps_cooldown_timer: f64,
    /// Whether device is currently charging
    is_charging: bool,
    /// Whether thermal FPS cap system is enabled
    thermal_fps_cap_enabled: bool,

    // === Platform polling throttle ===
    /// Timer for throttling JNI/platform calls
    platform_poll_timer: f64,
    /// Cached thermal state string from last platform poll
    cached_thermal_str: String,
}

#[godot_api]
impl INode for DclDynamicGraphicsManager {
    fn init(base: Base<Node>) -> Self {
        let has_ios_plugin = DclIosPlugin::is_available();
        let has_android_plugin = DclAndroidPlugin::is_available();

        Self {
            base,
            state: DynamicGraphicsState::new(),
            has_ios_plugin,
            has_android_plugin,
            viewport_rid: None,
            render_time_enabled: false,
            // Thermal FPS cap state
            thermal_fps_cap: THERMAL_FPS_NO_CAP,
            thermal_fps_normal_timer: 0.0,
            thermal_fps_cooldown_timer: 0.0,
            is_charging: false,
            thermal_fps_cap_enabled: true,
            // Platform polling throttle
            platform_poll_timer: PLATFORM_POLL_INTERVAL, // Start at limit to poll immediately on first frame
            cached_thermal_str: "nominal".to_string(),
        }
    }

    fn ready(&mut self) {
        if let Some(viewport) = self.base().get_viewport() {
            let rid = viewport.get_viewport_rid();
            self.viewport_rid = Some(rid);
            // Don't enable render time measurement yet - it will be enabled
            // when the manager transitions to an active state (WarmingUp/Monitoring/Cooldown).
            // GPU timestamp queries can hurt performance on mobile tile-based renderers.
            self.render_time_enabled = false;

            godot_print!(
                "[DynamicGraphics] ready: viewport acquired, render time measurement deferred (iOS={}, Android={})",
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
        let render_time_ms = self.get_total_render_time_ms();

        // Throttle platform calls (JNI on Android) to avoid per-frame overhead
        self.platform_poll_timer += delta;
        if self.platform_poll_timer >= PLATFORM_POLL_INTERVAL {
            self.platform_poll_timer = 0.0;
            let (thermal, charging) = self.get_platform_metrics();
            self.cached_thermal_str = thermal;
            self.is_charging = charging == "charging";
        }
        let thermal_str = self.cached_thermal_str.clone();

        // Process thermal FPS cap (independent of profile management)
        if self.thermal_fps_cap_enabled {
            self.process_thermal_fps_cap(delta, &thermal_str);
        }

        // Process profile management
        if let Some(action) = self
            .state
            .process_frame(delta, render_time_ms, &thermal_str)
        {
            let new_profile = match action {
                ProfileAction::Downgrade(p) => {
                    godot_print!("[DynamicGraphics] Profile downgrade to {}", p);
                    p
                }
                ProfileAction::Upgrade(p) => {
                    godot_print!("[DynamicGraphics] Profile upgrade to {}", p);
                    p
                }
            };
            self.base_mut()
                .emit_signal("profile_change_requested", &[new_profile.to_variant()]);
        }
    }
}

#[godot_api]
impl DclDynamicGraphicsManager {
    #[signal]
    fn profile_change_requested(new_profile: i32);

    #[signal]
    fn thermal_fps_cap_changed(fps_cap: i32);

    #[func]
    pub fn initialize(&mut self, enabled: bool, current_profile: i32, fps_limit: i32) {
        self.state.initialize(enabled, current_profile, fps_limit);
        self.set_render_time_measurement(self.state.is_active());
        godot_print!(
            "[DynamicGraphics] initialized: enabled={}, profile={}, target={}ms",
            enabled,
            current_profile,
            self.state.target_frame_time_ms
        );
    }

    #[func]
    pub fn on_fps_limit_changed(&mut self, fps_limit: i32) {
        self.state.on_fps_limit_changed(fps_limit);
        godot_print!(
            "[DynamicGraphics] FPS limit changed, target={}ms",
            self.state.target_frame_time_ms
        );
    }

    #[func]
    pub fn on_loading_started(&mut self) {
        self.state.on_loading_started();
        godot_print!("[DynamicGraphics] loading started");
    }

    #[func]
    pub fn on_loading_finished(&mut self) {
        self.state.on_loading_finished();
        godot_print!("[DynamicGraphics] loading finished");
    }

    #[func]
    pub fn on_manual_profile_change(&mut self, new_profile: i32) {
        self.state.on_manual_profile_change(new_profile);
        self.set_render_time_measurement(self.state.is_active());
    }

    #[func]
    pub fn set_enabled(&mut self, enabled: bool) {
        self.state.set_enabled(enabled);
        self.set_render_time_measurement(self.state.is_active());
        godot_print!("[DynamicGraphics] set_enabled({})", enabled);
    }

    #[func]
    pub fn is_gameplay_active(&self) -> bool {
        self.state.is_gameplay_active
    }

    #[func]
    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    #[func]
    pub fn is_enabled(&self) -> bool {
        self.state.is_enabled()
    }

    #[func]
    pub fn get_state_name(&self) -> GString {
        match self.state.state {
            ManagerState::Disabled => "Disabled".into(),
            ManagerState::WarmingUp => "WarmingUp".into(),
            ManagerState::Monitoring => "Monitoring".into(),
            ManagerState::Cooldown => "Cooldown".into(),
        }
    }

    #[func]
    pub fn get_state_string(&self) -> GString {
        match self.state.state {
            ManagerState::Disabled => "disabled".into(),
            ManagerState::WarmingUp => {
                GString::from(format!("warming_up ({:.0}s)", self.state.state_timer).as_str())
            }
            ManagerState::Monitoring => "monitoring".into(),
            ManagerState::Cooldown => {
                GString::from(format!("cooldown ({:.0}s)", self.state.state_timer).as_str())
            }
        }
    }

    #[func]
    pub fn get_warmup_remaining(&self) -> f64 {
        self.state.get_warmup_remaining()
    }

    #[func]
    pub fn get_cooldown_remaining(&self) -> f64 {
        self.state.get_cooldown_remaining()
    }

    #[func]
    pub fn get_current_profile(&self) -> i32 {
        self.state.current_profile
    }

    #[func]
    pub fn get_average_frame_time(&self) -> f64 {
        self.state.get_average_frame_time()
    }

    #[func]
    pub fn get_target_frame_time(&self) -> f64 {
        self.state.target_frame_time_ms
    }

    #[func]
    pub fn get_frame_time_ratio(&self) -> f64 {
        self.state.get_frame_time_ratio()
    }

    #[func]
    pub fn get_thermal_state_string(&self) -> GString {
        match self.state.thermal_state {
            ThermalState::Normal => "normal".into(),
            ThermalState::High => "high".into(),
            ThermalState::Critical => "critical".into(),
        }
    }

    #[func]
    pub fn get_debug_info(&self) -> GString {
        GString::from(
            format!(
                "state={:?}, timer={:.1}s, gameplay={}, samples={}",
                self.state.state,
                self.state.state_timer,
                self.state.is_gameplay_active,
                self.state.frame_time_samples.len()
            )
            .as_str(),
        )
    }

    // === Thermal FPS Cap Public API ===

    #[func]
    pub fn set_thermal_fps_cap_enabled(&mut self, enabled: bool) {
        let was_enabled = self.thermal_fps_cap_enabled;
        self.thermal_fps_cap_enabled = enabled;

        if !enabled && was_enabled && self.thermal_fps_cap != THERMAL_FPS_NO_CAP {
            self.apply_thermal_fps_cap(THERMAL_FPS_NO_CAP);
        }
        godot_print!("[DynamicGraphics] thermal FPS cap enabled={}", enabled);
    }

    #[func]
    pub fn is_thermal_fps_cap_enabled(&self) -> bool {
        self.thermal_fps_cap_enabled
    }

    #[func]
    pub fn get_thermal_fps_cap(&self) -> i32 {
        self.thermal_fps_cap
    }

    #[func]
    pub fn is_device_charging(&self) -> bool {
        self.is_charging
    }

    #[func]
    pub fn get_thermal_fps_cap_debug_info(&self) -> GString {
        GString::from(
            format!(
                "cap={}, normal_timer={:.1}s, cooldown={:.1}s, charging={}",
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
    fn set_render_time_measurement(&mut self, enabled: bool) {
        if self.render_time_enabled == enabled {
            return;
        }
        if let Some(rid) = self.viewport_rid {
            RenderingServer::singleton().viewport_set_measure_render_time(rid, enabled);
            self.render_time_enabled = enabled;
            godot_print!(
                "[DynamicGraphics] render time measurement {}",
                if enabled { "enabled" } else { "disabled" }
            );
        }
    }

    /// Get thermal and charging state from the platform in a single call
    fn get_platform_metrics(&self) -> (String, String) {
        if self.has_ios_plugin {
            return DclIosPlugin::get_thermal_and_charging_state();
        }
        if self.has_android_plugin {
            return DclAndroidPlugin::get_thermal_and_charging_state();
        }
        ("nominal".to_string(), "unknown".to_string())
    }

    fn get_total_render_time_ms(&self) -> f64 {
        if !self.render_time_enabled {
            return self.state.target_frame_time_ms;
        }

        if let Some(rid) = self.viewport_rid {
            let rs = RenderingServer::singleton();
            let cpu_time = rs.viewport_get_measured_render_time_cpu(rid);
            let gpu_time = rs.viewport_get_measured_render_time_gpu(rid);
            cpu_time.max(gpu_time)
        } else {
            self.state.target_frame_time_ms
        }
    }

    /// Process thermal FPS cap logic
    fn process_thermal_fps_cap(&mut self, delta: f64, thermal_str: &str) {
        // Update cooldown timer
        if self.thermal_fps_cooldown_timer > 0.0 {
            self.thermal_fps_cooldown_timer -= delta;
            if self.thermal_fps_cooldown_timer < 0.0 {
                self.thermal_fps_cooldown_timer = 0.0;
            }
        }

        // Calculate target cap based on thermal state and charging
        let target_cap = self.calculate_target_fps_cap(thermal_str);

        if target_cap != self.thermal_fps_cap {
            let is_downgrade = self.is_fps_cap_downgrade(self.thermal_fps_cap, target_cap);

            if is_downgrade {
                // Downgrade immediately
                self.apply_thermal_fps_cap(target_cap);
                self.thermal_fps_cooldown_timer = THERMAL_FPS_CHANGE_COOLDOWN;
                self.thermal_fps_normal_timer = 0.0;
            } else if self.thermal_fps_cooldown_timer <= 0.0 {
                // Upgrade requires sustained Normal thermal
                let is_normal = matches!(thermal_str, "nominal" | "fair");
                if is_normal {
                    self.thermal_fps_normal_timer += delta;
                    if self.thermal_fps_normal_timer >= THERMAL_FPS_NORMAL_DURATION_FOR_UPGRADE {
                        self.apply_thermal_fps_cap(target_cap);
                        self.thermal_fps_cooldown_timer = THERMAL_FPS_CHANGE_COOLDOWN;
                        self.thermal_fps_normal_timer = 0.0;
                    }
                } else {
                    self.thermal_fps_normal_timer = 0.0;
                }
            }
        } else {
            // Update normal timer
            let is_normal = matches!(thermal_str, "nominal" | "fair");
            if is_normal {
                self.thermal_fps_normal_timer += delta;
            } else {
                self.thermal_fps_normal_timer = 0.0;
            }
        }
    }

    /// Calculate target FPS cap based on thermal state and charging
    /// FPS Cap Rules:
    /// | Thermal  | Charging | Cap     |
    /// |----------|----------|---------|
    /// | Normal   | No       | No cap  |
    /// | Normal   | Yes      | 60 FPS  |
    /// | High     | No       | 45 FPS  |
    /// | High     | Yes      | 30 FPS  |
    /// | Critical | Any      | 30 FPS  |
    fn calculate_target_fps_cap(&self, thermal_str: &str) -> i32 {
        match thermal_str {
            "nominal" | "fair" => {
                if self.is_charging {
                    THERMAL_FPS_CAP_60
                } else {
                    THERMAL_FPS_NO_CAP
                }
            }
            "serious" => {
                if self.is_charging {
                    THERMAL_FPS_CAP_30
                } else {
                    THERMAL_FPS_CAP_45
                }
            }
            "critical" => THERMAL_FPS_CAP_30,
            _ => THERMAL_FPS_NO_CAP,
        }
    }

    fn is_fps_cap_downgrade(&self, current: i32, new: i32) -> bool {
        if current == THERMAL_FPS_NO_CAP {
            return new != THERMAL_FPS_NO_CAP;
        }
        if new == THERMAL_FPS_NO_CAP {
            return false;
        }
        new < current
    }

    fn apply_thermal_fps_cap(&mut self, new_cap: i32) {
        let old_cap = self.thermal_fps_cap;
        self.thermal_fps_cap = new_cap;

        self.base_mut()
            .emit_signal("thermal_fps_cap_changed", &[new_cap.to_variant()]);

        godot_print!(
            "[DynamicGraphics] Thermal FPS cap: {} -> {} (charging={})",
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
            self.is_charging
        );
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_state() -> DynamicGraphicsState {
        let mut state = DynamicGraphicsState::new();
        state.is_gameplay_active = true;
        state
    }

    #[test]
    fn test_warmup_to_monitoring() {
        let mut state = create_state();
        state.initialize(true, 2, 3); // enabled, Medium profile, 30fps

        assert_eq!(state.state, ManagerState::WarmingUp);

        // Simulate warmup duration
        for _ in 0..180 {
            state.process_frame(1.0, 20.0, "nominal");
        }

        assert_eq!(state.state, ManagerState::Monitoring);
    }

    #[test]
    fn test_downgrade_on_critical_thermal() {
        let mut state = create_state();
        state.initialize(true, 2, 3);

        // Skip warmup
        state.state = ManagerState::Monitoring;
        state.is_gameplay_active = true;

        let action = state.process_frame(0.016, 20.0, "critical");

        assert!(matches!(action, Some(ProfileAction::Downgrade(1))));
        assert_eq!(state.current_profile, 1);
        assert_eq!(state.state, ManagerState::Cooldown);
    }

    #[test]
    fn test_downgrade_on_sustained_high_thermal() {
        let mut state = create_state();
        state.initialize(true, 2, 3);
        state.state = ManagerState::Monitoring;

        // Simulate 30+ seconds of high thermal
        for _ in 0..31 {
            let action = state.process_frame(1.0, 20.0, "serious");
            if action.is_some() {
                assert!(matches!(action, Some(ProfileAction::Downgrade(_))));
                return;
            }
        }

        // Should have downgraded by now
        assert_eq!(state.current_profile, 1);
    }

    #[test]
    fn test_downgrade_on_high_frame_time() {
        let mut state = create_state();
        state.initialize(true, 2, 3); // 30fps = 33.3ms target
        state.state = ManagerState::Monitoring;

        // Simulate 60+ seconds of high frame times (>40ms, ratio > 1.2)
        for _ in 0..65 {
            let action = state.process_frame(1.0, 45.0, "nominal");
            if action.is_some() {
                assert!(matches!(action, Some(ProfileAction::Downgrade(_))));
                return;
            }
        }

        assert_eq!(state.current_profile, 1);
    }

    #[test]
    fn test_upgrade_conditions() {
        let mut state = create_state();
        state.initialize(true, 1, 3); // Low profile, 30fps
        state.state = ManagerState::Monitoring;

        // Simulate 120+ seconds of good performance (<16.6ms, ratio < 0.5)
        for _ in 0..125 {
            let action = state.process_frame(1.0, 10.0, "nominal");
            if action.is_some() {
                assert!(matches!(action, Some(ProfileAction::Upgrade(2))));
                return;
            }
        }

        assert_eq!(state.current_profile, 2);
    }

    #[test]
    fn test_cooldown_blocks_changes() {
        let mut state = create_state();
        state.initialize(true, 2, 3);
        state.state = ManagerState::Cooldown;
        state.current_cooldown_duration = 120.0;

        // During cooldown, no actions should be returned
        for _ in 0..60 {
            let action = state.process_frame(1.0, 50.0, "critical");
            assert!(action.is_none());
        }

        // Still in cooldown
        assert_eq!(state.state, ManagerState::Cooldown);
    }

    #[test]
    fn test_cooldown_completes() {
        let mut state = create_state();
        state.state = ManagerState::Cooldown;
        state.current_cooldown_duration = 120.0;
        state.is_gameplay_active = true;

        // Simulate full cooldown
        for _ in 0..121 {
            state.process_frame(1.0, 20.0, "nominal");
        }

        assert_eq!(state.state, ManagerState::Monitoring);
    }

    #[test]
    fn test_profile_bounds_min() {
        let mut state = create_state();
        state.initialize(true, 0, 3); // Already at Very Low
        state.state = ManagerState::Monitoring;

        // Try to downgrade - should not go below 0
        let action = state.process_frame(0.016, 20.0, "critical");
        assert!(action.is_none());
        assert_eq!(state.current_profile, 0);
    }

    #[test]
    fn test_profile_bounds_max() {
        let mut state = create_state();
        state.initialize(true, 3, 3); // Already at High
        state.state = ManagerState::Monitoring;

        // Fill samples for upgrade check
        for _ in 0..125 {
            state.process_frame(1.0, 10.0, "nominal");
        }

        // Should not go above High (3)
        assert_eq!(state.current_profile, 3);
    }

    #[test]
    fn test_custom_profile_disables() {
        let mut state = create_state();
        state.initialize(true, 2, 3);

        state.on_manual_profile_change(PROFILE_CUSTOM);

        assert_eq!(state.state, ManagerState::Disabled);
    }

    #[test]
    fn test_loading_pauses_monitoring() {
        let mut state = create_state();
        state.initialize(true, 2, 3);
        state.state = ManagerState::Monitoring;
        state.is_gameplay_active = true;

        state.on_loading_started();

        assert!(!state.is_gameplay_active);

        // Critical thermal should not trigger downgrade during loading
        let action = state.process_frame(0.016, 20.0, "critical");
        assert!(action.is_none());
    }
}
