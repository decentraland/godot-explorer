/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::RwLock;

/// Decentraland environment for URL transformation
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DclEnvironment {
    /// Production environment (decentraland.org)
    #[default]
    Org,
    /// Staging environment (decentraland.zone)
    Zone,
    /// Development environment (decentraland.today)
    Today,
}

impl DclEnvironment {
    /// Returns the domain suffix for this environment
    pub fn suffix(&self) -> &'static str {
        match self {
            Self::Org => "org",
            Self::Zone => "zone",
            Self::Today => "today",
        }
    }

    /// Parse environment from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "org" => Some(Self::Org),
            "zone" => Some(Self::Zone),
            "today" => Some(Self::Today),
            _ => None,
        }
    }
}

/// Service groups for per-group environment overrides.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ServiceGroup {
    Auth,
    Catalyst,
    Comms,
    Events,
    Places,
    MobileBff,
    Notifications,
}

impl ServiceGroup {
    pub const COUNT: usize = 7;

    pub fn index(self) -> usize {
        match self {
            Self::Auth => 0,
            Self::Catalyst => 1,
            Self::Comms => 2,
            Self::Events => 3,
            Self::Places => 4,
            Self::MobileBff => 5,
            Self::Notifications => 6,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auth => "auth",
            Self::Catalyst => "catalyst",
            Self::Comms => "comms",
            Self::Events => "events",
            Self::Places => "places",
            Self::MobileBff => "mobilebff",
            Self::Notifications => "notifications",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "auth" => Some(Self::Auth),
            "catalyst" => Some(Self::Catalyst),
            "comms" => Some(Self::Comms),
            "events" => Some(Self::Events),
            "places" => Some(Self::Places),
            "mobilebff" => Some(Self::MobileBff),
            "notifications" => Some(Self::Notifications),
            _ => None,
        }
    }

    const ALL: [Self; Self::COUNT] = [
        Self::Auth,
        Self::Catalyst,
        Self::Comms,
        Self::Events,
        Self::Places,
        Self::MobileBff,
        Self::Notifications,
    ];
}

/// Per-group environment configuration.
///
/// Supports RUST_LOG-style format:
/// - `"zone"` → everything uses zone (backward compatible)
/// - `"auth::zone,org"` → default org, auth uses zone
/// - `"auth::zone,comms::today,org"` → default org, auth=zone, comms=today
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DclEnvConfig {
    pub default: DclEnvironment,
    overrides: [Option<DclEnvironment>; ServiceGroup::COUNT],
}

impl Default for DclEnvConfig {
    fn default() -> Self {
        Self {
            default: DclEnvironment::Org,
            overrides: [None; ServiceGroup::COUNT],
        }
    }
}

impl DclEnvConfig {
    /// Parse a dclenv string. Supports:
    /// - `"zone"` → uniform environment
    /// - `"auth::zone,org"` → default org, auth override zone
    /// - `"auth::zone,comms::today,org"` → default org, auth=zone, comms=today
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }

        // Simple case: single environment name with no commas or colons
        if !s.contains(',') && !s.contains("::") {
            let env = DclEnvironment::parse(s)?;
            return Some(Self {
                default: env,
                overrides: [None; ServiceGroup::COUNT],
            });
        }

        let mut config = Self::default();
        let mut found_default = false;
        let mut overrides = [None; ServiceGroup::COUNT];

        for part in s.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            if let Some((group_str, env_str)) = part.split_once("::") {
                // group::env override
                let group = ServiceGroup::parse(group_str)?;
                let env = DclEnvironment::parse(env_str)?;
                overrides[group.index()] = Some(env);
            } else {
                // bare environment name = default
                if found_default {
                    return None; // multiple defaults
                }
                config.default = DclEnvironment::parse(part)?;
                found_default = true;
            }
        }

        // Must have a default if there are overrides
        if !found_default {
            return None;
        }

        config.overrides = overrides;
        Some(config)
    }

    /// Returns the resolved environment for a service group.
    pub fn env_for(&self, group: ServiceGroup) -> DclEnvironment {
        self.overrides[group.index()].unwrap_or(self.default)
    }

    /// Returns the domain suffix for a service group.
    pub fn suffix_for(&self, group: ServiceGroup) -> &'static str {
        self.env_for(group).suffix()
    }

    /// Returns true if a specific group has an explicit override.
    pub fn has_override(&self, group: ServiceGroup) -> bool {
        self.overrides[group.index()].is_some()
    }

    /// Returns true if there are no per-group overrides (uniform config).
    pub fn is_uniform(&self) -> bool {
        self.overrides.iter().all(|o| o.is_none())
    }

    /// Serialize back to the string representation.
    pub fn to_string_repr(&self) -> String {
        if self.is_uniform() {
            return self.default.suffix().to_string();
        }

        let mut parts = Vec::new();
        for group in ServiceGroup::ALL {
            if let Some(env) = self.overrides[group.index()] {
                parts.push(format!("{}::{}", group.as_str(), env.suffix()));
            }
        }
        parts.push(self.default.suffix().to_string());
        parts.join(",")
    }
}

static CURRENT_CONFIG: RwLock<DclEnvConfig> = RwLock::new(DclEnvConfig {
    default: DclEnvironment::Org,
    overrides: [None; ServiceGroup::COUNT],
});

/// Get the current environment config.
pub fn get_config() -> DclEnvConfig {
    CURRENT_CONFIG.read().unwrap().clone()
}

/// Set the environment config. Can be called multiple times (e.g. hot deep link).
pub fn set_environment_config(config: DclEnvConfig) {
    tracing::info!("Environment config set to: {}", config.to_string_repr());
    *CURRENT_CONFIG.write().unwrap() = config;
}

