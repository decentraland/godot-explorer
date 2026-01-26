/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::OnceLock;

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

static CURRENT_ENV: OnceLock<DclEnvironment> = OnceLock::new();

/// Get the current environment (defaults to Org if not set)
pub fn get_environment() -> DclEnvironment {
    *CURRENT_ENV.get().unwrap_or(&DclEnvironment::Org)
}

/// Set the environment (can only be set once)
pub fn set_environment(env: DclEnvironment) {
    let _ = CURRENT_ENV.set(env);
    tracing::info!("Environment set to: {:?} ({})", env, env.suffix());
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
}
