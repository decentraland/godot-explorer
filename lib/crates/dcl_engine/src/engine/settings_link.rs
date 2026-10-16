//! Engine side of the core's settings contract: the applier, the content cache and the build
//! info. The core calls them from any thread; everything that touches Godot is queued here and
//! run on the main thread by a `process_frame` pump, which the other links reuse.

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};

use async_trait::async_trait;
use dcl_core::settings::{
    self, BuildInfo, ContentCache, EngineApplier, SettingKey, SettingKind, SettingValue,
};
use dcl_core::Core;
use godot::classes::{Engine, SceneTree};
use godot::prelude::*;
use tokio::sync::oneshot;

use super::DclEngine;
use crate::app::{audio_settings, general_settings, graphics, locale};
use crate::content::resource_provider::ResourceProvider;
use crate::godot_classes::dcl_global::DclGlobal;

type Task = Box<dyn FnOnce() + Send>;

static TASKS: Mutex<Vec<Task>> = Mutex::new(Vec::new());
static REGISTERED: AtomicBool = AtomicBool::new(false);

/// Queues `task` for the main thread's next frame. Safe from any thread; tasks run in order.
pub(super) fn run_on_main(task: impl FnOnce() + Send + 'static) {
    TASKS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(Box::new(task));
}

fn drain_main_tasks() {
    let tasks = std::mem::take(&mut *TASKS.lock().unwrap_or_else(|e| e.into_inner()));
    for task in tasks {
        task();
    }
}

/// Runs `frame` on every `process_frame` of the main loop. Main thread only.
pub(super) fn on_process_frame(name: &str, frame: impl FnMut() + 'static) {
    let Some(tree) = Engine::singleton()
        .get_main_loop()
        .and_then(|main_loop| main_loop.try_cast::<SceneTree>().ok())
    else {
        godot_error!("engine links: no scene tree for {name}");
        return;
    };
    tree.signals().process_frame().connect(frame);
}

/// Registers the settings contracts with the core. Called once when the engine starts.
pub(super) fn register() {
    if REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }
    on_process_frame("engine_main_tasks", drain_main_tasks);
    let Some(core) = Core::try_get() else {
        godot_error!("settings_link: no core to register with");
        return;
    };
    let settings = settings::global(core.data_dir());
    settings.set_build(build_info());
    settings.set_applier(Arc::new(MainThreadApplier));
    match engine_cache() {
        Some(cache) => settings.set_content_cache(Arc::new(cache)),
        None => godot_error!("settings_link: no content provider for the cache contract"),
    }
    // Without the host the legacy GDScript settings own the config; the core's store would
    // overwrite it with defaults.
    if DclEngine::core_is_driving() {
        settings.apply_all();
    }
}

fn build_info() -> BuildInfo {
    BuildInfo {
        short_version: DclGlobal::get_short_version().to_string(),
        full_version: DclGlobal::get_full_version().to_string(),
        production: DclGlobal::is_production(),
    }
}

struct MainThreadApplier;

impl EngineApplier for MainThreadApplier {
    fn apply(&self, key: SettingKey, value: &SettingValue) {
        let value = value.clone();
        run_on_main(move || apply_in_engine(key, &value));
    }
}

fn store() -> Option<&'static settings::Settings> {
    Core::try_get().map(|core| settings::global(core.data_dir()))
}

/// The value of an engine-read setting; `None` without a core.
pub(crate) fn value(key: SettingKey) -> Option<SettingValue> {
    store().map(|settings| settings.get(key))
}

pub(crate) fn texture_quality_setting() -> crate::content::texture_quality::TextureQuality {
    use crate::content::texture_quality::TextureQuality;
    match value(SettingKey::TextureQuality) {
        Some(SettingValue::Int(quality)) => TextureQuality::from_i32(quality as i32),
        _ => TextureQuality::Medium,
    }
}

pub(crate) fn benchmark_gpu_score() -> Option<f64> {
    match value(SettingKey::BenchmarkGpuScore) {
        Some(SettingValue::Float(score)) => Some(score),
        _ => None,
    }
}

