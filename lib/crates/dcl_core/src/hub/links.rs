//! Deep links: routes one with the pure router, then does the parts that belong to the account
//! (finishing a browser sign-in, resuming one parked by a cold start) and the log filter. What is
//! left (navigation, settings the engine owns, analytics) goes back to the shell.

use std::sync::Arc;

use crate::{
    app::State,
    deep_link_route::AuthSnapshot,
    deep_link_route::{
        ConfigParam, DeepLinkRouter, Effect, MonotonicClock, RouteContext, Routed, StartKind,
    },
    Platform,
};

use super::Hub;

/// The result of routing one link.
pub struct LinkOutcome {
    pub routed: Routed,
    /// A browser sign-in the caller must finish now with `complete_browser`.
    pub complete_sign_in: Option<String>,
}

impl Hub {
    pub(super) fn new_router(clock: Arc<dyn MonotonicClock>) -> DeepLinkRouter {
        DeepLinkRouter::new(clock)
    }

    fn route_context(&self, platform: Platform) -> RouteContext {
        let (signed_out, in_flow) = self.with_app(|app| {
            (
                matches!(app.state(), State::SignedOut),
                matches!(app.state(), State::SigningIn { .. }),
            )
        });
        let (pending, cancelled, connected) = self.with_session(|s| {
            (
                s.has_pending_mobile_auth(),
                s.was_mobile_auth_cancelled(),
                s.try_address().is_some(),
            )
        });
        RouteContext {
            real_mobile: platform != Platform::Desktop,
            production: self.production.load(std::sync::atomic::Ordering::Relaxed),
            current_dclenv: crate::env::get_environment().suffix().to_string(),
            dclenv_explicit: false,
            marketplace_awaiting_return: false,
            auth: AuthSnapshot {
                sign_in_cancelled: cancelled && (signed_out || in_flow),
                pending_mobile_auth: pending,
                wallet_connected: connected,
            },
        }
    }

    pub fn route_deep_link(&self, url: &str, start: StartKind, platform: Platform) -> LinkOutcome {
        let context = self.route_context(platform);
        let mut routed = self
            .router
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .handle(url, start, &context);

        let mut complete_sign_in = None;
        let mut rest = Vec::with_capacity(routed.effects.len());
        for effect in routed.effects.drain(..) {
            match effect {
                Effect::CompleteMobileSignIn { identity_id } => {
                    complete_sign_in = Some(identity_id)
                }
                Effect::SignInParked => {
                    complete_sign_in = complete_sign_in.or_else(|| self.resume_parked_sign_in());
                }
                Effect::Config {
                    param: ConfigParam::RustLog { filter },
                } => {
                    if let Err(e) = crate::logging::set_filter(&filter) {
                        tracing::warn!("deep link log filter: {e}");
                    }
                }
                Effect::Config {
                    param: ConfigParam::EnableUpgradedDeletion,
                } => self.allow_upgraded_deletion(),
                Effect::SetPreviewUrl { url } => {
                    self.with_app(|app| app.control(crate::app::Control::SetPreviewUrl { url }))
                }
                other => rest.push(other),
            }
        }
        routed.effects = rest;
        LinkOutcome {
            routed,
            complete_sign_in,
        }
    }

    /// The identity of a sign-in a cold start parked, if the user is signed out to take it. Called
    /// again after boot, since the link can arrive before the saved account was looked up.
    pub fn resume_parked_sign_in(&self) -> Option<String> {
        if !matches!(self.with_app(|app| app.state().clone()), State::SignedOut) {
            return None;
        }
        self.router
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take_pending_signin_identity_id()
    }
}
