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
use godot::classes::{object::ConnectFlags, Engine, SceneTree};
use godot::prelude::*;
use tokio::sync::oneshot;

use super::{root::call_static, DclEngine};
use crate::content::resource_provider::ResourceProvider;
use crate::godot_classes::dcl_global::DclGlobal;

const GRAPHICS: &str = "res://src/config/graphic_settings.gd";
const AUDIO: &str = "res://src/config/audio_settings.gd";
const GENERAL: &str = "res://src/config/general_settings.gd";
const LOCALE: &str = "res://src/config/locale_settings.gd";
const AVATAR_ANIM: &str = "res://src/decentraland_components/avatar/avatar_anim_helpers.gd";

/// The settings the dynamic graphics manager rewrites when it picks a profile.
const DYNAMIC_PROFILE_KEYS: [SettingKey; 4] = [
    SettingKey::GraphicProfile,
    SettingKey::ViewDistance,
    SettingKey::ParticleQuality,
    SettingKey::LimitFps,
];

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
pub(super) fn on_process_frame(name: &str, mut frame: impl FnMut() + 'static) {
    let Some(mut tree) = Engine::singleton()
        .get_main_loop()
        .and_then(|main_loop| main_loop.try_cast::<SceneTree>().ok())
    else {
        godot_error!("engine links: no scene tree for {name}");
        return;
    };
    let callable = Callable::from_fn(name, move |_| {
        frame();
        Variant::nil()
    });
    tree.connect("process_frame", &callable);
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
    adopt_dynamic_graphics();
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

fn to_variant(value: &SettingValue) -> Variant {
    match value {
        SettingValue::Bool(v) => v.to_variant(),
        SettingValue::Int(v) => v.to_variant(),
        SettingValue::Float(v) => v.to_variant(),
        SettingValue::Text(v) => v.to_variant(),
    }
}

/// The `AudioSettings` function that re-reads the key's volume from the config.
fn audio_apply_function(key: SettingKey) -> Option<&'static str> {
    Some(match key {
        SettingKey::AudioGeneralVolume => "apply_general_volume_settings",
        SettingKey::AudioSceneVolume => "apply_scene_volume_settings",
        SettingKey::AudioVoiceChatVolume => "apply_voice_chat_volume_settings",
        SettingKey::AudioUiVolume => "apply_ui_volume_settings",
        SettingKey::AudioMusicVolume => "apply_music_volume_settings",
        SettingKey::AudioAvatarAndEmotesVolume => "apply_avatar_and_emotes_volume_settings",
        SettingKey::AudioMicAmplification => "apply_mic_amplification_settings",
        _ => return None,
    })
}

/// Stores the value in the config and runs the same functions the settings page runs. Main thread.
fn apply_in_engine(key: SettingKey, value: &SettingValue) {
    let Some(global) = DclGlobal::try_singleton() else {
        godot_error!("settings_link: no global to apply {}", key.name());
        return;
    };
    // The locale is stored by `LocaleSettings.set_locale` itself.
    if key != SettingKey::Locale {
        global
            .bind()
            .get_config()
            .set(key.name(), &to_variant(value));
    }
    let int = match value {
        SettingValue::Int(v) => *v,
        _ => 0,
    };
    match (key, value) {
        (SettingKey::Locale, SettingValue::Text(locale)) => {
            call_static(LOCALE, "set_locale", &[locale.to_variant()]);
        }
        (SettingKey::GraphicProfile, _) => {
            // A custom profile index is a no-op there.
            call_static(GRAPHICS, "apply_graphic_profile", &[int.to_variant()]);
            let mut manager = global.bind().dynamic_graphics_manager.clone();
            manager.bind_mut().on_manual_profile_change(int as i32);
        }
        (SettingKey::DynamicGraphicsEnabled, SettingValue::Bool(enabled)) => {
            let mut manager = global.bind().dynamic_graphics_manager.clone();
            manager.bind_mut().set_enabled(*enabled);
        }
        (SettingKey::ViewDistance, SettingValue::Float(far)) => {
            if let Ok(mut camera) = global
                .get("player_camera_node")
                .try_to::<Gd<godot::classes::Camera3D>>()
            {
                camera.set_far(*far as f32);
            }
        }
        (SettingKey::ParticleQuality, _) => {
            call_static(
                AVATAR_ANIM,
                "apply_particles_enabled",
                &[(int > 0).to_variant()],
            );
            call_static(GRAPHICS, "apply_particle_quality", &[int.to_variant()]);
        }
        (SettingKey::LimitFps, _) => {
            call_static(GRAPHICS, "apply_fps_limit", &[]);
        }
        (SettingKey::MaxCacheSize, _) => {
            call_static(GENERAL, "apply_max_cache_size", &[]);
        }
        _ => {
            if let Some(function) = audio_apply_function(key) {
                call_static(AUDIO, function, &[]);
            }
        }
    }
}

/// Reports the profile the dynamic graphics manager picked: `global.gd` applies it to the config
/// first, and this deferred handler reads the result back into the core's store.
fn adopt_dynamic_graphics() {
    let Some(global) = DclGlobal::try_singleton() else {
        return;
    };
    let mut manager = global.bind().dynamic_graphics_manager.clone();
    let callable = Callable::from_fn("adopt_dynamic_graphics", |_| {
        adopt_config_values();
        Variant::nil()
    });
    manager.connect_flags(
        "profile_change_requested",
        &callable,
        ConnectFlags::DEFERRED,
    );
}

fn adopt_config_values() {
    let (Some(global), Some(core)) = (DclGlobal::try_singleton(), Core::try_get()) else {
        return;
    };
    let config = global.bind().get_config();
    let settings = settings::global(core.data_dir());
    for key in DYNAMIC_PROFILE_KEYS {
        let raw = config.get(key.name());
        let value = match key.kind() {
            SettingKind::Int => raw.try_to::<i64>().map(SettingValue::Int),
            _ => raw.try_to::<f64>().map(SettingValue::Float),
        };
        match value {
            Ok(value) => {
                if let Err(error) = settings.adopt(key, value) {
                    godot_warn!("settings_link: cannot adopt {}: {error}", key.name());
                }
            }
            Err(_) => godot_warn!("settings_link: {} is not a number", key.name()),
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
