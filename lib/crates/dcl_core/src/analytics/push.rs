//! The push registration: what this install tells the warehouse about how to reach it.
//!
//! The shell reads the token and the permission (the OS owns both) and hands them over; the core
//! decides when the `identify` goes out: not before the `push-enabled` flag is known, not when the
//! flag is off, and once per token and permission.

use std::sync::Mutex;

use super::{
    data_definition::{PushIdentifyTraits, PushPlatform},
    service::Analytics,
};

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct PushRegistration {
    pub platform: PushPlatform,
    /// The FCM token or the APNs token as hex; empty when the device cannot receive push.
    pub token: String,
    /// "granted" | "denied"
    pub permission: String,
    pub app_version: String,
    /// The OS locale, e.g. "es_AR".
    pub locale: String,
    pub timezone: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PushOutcome {
    Sent,
    /// The flag is not known yet; the registration goes out when it is.
    Held,
    Disabled,
    /// The same token and permission already went out.
    Unchanged,
    NoToken,
}

#[derive(Default)]
struct Inner {
    held: Option<PushRegistration>,
    sent: Option<(String, String)>,
}

#[derive(Default)]
pub struct PushRegistrar {
    inner: Mutex<Inner>,
}

impl PushRegistrar {
    /// `flag` is the `push-enabled` verdict, `None` while the flags are not loaded.
    pub fn register(
        &self,
        analytics: &Analytics,
        registration: PushRegistration,
        flag: Option<bool>,
    ) -> PushOutcome {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if registration.token.is_empty() {
            return PushOutcome::NoToken;
        }
        let Some(enabled) = flag else {
            inner.held = Some(registration);
            return PushOutcome::Held;
        };
        inner.held = None;
        if !enabled {
            return PushOutcome::Disabled;
        }
        let key = (registration.token.clone(), registration.permission.clone());
        if inner.sent.as_ref() == Some(&key) {
            return PushOutcome::Unchanged;
        }
        inner.sent = Some(key);
        analytics.queue_push_identify(PushIdentifyTraits {
            platform: registration.platform,
            token: registration.token,
            push_permission: registration.permission,
            app_version: registration.app_version,
            locale: registration.locale,
            timezone: registration.timezone,
        });
        PushOutcome::Sent
    }

    /// The flags just loaded: sends what was held.
    pub fn flags_loaded(&self, analytics: &Analytics, enabled: bool) -> Option<PushOutcome> {
        let held = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .held
            .clone()?;
        Some(self.register(analytics, held, Some(enabled)))
    }
}

/// The process-wide registrar.
pub fn registrar() -> &'static PushRegistrar {
    static REGISTRAR: std::sync::OnceLock<PushRegistrar> = std::sync::OnceLock::new();
    REGISTRAR.get_or_init(PushRegistrar::default)
}

/// Called by the flags when they finish loading.
pub fn on_flags_loaded() {
    if let Some(analytics) = super::global::analytics() {
        registrar().flags_loaded(analytics, crate::flags::is_enabled("push-enabled", true));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use serde_json::Value;

    use super::*;
    use crate::analytics::{service::AnalyticsConfig, transport::SegmentTransport};

    #[derive(Default)]
    struct Sink(Mutex<Vec<String>>);

    #[async_trait]
    impl SegmentTransport for Sink {
        async fn send_batch(&self, body: String) -> Result<(), String> {
            self.0.lock().unwrap().push(body);
            Ok(())
        }
    }

    fn analytics() -> (Arc<Analytics>, Arc<Sink>) {
        let sink = Arc::new(Sink::default());
        let analytics = Arc::new(Analytics::new(
            AnalyticsConfig {
                write_key: "k".into(),
                user_id: "u".into(),
                session_id: "s".into(),
                os_name: "Android".into(),
                renderer_version: "2.0.0".into(),
                utc_offset_minutes: 0,
            },
            sink.clone(),
        ));
        (analytics, sink)
    }

    fn registration(token: &str, permission: &str) -> PushRegistration {
        PushRegistration {
            platform: PushPlatform::Android,
            token: token.into(),
            permission: permission.into(),
            app_version: "2.0.0".into(),
            locale: "es_AR".into(),
            timezone: "America/Argentina/Buenos_Aires".into(),
        }
    }

    async fn identifies(analytics: &Analytics, sink: &Sink) -> Vec<Value> {
        analytics.set_eula_accepted(true).await;
        sink.0
            .lock()
            .unwrap()
            .iter()
            .flat_map(|body| {
                serde_json::from_str::<Value>(body).unwrap()["batch"]
                    .as_array()
                    .unwrap()
                    .clone()
            })
            .filter(|item| item["type"] == "identify")
            .collect()
    }

    #[tokio::test]
    async fn the_identify_waits_for_the_flag_and_then_goes_out_once() {
        let (analytics, sink) = analytics();
        let registrar = PushRegistrar::default();

        assert_eq!(
            registrar.register(&analytics, registration("tok", "granted"), None),
            PushOutcome::Held
        );
        assert_eq!(
            registrar.flags_loaded(&analytics, true),
            Some(PushOutcome::Sent)
        );
        assert_eq!(
            registrar.flags_loaded(&analytics, true),
            None,
            "nothing held now"
        );
        assert_eq!(
            registrar.register(&analytics, registration("tok", "granted"), Some(true)),
            PushOutcome::Unchanged
        );
        let sent = identifies(&analytics, &sink).await;
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0]["traits"]["fcm_token"], "tok");
        assert_eq!(sent[0]["traits"]["push_permission"], "granted");
    }

    #[tokio::test]
    async fn a_new_token_or_permission_is_a_new_identify() {
        let (analytics, sink) = analytics();
        let registrar = PushRegistrar::default();
        for (token, permission) in [("a", "denied"), ("a", "granted"), ("b", "granted")] {
            assert_eq!(
                registrar.register(&analytics, registration(token, permission), Some(true)),
                PushOutcome::Sent
            );
        }
        assert_eq!(identifies(&analytics, &sink).await.len(), 3);
    }

    #[tokio::test]
    async fn the_flag_off_and_a_missing_token_send_nothing() {
        let (analytics, sink) = analytics();
        let registrar = PushRegistrar::default();
        assert_eq!(
            registrar.register(&analytics, registration("tok", "granted"), Some(false)),
            PushOutcome::Disabled
        );
        assert_eq!(
            registrar.register(&analytics, registration("", "denied"), Some(true)),
            PushOutcome::NoToken
        );
        assert!(identifies(&analytics, &sink).await.is_empty());
    }
}
