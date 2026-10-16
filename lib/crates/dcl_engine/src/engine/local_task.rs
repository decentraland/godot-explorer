//! Main-thread futures: flows that hold Godot objects across awaits. They are polled once per
//! `process_frame`, which is also when a GDScript `await` resumed.

use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::task::noop_waker_ref;
use godot::prelude::*;

use super::settings_link::on_process_frame;
use crate::godot_classes::promise::Promise;

type LocalFuture = Pin<Box<dyn Future<Output = ()>>>;

thread_local! {
    static RUNNING: RefCell<Vec<LocalFuture>> = const { RefCell::new(Vec::new()) };
    static SPAWNED: RefCell<Vec<LocalFuture>> = const { RefCell::new(Vec::new()) };
    static DRIVER_CONNECTED: Cell<bool> = const { Cell::new(false) };
}

/// Runs `future` on the main thread, starting with the next frame.
pub fn spawn_local(future: impl Future<Output = ()> + 'static) {
    SPAWNED.with(|spawned| spawned.borrow_mut().push(Box::pin(future)));
    if !DRIVER_CONNECTED.with(|connected| connected.replace(true)) {
        on_process_frame("local_tasks", poll_tasks);
    }
}

fn poll_tasks() {
    let mut running = RUNNING.with(|running| std::mem::take(&mut *running.borrow_mut()));
    running.extend(SPAWNED.with(|spawned| std::mem::take(&mut *spawned.borrow_mut())));
    let mut context = Context::from_waker(noop_waker_ref());
    // A task may spawn another: that one is queued in SPAWNED and starts next frame.
    running.retain_mut(|task| task.as_mut().poll(&mut context).is_pending());
    RUNNING.with(|slot| slot.borrow_mut().extend(running));
}

/// Like `spawn_local`, but polls `future` once right away, so everything up to its first real wait
/// happens before this returns. For a flow that used to run synchronously inside a GDScript
/// `_ready` and must still finish before the nodes that follow it.
pub fn spawn_local_now(future: impl Future<Output = ()> + 'static) {
    let mut future: LocalFuture = Box::pin(future);
    let mut context = Context::from_waker(noop_waker_ref());
    if future.as_mut().poll(&mut context).is_pending() {
        SPAWNED.with(|spawned| spawned.borrow_mut().push(future));
        if !DRIVER_CONNECTED.with(|connected| connected.replace(true)) {
            on_process_frame("local_tasks", poll_tasks);
        }
    }
}

/// Completes on the next `process_frame`.
pub fn next_frame() -> impl Future<Output = ()> {
    let mut polled = false;
    std::future::poll_fn(move |_| {
        if polled {
            Poll::Ready(())
        } else {
            polled = true;
            Poll::Pending
        }
    })
}

thread_local! {
    static PHYSICS_TICKS: Cell<u64> = const { Cell::new(0) };
}

/// Called once per physics frame by the node that owns the loop.
pub fn physics_tick() {
    PHYSICS_TICKS.with(|ticks| ticks.set(ticks.get() + 1));
}

/// Completes after the next physics frame (needs `physics_tick` driven).
pub fn next_physics_frame() -> impl Future<Output = ()> {
    let start = PHYSICS_TICKS.with(Cell::get);
    std::future::poll_fn(move |_| {
        if PHYSICS_TICKS.with(Cell::get) > start {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
}

/// The resolved value of `promise` (a `PromiseError` object when it was rejected).
pub fn await_promise(promise: Gd<Promise>) -> impl Future<Output = Variant> {
    std::future::poll_fn(move |_| {
        let bound = promise.bind();
        if bound.is_resolved() {
            Poll::Ready(bound.get_data())
        } else {
            Poll::Pending
        }
    })
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
pub fn on_core<T: Send + 'static>(
    future: impl Future<Output = T> + Send + 'static,
) -> impl Future<Output = Option<T>> {
    let (sender, mut receiver) = tokio::sync::oneshot::channel();
    dcl_core::Core::spawn(async move {
        let _ = sender.send(future.await);
    });
    std::future::poll_fn(move |_| match receiver.try_recv() {
        Ok(value) => Poll::Ready(Some(value)),
        Err(tokio::sync::oneshot::error::TryRecvError::Empty) => Poll::Pending,
        Err(tokio::sync::oneshot::error::TryRecvError::Closed) => Poll::Ready(None),
    })
}
