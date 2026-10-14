//! Sentry tag naming the resource a worker thread is loading, so a native crash
//! inside Godot's loader says which file it died on.

use godot::{classes::Engine, prelude::*};

const TAG: &str = "loading_resource";

pub fn set_loading_resource(path: &str) {
    call_sentry("set_tag", &[TAG.to_variant(), path.to_variant()]);
}

pub fn clear_loading_resource() {
    call_sentry("remove_tag", &[TAG.to_variant()]);
}

// `SentrySDK` is absent when the sentry addon is not loaded (editor, tests).
fn call_sentry(method: &str, args: &[Variant]) {
    if let Some(mut sentry) = Engine::singleton().get_singleton("SentrySDK") {
        sentry.call(method, args);
    }
}
