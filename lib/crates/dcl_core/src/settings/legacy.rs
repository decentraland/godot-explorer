//! Reads the `[config]` section of Godot's `settings.cfg` (`key=value`, values in Godot text).

use std::collections::BTreeMap;

use super::{key::SettingKey, value::SettingValue};

/// The whitelisted keys of `[config]`; values of the wrong type or unreadable text are skipped.
pub fn parse(text: &str) -> BTreeMap<SettingKey, SettingValue> {
    let mut values = BTreeMap::new();
    let mut in_config = false;
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if let Some(section) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            in_config = section == "config";
            continue;
        }
        if !in_config {
            continue;
        }
        let Some((name, raw)) = line.split_once('=') else {
            continue;
        };
        let Some(key) = SettingKey::from_name(name.trim()) else {
            continue;
        };
        if let Some(value) = parse_value(raw.trim()).and_then(|v| v.coerce(key.kind())) {
            values.insert(key, value);
        }
    }
    values
}

fn parse_value(raw: &str) -> Option<SettingValue> {
    match raw {
        "true" => return Some(SettingValue::Bool(true)),
        "false" => return Some(SettingValue::Bool(false)),
        _ => {}
    }
    if let Some(quoted) = raw.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        return Some(SettingValue::Text(unescape(quoted)));
    }
    raw.parse::<i64>()
        .map(SettingValue::Int)
        .or_else(|_| raw.parse::<f64>().map(SettingValue::Float))
        .ok()
}

fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use SettingKey as K;
    use SettingValue as V;

    const SAMPLE: &str = r#"; Engine configuration file.
; It's best edited using the editor UI and not directly.

[config]

process_tick_quota_ms=10
limit_fps=4
graphic_profile=2
dynamic_graphics_enabled=false
local_content_dir="/data/user/0/org.decentraland.godotexplorer/files/content"
max_cache_size=2
locale="pt_BR"
dynamic_skybox=true
skybox_time=21600
submit_message_closes_chat=true
view_distance=200.0
particle_quality=2
audio_general_volume=80.0
audio_scene_volume=100
audio_music_volume=0.0

[user]

last_parcel_position=Vector2i(72, -10)
last_realm_joined="https://realm-provider.decentraland.org/main"
locale="xx"
last_places={
"a": 1,
"limit_fps=9": 2
}
"#;

    #[test]
    fn imports_the_config_section() {
        let v = parse(SAMPLE);
        assert_eq!(v[&K::LimitFps], V::Int(4));
        assert_eq!(v[&K::GraphicProfile], V::Int(2));
        assert_eq!(v[&K::DynamicGraphicsEnabled], V::Bool(false));
        assert_eq!(v[&K::MaxCacheSize], V::Int(2));
        assert_eq!(v[&K::Locale], V::Text("pt_BR".into()));
        assert_eq!(v[&K::SkyboxTime], V::Int(21600));
        assert_eq!(v[&K::SubmitMessageClosesChat], V::Bool(true));
        assert_eq!(v[&K::ViewDistance], V::Float(200.0));
        assert_eq!(v[&K::AudioGeneralVolume], V::Float(80.0));
        assert_eq!(v[&K::AudioSceneVolume], V::Float(100.0));
        assert_eq!(v[&K::AudioMusicVolume], V::Float(0.0));
        assert_eq!(v.len(), 13);
    }

    #[test]
    fn ignores_other_sections_and_unknown_keys() {
        let v = parse("[user]\nlocale=\"es\"\n[config]\nshow_fps=true\nbogus=1\n");
        assert!(v.is_empty());
    }

    #[test]
    fn skips_wrong_types_and_garbage() {
        let v = parse("[config]\nlimit_fps=\"fast\"\nview_distance=nan\nlocale=3\nskybox_time=\n=1\nnoequals\ngraphic_profile=1\n");
        assert_eq!(v.len(), 1);
        assert_eq!(v[&K::GraphicProfile], V::Int(1));
    }

    #[test]
    fn float_text_for_an_int_key_truncates() {
        assert_eq!(
            parse("[config]\nskybox_time=43200.0\n")[&K::SkyboxTime],
            V::Int(43200)
        );
    }

    #[test]
    fn unescapes_strings() {
        assert_eq!(
            parse_value(r#""a\"b\\c\n""#),
            Some(V::Text("a\"b\\c\n".into()))
        );
        assert_eq!(parse_value(r#""""#), Some(V::Text(String::new())));
    }

    #[test]
    fn tolerates_spaces_and_crlf() {
        let v = parse("[config]\r\nlimit_fps = 5\r\nlocale = \"es\"\r\n");
        assert_eq!(v[&K::LimitFps], V::Int(5));
        assert_eq!(v[&K::Locale], V::Text("es".into()));
    }
}
