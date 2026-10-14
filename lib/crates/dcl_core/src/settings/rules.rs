//! The rules around the stored values: graphic profiles, cache sizing and the version shown.

use super::{engine::BuildInfo, key::SettingKey, value::SettingValue};

pub const LOCALES: [&str; 3] = ["en", "es", "pt_BR"];
pub const PSEUDO_LOCALE: &str = "xx-pseudo";

const BYTES_PER_MB: u64 = 1000 * 1000;
const CACHE_SIZE_MB: [u64; 3] = [1024, 2048, 4096];

/// What the visible settings become when a profile is picked; the engine keeps its own copy of
/// the rest of the profile.
struct ProfilePreset {
    limit_fps: i64,
    view_distance: f64,
    particle_quality: i64,
}

const PROFILES: [ProfilePreset; 4] = [
    ProfilePreset {
        limit_fps: 2,
        view_distance: 100.0,
        particle_quality: 0,
    },
    ProfilePreset {
        limit_fps: 3,
        view_distance: 150.0,
        particle_quality: 1,
    },
    ProfilePreset {
        limit_fps: 3,
        view_distance: 200.0,
        particle_quality: 2,
    },
    ProfilePreset {
        limit_fps: 4,
        view_distance: 300.0,
        particle_quality: 3,
    },
];

/// The settings other than the profile itself that picking it announces.
pub const PROFILE_DEPENDENT: [SettingKey; 3] = [
    SettingKey::ViewDistance,
    SettingKey::ParticleQuality,
    SettingKey::LimitFps,
];

/// The values a profile writes; empty for Custom (index 4) and for an index with no preset.
pub fn profile_values(profile: i64) -> Vec<(SettingKey, SettingValue)> {
    let Some(preset) = usize::try_from(profile).ok().and_then(|i| PROFILES.get(i)) else {
        return Vec::new();
    };
    vec![
        (SettingKey::LimitFps, SettingValue::Int(preset.limit_fps)),
        (
            SettingKey::ViewDistance,
            SettingValue::Float(preset.view_distance),
        ),
        (
            SettingKey::ParticleQuality,
            SettingValue::Int(preset.particle_quality),
        ),
    ]
}

pub fn locale_supported(locale: &str, allow_pseudo: bool) -> bool {
    locale.is_empty() || LOCALES.contains(&locale) || (allow_pseudo && locale == PSEUDO_LOCALE)
}

/// The size limit of the cache for the stored index, clamped into the table.
pub fn max_cache_bytes(index: i64) -> u64 {
    let last = CACHE_SIZE_MB.len() as i64 - 1;
    CACHE_SIZE_MB[index.clamp(0, last) as usize] * BYTES_PER_MB
}

/// Production shows `x.y.z`; any other build shows the full version.
pub fn display_version(build: &BuildInfo) -> String {
    if !build.production {
        return build.full_version.clone();
    }
    let parts: Vec<&str> = build.short_version.split('.').collect();
    match parts.as_slice() {
        [major, minor, patch, ..] => format!("{major}.{minor}.{patch}"),
        _ => build.short_version.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(production: bool, short: &str) -> BuildInfo {
        BuildInfo {
            short_version: short.into(),
            full_version: "1.14.0.1844-6cefb54-staging".into(),
            production,
        }
    }

    #[test]
    fn version_trims_to_three_parts_in_production() {
        assert_eq!(display_version(&build(true, "1.14.0.1844")), "1.14.0");
        assert_eq!(display_version(&build(true, "1.14.0")), "1.14.0");
        assert_eq!(display_version(&build(true, "1.14")), "1.14");
        assert_eq!(display_version(&build(true, "")), "");
    }

    #[test]
    fn version_is_full_outside_production() {
        assert_eq!(
            display_version(&build(false, "1.14.0.1844")),
            "1.14.0.1844-6cefb54-staging"
        );
    }

    #[test]
    fn cache_limit_table_clamps() {
        assert_eq!(max_cache_bytes(0), 1_024_000_000);
        assert_eq!(max_cache_bytes(1), 2_048_000_000);
        assert_eq!(max_cache_bytes(2), 4_096_000_000);
        assert_eq!(max_cache_bytes(-5), 1_024_000_000);
        assert_eq!(max_cache_bytes(99), 4_096_000_000);
    }

    #[test]
    fn profiles_write_three_values_except_custom() {
        let high = profile_values(3);
        assert_eq!(high.len(), 3);
        assert!(high.contains(&(SettingKey::ViewDistance, SettingValue::Float(300.0))));
        assert!(high.contains(&(SettingKey::LimitFps, SettingValue::Int(4))));
        assert!(profile_values(4).is_empty());
        assert!(profile_values(-1).is_empty());
        assert!(profile_values(9).is_empty());
    }

    #[test]
    fn locales() {
        assert!(locale_supported("", false));
        assert!(locale_supported("pt_BR", false));
        assert!(!locale_supported("fr", true));
        assert!(!locale_supported(PSEUDO_LOCALE, false));
        assert!(locale_supported(PSEUDO_LOCALE, true));
    }
}
