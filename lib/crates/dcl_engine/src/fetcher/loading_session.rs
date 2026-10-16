use std::cell::Cell;

use super::globals;
use godot::classes::{INode, Time};
use godot::prelude::*;

use crate::app::audio_settings;
use crate::godot_classes::dcl_global::DclGlobal;

/// Hard maximum loading time, whatever the download activity.
const MAX_LOADING_TIME_SECONDS: f64 = 90.0;
const STALL_SECONDS: i64 = 20;
const CHECK_INTERVAL_SECONDS: f64 = 1.0;
const MAX_DOWNLOADS: i32 = 12;
const MAX_LOW_PRIORITY_DOWNLOADS: i32 = 2;

/// The engine's side of a world load: content limits, comms and audio while it runs, the
/// loading funnel metrics, the stall timeout, and the report that ends the core's Loading
/// state. The screen the player sees is the shell's.
#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct LoadingSession {
    base: Base<Node>,
    // Cells, and `&self` methods: starting and ending a load announces itself to handlers that
    // ask this node whether it is running, which a `&mut` bind would turn into a panic.
    running: Cell<bool>,
    start_msec: Cell<u64>,
    last_activity_msec: Cell<u64>,
    loaded_offset: Cell<i64>,
    last_percent: Cell<f32>,
    timeout_tracked: Cell<bool>,
    since_check: Cell<f64>,
}

#[godot_api]
impl INode for LoadingSession {
    fn ready(&mut self) {
        let mut runner = globals::scene_runner();
        let this = self.to_gd();
        runner.connect(
            "loading_started",
            &Callable::from_fn("loading_session_started", {
                let this = this.clone();
                move |_| {
                    this.bind().start();
                    Variant::nil()
                }
            }),
        );
        runner.connect(
            "loading_progress",
            &Callable::from_fn("loading_session_progress", {
                let this = this.clone();
                move |args| {
                    this.bind().on_progress(args[0].to::<f32>());
                    Variant::nil()
                }
            }),
        );
        runner.connect(
            "loading_complete",
            &Callable::from_fn("loading_session_complete", {
                let this = this.clone();
                move |_| {
                    this.bind().end("Success".into());
                    Variant::nil()
                }
            }),
        );
        runner.connect(
            "loading_timeout",
            &Callable::from_fn("loading_session_timeout", {
                let this = this.clone();
                move |_| {
                    godot_warn!("Loading session timed out");
                    this.bind().end("Success".into());
                    Variant::nil()
                }
            }),
        );
    }

    fn process(&mut self, delta: f64) {
        if !self.running.get() {
            return;
        }
        self.since_check.set(self.since_check.get() + delta);
        if self.since_check.get() >= CHECK_INTERVAL_SECONDS {
            self.since_check.set(0.0);
            self.check_stall();
        }
    }
}

fn now_msec() -> u64 {
    Time::singleton().get_ticks_msec()
}

fn track(event: &str, data: serde_json::Value) {
    globals::metrics()
        .bind_mut()
        .track_screen_viewed(event.to_string(), data.to_string());
}

fn set_audio(forced: Option<f32>) {
    for setting in [
        audio_settings::SCENE_VOLUME,
        audio_settings::VOICE_CHAT_VOLUME,
        audio_settings::AVATAR_AND_EMOTES_VOLUME,
    ] {
        audio_settings::apply_setting_with(setting, forced);
    }
}

#[godot_api]
impl LoadingSession {
    #[func]
    pub fn is_running(&self) -> bool {
        self.running.get()
    }

    /// `begin_episode` is false when the caller already opened the episode on the navigation
    /// intent; a second one would supersede it and lose what it was measuring.
    #[func]
    pub fn begin(
        &self,
        intended_realm: GString,
        when: GString,
        #[opt(default = true)] begin_episode: bool,
    ) {
        let realm = if intended_realm.is_empty() {
            globals::realm().bind().get_realm_string()
        } else {
            intended_realm
        };
        if begin_episode {
            globals::scene_runner()
                .bind_mut()
                .loading_begin_episode(when.clone(), realm.clone());
        }
        self.start();
        if !when.is_empty() {
            let position = current_position();
            track(
                "LOADING_START",
                serde_json::json!({
                    "position": format!("({}, {})", position.x, position.y),
                    "realm": realm.to_string(),
                    "when": when.to_string(),
                }),
            );
        }
    }

    /// The load is over without a scene: a realm change that failed, or the player backing out.
    #[func]
    pub fn end(&self, status: GString) {
        if !self.running.get() {
            return;
        }
        self.running.set(false);
        let mut global = DclGlobal::singleton();
        let mut provider = globals::content_provider();
        provider
            .bind_mut()
            .set_max_concurrent_downloads(MAX_DOWNLOADS);
        provider
            .bind_mut()
            .set_max_low_priority_downloads(MAX_DOWNLOADS);
        // Held back during the GP benchmark so two devices running the matrix never see each other.
        if !global.call("is_gp_benchmark", &[]).to::<bool>() {
            globals::comms().bind_mut().release_comms();
        }
        set_audio(None);
        global.emit_signal("loading_finished", &[]);
        globals::scene_runner()
            .bind_mut()
            .loading_end_episode("hidden".into());
        let position = current_position();
        track(
            "LOADING_END",
            serde_json::json!({
                "scene_id": current_scene_entity_id(),
                "position": format!("{},{}", position.x, position.y),
                "status": status.to_string(),
            }),
        );
        if let Some(explorer) = super::scene_fetcher::explorer() {
            let parcel = explorer.get("parcel_position").to::<Vector2i>();
            let realm = globals::realm().bind().get_realm_string();
            globals::engine()
                .bind()
                .report_loading_finished(realm, parcel.x, parcel.y);
        }
    }

