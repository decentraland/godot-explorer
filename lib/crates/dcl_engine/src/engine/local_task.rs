//! Main-thread futures: flows that hold Godot objects across awaits. They run on gdext's own
//! runtime (`godot::task`): a task is woken by the signal it awaits (from any thread, through a
//! deferred call), a panic inside one is caught and reported, and strict builds warn when a
//! `bind()` guard is held across an `.await`.

use std::{cell::RefCell, future::Future};

use godot::classes::{Engine, SceneTree};
use godot::prelude::*;

use crate::godot_classes::promise::Promise;

/// Runs `future` on the main thread, starting at the end of the current frame (so it never runs
/// inside a borrow its spawner holds).
pub fn spawn_local(future: impl Future<Output = ()> + 'static) {
    defer(move || {
        godot::task::spawn(future);
    });
}

/// Like `spawn_local`, but polls `future` right away, so everything up to its first real wait
/// happens before this returns. For a flow that used to run synchronously inside a GDScript
/// `_ready` and must still finish before the nodes that follow it.
pub fn spawn_local_now(future: impl Future<Output = ()> + 'static) {
    godot::task::spawn(future);
}

fn main_tree() -> Option<Gd<SceneTree>> {
    Engine::singleton()
        .get_main_loop()
        .and_then(|main_loop| main_loop.try_cast::<SceneTree>().ok())
}

/// Completes on the next `process_frame`.
pub async fn next_frame() {
    if let Some(tree) = main_tree() {
        let _ = tree.signals().process_frame().to_fallible_future().await;
    }
}

/// Completes after the next physics frame.
pub async fn next_physics_frame() {
    if let Some(tree) = main_tree() {
        let _ = tree.signals().physics_frame().to_fallible_future().await;
    }
}

/// The resolved value of `promise` (a `PromiseError` object when it was rejected, nil when the
/// promise was freed first).
pub async fn await_promise(promise: Gd<Promise>) -> Variant {
    let pending = !promise.bind().is_resolved();
    if pending {
        let watched = promise.clone();
        let _ = watched.signals().on_resolved().to_fallible_future().await;
    }
    if !promise.is_instance_valid() {
        return Variant::nil();
    }
    promise.bind().get_data()
}

/// Waits for every promise, in order; each result is the value `await_promise` would give.
pub async fn await_all(promises: Vec<Gd<Promise>>) -> Vec<Variant> {
    let mut results = Vec::with_capacity(promises.len());
    for promise in promises {
        results.push(await_promise(promise).await);
    }
    results
}

/// Runs `future` on the core runtime and hands back its output to a main-thread flow.
pub async fn on_core<T: Send + 'static>(
    future: impl Future<Output = T> + Send + 'static,
) -> Option<T> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    dcl_core::Core::spawn(async move {
        let _ = sender.send(future.await);
    });
    receiver.await.ok()
}

/// Completes once `seconds` have passed.
pub async fn sleep(seconds: f64) {
    if let Some(mut tree) = main_tree() {
        let timer = tree.create_timer(seconds);
        let _ = timer.signals().timeout().to_fallible_future().await;
    }
}

/// Runs `f` once on the main thread at the end of the current frame, from any thread (the typed,
/// thread-safe `call_deferred`). `f` carries ids and plain data, never a `Gd`; a `Variant` goes
/// through `args`, which the engine hands to `f` on the main thread.
pub fn defer_from_any_thread(args: &[Variant], f: impl FnOnce(&[&Variant]) + Send + 'static) {
    let cell = std::sync::Mutex::new(Some(f));
    Callable::from_sync_fn("defer_from_any_thread", move |args| {
        if let Some(f) = cell.lock().unwrap_or_else(|e| e.into_inner()).take() {
            f(args);
        }
        Variant::nil()
    })
    .call_deferred(args);
}

/// Runs `f` once, at the end of the current frame's message queue (the typed `call_deferred`).
pub fn defer(f: impl FnOnce() + 'static) {
    let cell = RefCell::new(Some(f));
    Callable::from_fn("defer", move |_| {
        if let Some(f) = cell.borrow_mut().take() {
            f();
        }
        Variant::nil()
    })
    .call_deferred(&[]);
}
