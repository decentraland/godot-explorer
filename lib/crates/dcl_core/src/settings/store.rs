//! `settings.json` in the data dir, and the one-time import of the legacy `settings.cfg`.

use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};

use super::{key::SettingKey, legacy, value::SettingValue};

const FILE_NAME: &str = "settings.json";
const LEGACY_FILE_NAME: &str = "settings.cfg";
const FORMAT_VERSION: u32 = 1;

pub(super) type Values = BTreeMap<SettingKey, SettingValue>;

pub(super) struct SettingsStore {
    path: PathBuf,
    legacy_path: PathBuf,
}

impl SettingsStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join(FILE_NAME),
            legacy_path: data_dir.join(LEGACY_FILE_NAME),
        }
    }

    /// The saved values (defaults fill the rest). With no readable `settings.json`, the legacy
    /// file is imported and written out, so the import happens once; `settings.cfg` stays.
    pub fn load(&self) -> Values {
        if let Some(values) = self.read_json() {
            return values;
        }
        let Ok(text) = fs::read_to_string(&self.legacy_path) else {
            return Values::new();
        };
        let values = legacy::parse(&text);
        if let Err(error) = self.save(&values) {
            tracing::error!("settings: could not write {}: {error}", self.path.display());
        }
        values
    }

    fn read_json(&self) -> Option<Values> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
            Err(error) => {
                tracing::error!("settings: could not read {}: {error}", self.path.display());
                return None;
            }
        };
        let Ok(Value::Object(root)) = serde_json::from_str::<Value>(&text) else {
            tracing::error!(
                "settings: {} is not valid, ignoring it",
                self.path.display()
            );
            return None;
        };
        let stored = root.get("values").and_then(Value::as_object)?;
        Some(
            stored
                .iter()
                .filter_map(|(name, raw)| {
                    let key = SettingKey::from_name(name)?;
                    let value = SettingValue::from_json(raw)?.coerce(key.kind())?;
                    Some((key, value))
                })
                .collect(),
        )
    }

    pub fn save(&self, values: &Values) -> std::io::Result<()> {
        let map: serde_json::Map<String, Value> = values
            .iter()
            .map(|(key, value)| (key.name().to_string(), value.to_json()))
            .collect();
        let body = json!({ "version": FORMAT_VERSION, "values": map });
        let tmp = self.path.with_extension("json.tmp");
        let mut file = fs::File::create(&tmp)?;
        file.write_all(serde_json::to_string_pretty(&body)?.as_bytes())?;
        file.sync_all()?;
        fs::rename(&tmp, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use SettingKey as K;
    use SettingValue as V;

    fn store() -> (tempfile::TempDir, SettingsStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(dir.path());
        (dir, store)
    }

    #[test]
    fn empty_dir_loads_nothing_and_writes_nothing() {
        let (dir, store) = store();
        assert!(store.load().is_empty());
        assert!(!dir.path().join("settings.json").exists());
    }

    #[test]
    fn round_trip() {
        let (_dir, store) = store();
        let values = Values::from([
            (K::Locale, V::Text("es".into())),
            (K::ViewDistance, V::Float(200.0)),
            (K::LimitFps, V::Int(4)),
            (K::DynamicSkybox, V::Bool(false)),
        ]);
        store.save(&values).unwrap();
        assert_eq!(store.load(), values);
    }

    #[test]
    fn imports_the_legacy_file_once_and_leaves_it() {
        let (dir, store) = store();
        let cfg = "[config]\nlimit_fps=5\nlocale=\"es\"\n";
        fs::write(dir.path().join("settings.cfg"), cfg).unwrap();
        let first = store.load();
        assert_eq!(first[&K::LimitFps], V::Int(5));
        assert_eq!(
            fs::read_to_string(dir.path().join("settings.cfg")).unwrap(),
            cfg
        );

        fs::write(dir.path().join("settings.cfg"), "[config]\nlimit_fps=2\n").unwrap();
        assert_eq!(store.load(), first);
    }

    #[test]
    fn json_wins_over_the_legacy_file() {
        let (dir, store) = store();
        store
            .save(&Values::from([(K::LimitFps, V::Int(2))]))
            .unwrap();
        fs::write(dir.path().join("settings.cfg"), "[config]\nlimit_fps=5\n").unwrap();
        assert_eq!(store.load()[&K::LimitFps], V::Int(2));
    }

    #[test]
    fn corrupt_json_falls_back_to_the_legacy_file() {
        let (dir, store) = store();
        fs::write(dir.path().join("settings.json"), "{not json").unwrap();
        fs::write(dir.path().join("settings.cfg"), "[config]\nlimit_fps=5\n").unwrap();
        assert_eq!(store.load()[&K::LimitFps], V::Int(5));
    }

    #[test]
    fn drops_unknown_and_mistyped_entries() {
        let (dir, store) = store();
        let body = r#"{"version":1,"values":{"limit_fps":3.0,"locale":5,"camera_mode":"first","view_distance":150}}"#;
        fs::write(dir.path().join("settings.json"), body).unwrap();
        let values = store.load();
        assert_eq!(values.len(), 2);
        assert_eq!(values[&K::LimitFps], V::Int(3));
        assert_eq!(values[&K::ViewDistance], V::Float(150.0));
    }
}
