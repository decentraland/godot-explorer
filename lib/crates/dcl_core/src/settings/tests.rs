use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use super::*;
use SettingKey as K;
use SettingValue as V;

#[derive(Default)]
struct Recorder {
    applied: Mutex<Vec<(SettingKey, SettingValue)>>,
    announced: Mutex<Vec<SettingEntry>>,
}

impl EngineApplier for Recorder {
    fn apply(&self, key: SettingKey, value: &SettingValue) {
        self.applied.lock().unwrap().push((key, value.clone()));
    }
}

impl SettingsListener for Recorder {
    fn settings_changed(&self, entry: &SettingEntry) {
        self.announced.lock().unwrap().push(entry.clone());
    }
}

struct FakeCache(Mutex<u64>);

#[async_trait]
impl ContentCache for FakeCache {
    fn total_bytes(&self) -> u64 {
        *self.0.lock().unwrap()
    }
    async fn clear(&self) {
        *self.0.lock().unwrap() = 0;
    }
}

fn open() -> (tempfile::TempDir, Settings, Arc<Recorder>) {
    let dir = tempfile::tempdir().unwrap();
    let settings = Settings::open(dir.path());
    let recorder = Arc::new(Recorder::default());
    settings.set_applier(recorder.clone());
    settings.add_listener(recorder.clone());
    (dir, settings, recorder)
}

fn reopen(dir: &tempfile::TempDir) -> Settings {
    Settings::open(dir.path())
}

#[test]
fn starts_with_every_default() {
    let (_dir, settings, _) = open();
    let entries = settings.entries();
    assert_eq!(entries.len(), 18, "the shell lists its 18 keys only");
    assert!(entries.iter().all(|entry| entry.key.shell_visible()));
    for key in SettingKey::ALL {
        assert_eq!(settings.get(key), key.default_value(), "{}", key.name());
    }
}

#[test]
fn set_coerces_stores_applies_and_announces() {
    let (_dir, settings, rec) = open();
    let out = settings
        .set(K::SkyboxTime, V::Float(21600.7), false)
        .unwrap();
    assert_eq!(
        out,
        vec![SettingEntry {
            key: K::SkyboxTime,
            value: V::Int(21600)
        }]
    );
    assert_eq!(settings.get(K::SkyboxTime), V::Int(21600));
    assert_eq!(*rec.announced.lock().unwrap(), out);

    settings
        .set(K::AudioMusicVolume, V::Int(40), false)
        .unwrap();
    let applied = rec.applied.lock().unwrap();
    assert_eq!(applied.as_slice(), [(K::AudioMusicVolume, V::Float(40.0))]);
}

#[test]
fn rejects_wrong_types_without_side_effects() {
    let (_dir, settings, rec) = open();
    assert_eq!(
        settings.set(K::LimitFps, V::Text("30".into()), true),
        Err(SettingsError::InvalidValue(K::LimitFps))
    );
    assert_eq!(
        settings.set(K::DynamicSkybox, V::Int(1), true),
        Err(SettingsError::InvalidValue(K::DynamicSkybox))
    );
    assert_eq!(settings.get(K::LimitFps), K::LimitFps.default_value());
    assert!(rec.applied.lock().unwrap().is_empty());
    assert!(rec.announced.lock().unwrap().is_empty());
}

#[test]
fn keys_without_engine_work_are_not_applied() {
    let (_dir, settings, rec) = open();
    settings
        .set(K::SubmitMessageClosesChat, V::Bool(true), false)
        .unwrap();
    settings
        .set(K::DynamicSkybox, V::Bool(false), false)
        .unwrap();
    settings.set(K::SkyboxTime, V::Int(86400), false).unwrap();
    assert!(rec.applied.lock().unwrap().is_empty());
    assert_eq!(rec.announced.lock().unwrap().len(), 3);
}