/// What a GDScript value becomes for `key`: the key's own type, numbers converted.
fn from_variant(key: SettingKey, value: &Variant) -> Option<SettingValue> {
    match key.kind() {
        SettingKind::Bool => value.try_to::<bool>().ok().map(SettingValue::Bool),
        SettingKind::Int => value
            .try_to::<i64>()
            .ok()
            .or_else(|| value.try_to::<f64>().ok().map(|f| f as i64))
            .map(SettingValue::Int),
        SettingKind::Float => value
            .try_to::<f64>()
            .ok()
            .or_else(|| value.try_to::<i64>().ok().map(|i| i as f64))
            .map(SettingValue::Float),
        SettingKind::Text => value
            .try_to::<GString>()
            .ok()
            .map(|text| SettingValue::Text(text.to_string())),
    }
}

/// `Global.engine.setting`: the value as a Godot variant, nil for an unknown name.
pub(crate) fn get_variant(name: &str) -> Variant {
    let Some(key) = SettingKey::from_name(name) else {
        godot_error!("setting: unknown key {name}");
        return Variant::nil();
    };
    value(key).map_or_else(Variant::nil, |value| to_variant(&value))
}

/// Tells the engine's listeners (the environment, ...) that `name` changed. Main thread.
fn notify_changed(name: &str) {
    let Some(global) = DclGlobal::try_singleton() else {
        return;
    };
    let engine = global.bind().get_engine();
    engine
        .signals()
        .setting_changed()
        .emit(&GString::from(name));
}

/// Stores a value the player or the engine chose. `adopt` skips applying and saving (the engine
/// already did the work, e.g. a graphic profile); otherwise it applies and `commit` saves.
pub(super) fn set_variant(name: &str, value: &Variant, commit: bool, adopt: bool) {
    let (Some(key), Some(settings)) = (SettingKey::from_name(name), store()) else {
        godot_error!("setting: cannot set {name}");
        return;
    };
    let Some(value) = from_variant(key, value) else {
        godot_error!("setting: {name} does not take {value}");
        return;
    };
    let result = if adopt {
        settings.adopt(key, value)
    } else {
        settings.set(key, value, commit)
    };
    match result {
        Ok(_) => notify_changed(name),
        Err(error) => godot_warn!("setting: {name}: {error}"),
    }
}

pub(super) fn save() {
    if let Some(settings) = store() {
        settings.save();
    }
}

fn to_variant(value: &SettingValue) -> Variant {
    match value {
        SettingValue::Bool(v) => v.to_variant(),
        SettingValue::Int(v) => v.to_variant(),
        SettingValue::Float(v) => v.to_variant(),
        SettingValue::Text(v) => v.to_variant(),
    }
}

/// The volume setting a key re-reads into its audio bus. The mic amplification has no bus: it is
/// read where the microphone is.
fn audio_apply_function(key: SettingKey) -> Option<&'static str> {
    Some(match key {
        SettingKey::AudioGeneralVolume => "audio_general_volume",
        SettingKey::AudioSceneVolume => "audio_scene_volume",
        SettingKey::AudioVoiceChatVolume => "audio_voice_chat_volume",
        SettingKey::AudioUiVolume => "audio_ui_volume",
        SettingKey::AudioMusicVolume => "audio_music_volume",
        SettingKey::AudioAvatarAndEmotesVolume => "audio_avatar_and_emotes_volume",
        SettingKey::AudioMicAmplification => "audio_mic_amplification",
        _ => return None,
    })
}