/// Get the current default environment (backward compat).
pub fn get_environment() -> DclEnvironment {
    get_config().default
}

/// Set the environment from a single DclEnvironment (backward compat).
pub fn set_environment(env: DclEnvironment) {
    set_environment_config(DclEnvConfig {
        default: env,
        overrides: [None; ServiceGroup::COUNT],
    });
}

/// Get the resolved environment for a service group.
pub fn env_for(group: ServiceGroup) -> DclEnvironment {
    get_config().env_for(group)
}

/// Get the domain suffix for a service group.
pub fn suffix_for(group: ServiceGroup) -> &'static str {
    get_config().suffix_for(group)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_suffix() {
        assert_eq!(DclEnvironment::Org.suffix(), "org");
        assert_eq!(DclEnvironment::Zone.suffix(), "zone");
        assert_eq!(DclEnvironment::Today.suffix(), "today");
    }

    #[test]
    fn test_environment_from_str() {
        assert_eq!(DclEnvironment::parse("org"), Some(DclEnvironment::Org));
        assert_eq!(DclEnvironment::parse("ORG"), Some(DclEnvironment::Org));
        assert_eq!(DclEnvironment::parse("zone"), Some(DclEnvironment::Zone));
        assert_eq!(DclEnvironment::parse("today"), Some(DclEnvironment::Today));
        assert_eq!(DclEnvironment::parse("invalid"), None);
    }

    #[test]
    fn test_service_group_parse() {
        assert_eq!(ServiceGroup::parse("auth"), Some(ServiceGroup::Auth));
        assert_eq!(
            ServiceGroup::parse("catalyst"),
            Some(ServiceGroup::Catalyst)
        );
        assert_eq!(ServiceGroup::parse("comms"), Some(ServiceGroup::Comms));
        assert_eq!(ServiceGroup::parse("events"), Some(ServiceGroup::Events));
        assert_eq!(ServiceGroup::parse("places"), Some(ServiceGroup::Places));
        assert_eq!(
            ServiceGroup::parse("mobilebff"),
            Some(ServiceGroup::MobileBff)
        );
        assert_eq!(
            ServiceGroup::parse("notifications"),
            Some(ServiceGroup::Notifications)
        );
        assert_eq!(ServiceGroup::parse("invalid"), None);
        // case insensitive
        assert_eq!(ServiceGroup::parse("AUTH"), Some(ServiceGroup::Auth));
    }

    #[test]
    fn test_config_parse_simple() {
        let config = DclEnvConfig::parse("zone").unwrap();
        assert_eq!(config.default, DclEnvironment::Zone);
        assert!(config.is_uniform());
    }

    #[test]
    fn test_config_parse_with_overrides() {
        let config = DclEnvConfig::parse("auth::zone,org").unwrap();
        assert_eq!(config.default, DclEnvironment::Org);
        assert_eq!(config.env_for(ServiceGroup::Auth), DclEnvironment::Zone);
        assert_eq!(config.env_for(ServiceGroup::Comms), DclEnvironment::Org);
        assert!(!config.is_uniform());
    }

    #[test]
    fn test_config_parse_multiple_overrides() {
        let config = DclEnvConfig::parse("auth::zone,comms::today,org").unwrap();
        assert_eq!(config.default, DclEnvironment::Org);
        assert_eq!(config.env_for(ServiceGroup::Auth), DclEnvironment::Zone);
        assert_eq!(config.env_for(ServiceGroup::Comms), DclEnvironment::Today);
        assert_eq!(config.env_for(ServiceGroup::Catalyst), DclEnvironment::Org);
    }

    #[test]
    fn test_config_parse_invalid() {
        assert!(DclEnvConfig::parse("").is_none());
        assert!(DclEnvConfig::parse("invalid").is_none());
        assert!(DclEnvConfig::parse("auth::zone").is_none()); // no default
        assert!(DclEnvConfig::parse("org,zone").is_none()); // multiple defaults
        assert!(DclEnvConfig::parse("unknown::zone,org").is_none()); // unknown group
        assert!(DclEnvConfig::parse("auth::invalid,org").is_none()); // invalid env
    }

    #[test]
    fn test_config_to_string_repr_simple() {
        let config = DclEnvConfig::parse("zone").unwrap();
        assert_eq!(config.to_string_repr(), "zone");
    }

    #[test]
    fn test_config_to_string_repr_with_overrides() {
        let config = DclEnvConfig::parse("auth::zone,org").unwrap();
        assert_eq!(config.to_string_repr(), "auth::zone,org");
    }

    #[test]
    fn test_config_to_string_repr_roundtrip() {
        let inputs = [
            "org",
            "zone",
            "today",
            "auth::zone,org",
            "auth::zone,comms::today,org",
        ];
        for input in inputs {
            let config = DclEnvConfig::parse(input).unwrap();
            let repr = config.to_string_repr();
            let config2 = DclEnvConfig::parse(&repr).unwrap();
            assert_eq!(config, config2, "Roundtrip failed for: {}", input);
        }
    }

    #[test]
    fn test_config_has_override() {
        let config = DclEnvConfig::parse("auth::zone,org").unwrap();
        assert!(config.has_override(ServiceGroup::Auth));
        assert!(!config.has_override(ServiceGroup::Comms));
    }

    #[test]
    fn test_config_suffix_for() {
        let config = DclEnvConfig::parse("auth::zone,org").unwrap();
        assert_eq!(config.suffix_for(ServiceGroup::Auth), "zone");
        assert_eq!(config.suffix_for(ServiceGroup::Comms), "org");
    }
}