#[test]
fn uncommitted_values_stay_in_memory_until_a_commit() {
    let (dir, settings, _) = open();
    settings
        .set(K::ViewDistance, V::Float(120.0), false)
        .unwrap();
    assert_eq!(reopen(&dir).get(K::ViewDistance), V::Float(300.0));

    settings.set(K::LimitFps, V::Int(5), true).unwrap();
    let again = reopen(&dir);
    assert_eq!(again.get(K::LimitFps), V::Int(5));
    assert_eq!(again.get(K::ViewDistance), V::Float(120.0));
}

#[test]
fn locale_always_saves_and_rejects_unsupported() {
    let (dir, settings, _) = open();
    settings
        .set(K::Locale, V::Text("es".into()), false)
        .unwrap();
    assert_eq!(reopen(&dir).get(K::Locale), V::Text("es".into()));
    assert_eq!(
        settings.set(K::Locale, V::Text("fr".into()), true),
        Err(SettingsError::UnsupportedLocale("fr".into()))
    );
    assert_eq!(settings.get(K::Locale), V::Text("es".into()));
    settings
        .set(K::Locale, V::Text(String::new()), false)
        .unwrap();
}

#[test]
fn pseudolocale_only_outside_production() {
    let (_dir, settings, _) = open();
    let build = |production| BuildInfo {
        short_version: "1.0.0".into(),
        full_version: "1.0.0-x".into(),
        production,
    };
    settings.set_build(build(true));
    assert!(settings
        .set(K::Locale, V::Text("xx-pseudo".into()), false)
        .is_err());
    settings.set_build(build(false));
    assert!(settings
        .set(K::Locale, V::Text("xx-pseudo".into()), false)
        .is_ok());
}

#[test]
fn profile_rewrites_and_reannounces_its_dependents() {
    let (_dir, settings, rec) = open();
    let out = settings.set(K::GraphicProfile, V::Int(1), false).unwrap();
    let keys: Vec<_> = out.iter().map(|e| e.key).collect();
    assert_eq!(
        keys,
        [
            K::GraphicProfile,
            K::ViewDistance,
            K::ParticleQuality,
            K::LimitFps
        ]
    );
    assert_eq!(settings.get(K::ViewDistance), V::Float(150.0));
    assert_eq!(settings.get(K::ParticleQuality), V::Int(1));
    assert_eq!(settings.get(K::LimitFps), V::Int(3));
    assert_eq!(*rec.announced.lock().unwrap(), out);
    assert_eq!(
        rec.applied.lock().unwrap().as_slice(),
        [(K::GraphicProfile, V::Int(1))]
    );
}

#[test]
fn custom_profile_keeps_the_values_but_still_announces_them() {
    let (_dir, settings, _) = open();
    settings
        .set(K::ViewDistance, V::Float(111.0), false)
        .unwrap();
    let out = settings.set(K::GraphicProfile, V::Int(4), false).unwrap();
    assert_eq!(out.len(), 4);
    assert_eq!(
        out[1],
        SettingEntry {
            key: K::ViewDistance,
            value: V::Float(111.0)
        }
    );
}

#[test]
fn adopt_stores_and_announces_without_applying_or_saving() {
    let (dir, settings, rec) = open();
    let out = settings.adopt(K::GraphicProfile, V::Int(0)).unwrap();
    assert_eq!(out.len(), 4);
    assert_eq!(settings.get(K::ViewDistance), V::Float(100.0));
    assert!(rec.applied.lock().unwrap().is_empty());
    assert_eq!(reopen(&dir).get(K::GraphicProfile), V::Int(3));
    assert!(settings.adopt(K::GraphicProfile, V::Bool(true)).is_err());
}

#[test]
fn apply_all_skips_inert_keys() {
    let (_dir, settings, rec) = open();
    settings.apply_all();
    let applied = rec.applied.lock().unwrap();
    assert_eq!(
        applied.len(),
        SettingKey::ALL.iter().filter(|k| k.needs_apply()).count()
    );
    assert!(applied.contains(&(K::ViewDistance, V::Float(300.0))));
    assert!(!applied.iter().any(|(k, _)| *k == K::SkyboxTime));
}

