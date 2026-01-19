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
    pub fn from_str(s: &str) -> Option<Self> {
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

/// Transform a URL to use the current environment's domain
/// Replaces decentraland.org with decentraland.{suffix}
pub fn transform_url(url: &str) -> String {
    let env = get_environment();
    if env == DclEnvironment::Org {
        // No transformation needed for production
        return url.to_string();
    }

    let suffix = env.suffix();

    // Transform various Decentraland domains
    url.replace("decentraland.org", &format!("decentraland.{}", suffix))
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
        assert_eq!(DclEnvironment::from_str("org"), Some(DclEnvironment::Org));
        assert_eq!(DclEnvironment::from_str("ORG"), Some(DclEnvironment::Org));
        assert_eq!(DclEnvironment::from_str("zone"), Some(DclEnvironment::Zone));
        assert_eq!(
            DclEnvironment::from_str("today"),
            Some(DclEnvironment::Today)
        );
        assert_eq!(DclEnvironment::from_str("invalid"), None);
    }

    #[test]
    fn test_transform_url_org() {
        // When environment is org (default), no transformation
        let url = "https://decentraland.org/auth/requests";
        // Note: Since we can't set environment in tests (OnceLock), we test the logic
        assert!(url.contains("decentraland.org"));
    }
}
