use godot::{builtin::Variant, classes::Script, meta::ToGodot, obj::Gd};

use crate::godot_classes::promise::Promise;

use super::content_provider::ContentProviderContext;

pub struct GodotSingleThreadSafety {
    _guard: tokio::sync::OwnedSemaphorePermit,
}

impl GodotSingleThreadSafety {
    pub async fn acquire_owned(ctx: &ContentProviderContext) -> Option<Self> {
        let guard = ctx.godot_single_thread.clone().acquire_owned().await.ok()?;
        set_thread_safety_checks_enabled(false);
        Some(Self { _guard: guard })
    }
}

impl Drop for GodotSingleThreadSafety {
    fn drop(&mut self) {
        set_thread_safety_checks_enabled(true);
    }
}

// Interacting with Godot API is not thread safe, so we need to disable thread safety checks
// When this option is triggered (as false), be sure to not use async/await until you set it back to true
// Following the same logic, do not exit of sync closure until you set it back to true
pub fn set_thread_safety_checks_enabled(enabled: bool) {
    let mut temp_script = godot::tools::load::<Script>("res://src/logic/thread_safety.gd");
    temp_script.call("set_thread_safety_checks_enabled", &[enabled.to_variant()]);
}

fn reject_promise(get_promise: impl Fn() -> Option<Gd<Promise>>, reason: String) -> bool {
    if let Some(mut promise) = get_promise() {
        promise.call_deferred("reject", &[reason.to_variant()]);
        true
    } else {
        false
    }
}

fn resolve_promise(get_promise: impl Fn() -> Option<Gd<Promise>>, value: Option<Variant>) -> bool {
    if let Some(mut promise) = get_promise() {
        if let Some(value) = value {
            promise.call_deferred("resolve_with_data", &[value]);
        } else {
            promise.call_deferred("resolve", &[]);
        }
        true
    } else {
        false
    }
}

pub fn then_promise(
    get_promise: impl Fn() -> Option<Gd<Promise>>,
    result: Result<Option<Variant>, anyhow::Error>,
) {
    match result {
        Ok(value) => resolve_promise(get_promise, value),
        Err(reason) => reject_promise(get_promise, reason.to_string()),
    };
}
