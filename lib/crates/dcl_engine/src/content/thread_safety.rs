//! Godot objects are used off the main thread only by the content loaders, and only while
//! holding `GodotSingleThreadSafety` (the `godot_single_thread` semaphore, one permit): textures
//! (`texture.rs`), audio (`audio.rs`), glTF (`gltf/common.rs`), the content provider's own
//! loads (`content_provider.rs`) and the asset server processor. With a permit held the engine's
//! thread-safety checks are off, so nothing between acquiring and dropping it may `.await`.
//! Every other thread or blocking task in the crate (impostor PNG encode, the scene CRDT lock,
//! the HTTP requester, the bundle cache) works on plain data and never holds a `Gd`.

use godot::{
    builtin::{GString, Variant},
    meta::ToGodot,
    obj::Gd,
};

use crate::godot_classes::promise::Promise;

use super::content_provider::ContentProviderContext;
use crate::engine::local_task::defer_from_any_thread;

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
    // `Thread` is not in the generated API, so its static is called through ClassDB.
    <godot::classes::ClassDb as godot::obj::Singleton>::singleton().class_call_static(
        "Thread",
        "set_thread_safety_checks_enabled",
        &[enabled.to_variant()],
    );
}

/// Settles the promise on the main thread: the promise is looked up again by its id there, and
/// a value travels as the deferred call's argument.
fn settle_promise(
    get_promise: impl Fn() -> Option<Gd<Promise>>,
    args: &[Variant],
    settle: impl FnOnce(&mut Promise, &[&Variant]) + Send + 'static,
) -> bool {
    let Some(promise) = get_promise() else {
        return false;
    };
    let id = promise.instance_id();
    defer_from_any_thread(args, move |args| {
        if let Ok(mut promise) = Gd::<Promise>::try_from_instance_id(id) {
            settle(&mut promise.bind_mut(), args);
        }
    });
    true
}

fn reject_promise(get_promise: impl Fn() -> Option<Gd<Promise>>, reason: String) -> bool {
    settle_promise(
        get_promise,
        &[GString::from(&reason).to_variant()],
        |promise, args| promise.reject(args[0].to::<GString>()),
    )
}

fn resolve_promise(get_promise: impl Fn() -> Option<Gd<Promise>>, value: Option<Variant>) -> bool {
    match value {
        Some(value) => settle_promise(get_promise, &[value], |promise, args| {
            promise.resolve_with_data(args[0].clone())
        }),
        None => settle_promise(get_promise, &[], |promise, _| promise.resolve()),
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