/// Stores the value in the config and runs the same functions the settings page runs. Main thread.
fn apply_in_engine(key: SettingKey, value: &SettingValue) {
    let Some(global) = DclGlobal::try_singleton() else {
        godot_error!("settings_link: no global to apply {}", key.name());
        return;
    };
    notify_changed(key.name());
    let int = match value {
        SettingValue::Int(v) => *v,
        _ => 0,
    };
    match (key, value) {
        (SettingKey::Locale, SettingValue::Text(_)) => {
            locale::apply_locale();
        }
        (SettingKey::GraphicProfile, _) => {
            // A custom profile index is a no-op there.
            graphics::apply_graphic_profile(int as i32);
            let mut manager = global.bind().dynamic_graphics_manager.clone();
            manager.bind_mut().on_manual_profile_change(int as i32);
        }
        (SettingKey::DynamicGraphicsEnabled, SettingValue::Bool(enabled)) => {
            let mut manager = global.bind().dynamic_graphics_manager.clone();
            manager.bind_mut().set_enabled(*enabled);
        }
        (SettingKey::ViewDistance, SettingValue::Float(far)) => {
            if let Some(mut camera) = global.bind().player_camera() {
                camera.set_far(*far as f32);
            }
        }
        (SettingKey::ParticleQuality, _) => {
            crate::avatars::avatar::lod::apply_particles_enabled(int > 0);
            graphics::apply_particle_quality(int as i32);
        }
        (SettingKey::LimitFps, _) => {
            let mut manager = global.bind().dynamic_graphics_manager.clone();
            manager.bind_mut().on_fps_limit_changed(int as i32);
            let thermal_cap = manager.bind().get_thermal_fps_cap();
            graphics::apply_fps_limit_with_thermal_cap(int as i32, thermal_cap);
        }
        (SettingKey::MaxCacheSize, _) => {
            general_settings::apply_max_cache_size();
        }
        _ => {
            if let Some(setting) = audio_apply_function(key) {
                audio_settings::apply_setting(setting);
            }
        }
    }
}

fn engine_cache() -> Option<EngineCache> {
    let global = DclGlobal::try_singleton()?;
    let provider = global.bind().content_provider.clone();
    let resources = provider.bind().get_resource_provider();
    Some(EngineCache {
        resources,
        last_total: AtomicU64::new(0),
    })
}

/// The content cache folder, read through the thread-safe `ResourceProvider`.
struct EngineCache {
    resources: Arc<ResourceProvider>,
    /// Answer while the index is locked by a writer.
    last_total: AtomicU64,
}

#[async_trait]
impl ContentCache for EngineCache {
    fn total_bytes(&self) -> u64 {
        match self.resources.try_cache_total_size() {
            Some(total) => {
                let total = total.max(0) as u64;
                self.last_total.store(total, Ordering::Relaxed);
                total
            }
            None => self.last_total.load(Ordering::Relaxed),
        }
    }

    async fn clear(&self) {
        // The content provider drops the promises that point into the folder; that is Godot state.
        let (done, dropped) = oneshot::channel();
        run_on_main(move || {
            if let Some(global) = DclGlobal::try_singleton() {
                let mut provider = global.bind().content_provider.clone();
                let promise = provider.bind_mut().clear_cache_folder();
                promise.free();
            }
            let _ = done.send(());
        });
        let _ = dropped.await;
        self.resources.clear().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_audio_key_has_an_apply_function() {
        let audio = SettingKey::ALL
            .into_iter()
            .filter(|key| key.name().starts_with("audio_"));
        for key in audio {
            assert!(audio_apply_function(key).is_some(), "{}", key.name());
        }
        assert_eq!(audio_apply_function(SettingKey::LimitFps), None);
    }
}

mod itests {
    use super::*;
    use crate::framework::TestContext;

    #[godot::test::itest]
    fn registering_gives_the_core_the_build_and_the_screenshot(_ctx: &TestContext) {
        register();
        super::super::screenshot_link::register();
        let core = Core::try_get().expect("the tests run with a core");
        assert!(settings::global(core.data_dir()).version().is_ok());
        assert!(dcl_core::scene::screenshot_provider().is_some());
    }

    #[godot::test::itest]
    fn tasks_queued_from_other_threads_run_in_order_on_the_drain(_ctx: &TestContext) {
        let order = Arc::new(Mutex::new(Vec::new()));
        for n in 0..3 {
            let order = order.clone();
            std::thread::spawn(move || run_on_main(move || order.lock().unwrap().push(n)))
                .join()
                .unwrap();
        }
        assert!(order.lock().unwrap().is_empty());
        drain_main_tasks();
        assert_eq!(*order.lock().unwrap(), vec![0, 1, 2]);
    }
}