    /// The player left while it was loading.
    #[func]
    pub fn cancel(&self) {
        if self.running.get() {
            self.running.set(false);
            globals::scene_runner()
                .bind_mut()
                .loading_end_episode("cancelled".into());
        }
    }
}

impl LoadingSession {
    fn start(&self) {
        let now = now_msec();
        self.last_activity_msec.set(now);
        let mut global = DclGlobal::singleton();
        let mut provider = globals::content_provider();
        if !self.running.get() {
            self.start_msec.set(now);
            self.loaded_offset
                .set(provider.bind().count_loaded_resources());
            self.timeout_tracked.set(false);
        }
        self.running.set(true);
        self.last_percent.set(0.0);
        self.since_check.set(0.0);
        provider
            .bind_mut()
            .set_max_concurrent_downloads(max_downloads());
        provider
            .bind_mut()
            .set_max_low_priority_downloads(MAX_LOW_PRIORITY_DOWNLOADS);
        // The scene room connects once loading finishes.
        globals::comms().bind_mut().hold_comms();
        // No voice, scene or avatar SFX (footsteps, jump/land) while loading.
        set_audio(Some(0.0));
        global.emit_signal("loading_started", &[]);
        global.call("release_mouse", &[]);
        globals::engine().bind().report_loading_progress(0.0);
    }

    fn on_progress(&self, percent: f32) {
        if !self.running.get() {
            return;
        }
        let percent = percent.clamp(0.0, 100.0);
        if percent != self.last_percent.get() {
            self.last_activity_msec.set(now_msec());
            self.last_percent.set(percent);
            globals::engine()
                .bind()
                .report_loading_progress(percent / 100.0);
        }
    }

    fn check_stall(&self) {
        if self.timeout_tracked.get() {
            return;
        }
        let global = DclGlobal::singleton();
        if globals::scene_runner().bind_mut().is_paused() {
            self.last_activity_msec.set(now_msec());
            return;
        }
        let provider = globals::content_provider();
        let speed = provider.bind().get_download_speed_mbs();
        // Only real network throughput counts as activity: a pending asset alone would defeat
        // the timeout on heavy scenes. Progress changes are counted in `on_progress`.
        if speed > 0.01 {
            self.last_activity_msec.set(now_msec());
        }
        let now = now_msec();
        let elapsed = (now - self.start_msec.get()) as f64 / 1000.0;
        let inactive = ((now - self.last_activity_msec.get()) / 1000) as i64;
        if inactive <= STALL_SECONDS && elapsed <= MAX_LOADING_TIME_SECONDS {
            return;
        }
        // Tests and the GP benchmark have their own cap; a timeout would cut their sample short.
        let mut global = global;
        let exempt = global.bind().testing_scene_mode
            || globals::cli().bind().scene_test_mode
            || global.call("is_gp_benchmark", &[]).to::<bool>();
        if exempt {
            return;
        }
        self.timeout_tracked.set(true);
        let loading = provider.bind().count_loading_resources() - self.loaded_offset.get();
        let loaded = provider.bind().count_loaded_resources() - self.loaded_offset.get();
        track(
            "LOADING_TIMEOUT",
            serde_json::json!({
                "loaded_resources": loaded,
                "loading_resources": loading,
                "download_speed_mbs": speed,
                "scene_id": current_scene_entity_id(),
                "inactive_seconds": inactive,
                "total_elapsed_seconds": elapsed as i64,
                "timeout_reason": if elapsed > MAX_LOADING_TIME_SECONDS { "max_time" } else { "inactivity" },
            }),
        );
    }
}

fn fetcher() -> Option<Gd<super::scene_fetcher::SceneFetcher>> {
    DclGlobal::singleton()
        .get("scene_fetcher")
        .try_to::<Gd<super::scene_fetcher::SceneFetcher>>()
        .ok()
}

fn current_position() -> Vector2i {
    fetcher().map_or(Vector2i::ZERO, |f| f.bind().get_current_position())
}

fn current_scene_entity_id() -> String {
    fetcher()
        .map(|f| f.bind().get_current_scene_entity_id().to_string())
        .unwrap_or_default()
}

/// `max-downloads=<n>` (deeplink) overrides the default 12, for benchmarking the download stage.
fn max_downloads() -> i32 {
    DclGlobal::singleton()
        .get("deep_link_obj")
        .try_to::<Gd<Object>>()
        .ok()
        .and_then(|link| {
            link.get("params")
                .try_to::<VarDictionary>()
                .ok()?
                .get("max-downloads")
                .map(|v| v.stringify().to_string())
        })
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(MAX_DOWNLOADS)
}