#[test]
fn legacy_cfg_is_imported_on_open() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = "[config]\nlimit_fps=4\ngraphic_profile=2\nlocale=\"es\"\nview_distance=200.0\n";
    std::fs::write(dir.path().join("settings.cfg"), cfg).unwrap();
    let settings = Settings::open(dir.path());
    assert_eq!(settings.get(K::GraphicProfile), V::Int(2));
    assert_eq!(settings.get(K::LimitFps), V::Int(4));
    assert_eq!(settings.get(K::Locale), V::Text("es".into()));
    assert_eq!(settings.get(K::AudioGeneralVolume), V::Float(100.0));
    assert!(dir.path().join("settings.json").exists());
    assert!(dir.path().join("settings.cfg").exists());
}

#[test]
fn removed_listener_is_not_called() {
    let (_dir, settings, rec) = open();
    let other = Arc::new(Recorder::default());
    let id = settings.add_listener(other.clone());
    settings.remove_listener(id);
    settings
        .set(K::DynamicSkybox, V::Bool(false), false)
        .unwrap();
    assert!(other.announced.lock().unwrap().is_empty());
    assert_eq!(rec.announced.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn cache_size_and_clear() {
    let (_dir, settings, _) = open();
    assert_eq!(
        settings.cache_size(),
        CacheSize {
            bytes: 0,
            max_bytes: 2_048_000_000
        }
    );
    assert_eq!(
        settings.clear_cache().await,
        Err(SettingsError::EngineUnavailable)
    );

    settings.set_content_cache(Arc::new(FakeCache(Mutex::new(123_456))));
    settings.set(K::MaxCacheSize, V::Int(2), false).unwrap();
    assert_eq!(
        settings.cache_size(),
        CacheSize {
            bytes: 123_456,
            max_bytes: 4_096_000_000
        }
    );
    assert_eq!(settings.clear_cache().await, Ok(0));
    assert_eq!(settings.cache_size().bytes, 0);

    settings.set(K::MaxCacheSize, V::Int(40), false).unwrap();
    assert_eq!(settings.cache_size().max_bytes, 4_096_000_000);
}

#[test]
fn version_needs_the_build() {
    let (_dir, settings, _) = open();
    assert_eq!(settings.version(), Err(SettingsError::EngineUnavailable));
    settings.set_build(BuildInfo {
        short_version: "1.14.0.1844".into(),
        full_version: "1.14.0.1844-6cefb54-staging".into(),
        production: true,
    });
    assert_eq!(settings.version().unwrap(), "1.14.0");
}

#[test]
fn engine_state_is_stored_and_saved_but_never_applied_or_announced() {
    let (dir, settings, rec) = open();
    settings
        .set(K::LastRealmJoined, V::Text("https://realm".into()), true)
        .unwrap();
    settings.set(K::LastParcelX, V::Float(12.0), true).unwrap();
    settings.adopt(K::BenchmarkGpuScore, V::Float(7.5)).unwrap();
    settings
        .adopt(K::FirstLaunchCompleted, V::Bool(true))
        .unwrap();

    assert!(rec.applied.lock().unwrap().is_empty());
    assert!(rec.announced.lock().unwrap().is_empty());
    assert!(settings
        .entries()
        .iter()
        .all(|entry| entry.key.shell_visible()));

    settings.save();
    let again = reopen(&dir);
    assert_eq!(
        again.get(K::LastRealmJoined),
        V::Text("https://realm".into())
    );
    assert_eq!(
        again.get(K::LastParcelX),
        V::Int(12),
        "a float truncates into an int key"
    );
    assert_eq!(again.get(K::BenchmarkGpuScore), V::Float(7.5));
    assert_eq!(again.get(K::FirstLaunchCompleted), V::Bool(true));
    assert_eq!(again.get(K::LastParcelY), V::Int(-10), "the default");
}
