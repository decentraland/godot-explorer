//! Reconnects a dropped comms session and tells the shell when it could not: the account logged in
//! elsewhere, the room closed, the connection did not come back, or the scene's room banned the
//! player.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use godot::prelude::*;

use super::host::EngineHost;
use crate::engine::local_task::{sleep, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;

/// Disconnect reasons of `CommunicationManager` (the Rust `DisconnectReason` enum).
const REASON_DUPLICATE_IDENTITY: i32 = 0;
const REASON_ROOM_CLOSED: i32 = 1;
const REASON_KICKED: i32 = 2;

const MAX_RECONNECT_ATTEMPTS: u32 = 3;
/// A pause between tries so a flapping connection does not spin.
const RETRY_PAUSE_SECONDS: f64 = 1.0;

#[derive(Default)]
pub struct SessionWatch {
    attempts: Cell<u32>,
    adapter: RefCell<String>,
    stop: Cell<bool>,
    banned: Cell<bool>,
    loading: Cell<bool>,
}

impl SessionWatch {
    /// Listens until `owner` is freed.
    pub fn connect(self: &Rc<Self>, owner: &Gd<EngineHost>) {
        let comms = globals::comms();
        let me = self.clone();
        comms
            .signals()
            .disconnected()
            .connect_other(owner, move |_, reason| me.on_disconnected(reason));
        let me = self.clone();
        comms
            .signals()
            .on_adapter_changed()
            .connect_other(owner, move |_, _voice, _adapter| me.on_adapter_changed());
        let global = DclGlobal::singleton();
        let me = self.clone();
        global
            .signals()
            .loading_started()
            .connect_other(owner, move |_| me.loading.set(true));
        let me = self.clone();
        global
            .signals()
            .loading_finished()
            .connect_other(owner, move |_| {
                me.loading.set(false);
                // Kicked while loading: the notice waited for the world.
                if me.banned.get() {
                    globals::engine()
                        .bind()
                        .report_notice("banned".into(), "".into());
                }
            });
    }

    /// The player chose to try again after a disconnect notice.
    pub fn reconnect(&self) {
        let remembered = self.adapter.borrow().clone();
        let adapter = if remembered.is_empty() {
            globals::comms()
                .bind()
                .get_current_adapter_conn_str()
                .to_string()
        } else {
            remembered
        };
        self.attempts.set(0);
        self.adapter.borrow_mut().clear();
        if !adapter.is_empty() {
            globals::comms()
                .bind_mut()
                .change_adapter(adapter.as_str().into());
        }
    }

    fn on_adapter_changed(&self) {
        self.attempts.set(0);
        self.adapter.borrow_mut().clear();
        self.stop.set(false);
        self.banned.set(false);
    }

    fn on_disconnected(self: &Rc<Self>, reason: i32) {
        // Taken before any cleanup clears it.
        if self.adapter.borrow().is_empty() {
            *self.adapter.borrow_mut() = globals::comms()
                .bind()
                .get_current_adapter_conn_str()
                .to_string();
        }
        let notice = |kind: &str| {
            globals::engine()
                .bind()
                .report_notice(kind.into(), "".into())
        };
        match reason {
            REASON_DUPLICATE_IDENTITY => {
                self.attempts.set(0);
                self.stop.set(true);
                notice("disconnected_duplicate");
                return;
            }
            REASON_KICKED => {
                self.stop.set(true);
                self.banned.set(true);
                if !self.loading.get() {
                    notice("banned");
                }
                return;
            }
            _ => {}
        }
        self.attempts.set(self.attempts.get() + 1);
        self.stop.set(false);
        if self.attempts.get() < MAX_RECONNECT_ATTEMPTS && !self.adapter.borrow().is_empty() {
            let me = self.clone();
            spawn_local(async move {
                sleep(RETRY_PAUSE_SECONDS).await;
                let adapter = me.adapter.borrow().clone();
                if !me.stop.get() && !adapter.is_empty() {
                    globals::comms()
                        .bind_mut()
                        .change_adapter(adapter.as_str().into());
                }
            });
            return;
        }
        notice(if reason == REASON_ROOM_CLOSED {
            "disconnected_room_closed"
        } else {
            "disconnected_other"
        });
    }
}
