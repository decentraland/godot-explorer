//! Reconnects a dropped comms session and tells the shell when it could not: the account logged in
//! elsewhere or was banned, the room closed, Pulse stayed down, the connection did not come back,
//! or the scene's room banned the player.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use godot::prelude::*;

use super::host::EngineHost;
use crate::engine::local_task::{defer, sleep, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;

/// Codes of the `disconnected` signal of `CommunicationManager`.
const REASON_DUPLICATE_IDENTITY: i32 = 0;
const REASON_ROOM_CLOSED: i32 = 1;
const REASON_KICKED: i32 = 2;
const REASON_PULSE_UNAVAILABLE: i32 = 4;
const REASON_PLATFORM_BANNED: i32 = 5;

const MAX_RECONNECT_ATTEMPTS: u32 = 3;
/// A pause between tries so a flapping connection does not spin.
const RETRY_PAUSE_SECONDS: f64 = 1.0;

fn notice(kind: &str) {
    globals::engine()
        .bind()
        .report_notice(kind.into(), "".into())
}

#[derive(Default)]
pub struct SessionWatch {
    attempts: Cell<u32>,
    adapter: RefCell<String>,
    stop: Cell<bool>,
    banned: Cell<bool>,
    loading: Cell<bool>,
    /// The account's session ended (evicted or banned): nothing reconnects it.
    ended: Cell<Option<i32>>,
    /// Pulse is down: only retrying Pulse brings multiplayer back.
    pulse_outage: Cell<bool>,
    /// Bumped on every disconnect so a pending retry from an older one does not fire.
    generation: Cell<u32>,
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
            .pulse_recovered()
            .connect_other(owner, move |_| me.on_pulse_recovered());
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
            .connect_other(owner, move |_| {
                me.generation.set(me.generation.get() + 1);
                me.loading.set(true);
            });
        let me = self.clone();
        global
            .signals()
            .loading_finished()
            .connect_other(owner, move |_| {
                me.loading.set(false);
                if let Some(reason) = me.ended.get() {
                    me.announce_ended(reason);
                    return;
                }
                if me.pulse_outage.get() {
                    notice("disconnected_other");
                    return;
                }
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
        if self.ended.get().is_some() {
            return;
        }
        if self.pulse_outage.get() {
            self.pulse_outage.set(false);
            defer(|| globals::comms().bind_mut().retry_pulse());
            return;
        }
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
        if self.ended.get().is_some() || self.pulse_outage.get() {
            return;
        }
        self.attempts.set(0);
        self.adapter.borrow_mut().clear();
        self.stop.set(false);
        self.banned.set(false);
    }

    fn on_pulse_recovered(&self) {
        if !self.pulse_outage.get() || self.ended.get().is_some() {
            return;
        }
        self.pulse_outage.set(false);
        self.generation.set(self.generation.get() + 1);
        self.stop.set(false);
        self.attempts.set(0);
        notice("disconnect_recovered");
    }

    fn announce_ended(&self, reason: i32) {
        notice(if reason == REASON_PLATFORM_BANNED {
            "disconnected_platform_ban"
        } else {
            "disconnected_duplicate"
        });
    }

    fn on_disconnected(self: &Rc<Self>, reason: i32) {
        if self.ended.get().is_some() {
            return;
        }
        self.generation.set(self.generation.get() + 1);
        if reason == REASON_PULSE_UNAVAILABLE {
            self.stop.set(true);
            self.pulse_outage.set(true);
            if !self.loading.get() {
                notice("disconnected_other");
            }
            return;
        }
        // Taken before any cleanup clears it.
        if self.adapter.borrow().is_empty() {
            *self.adapter.borrow_mut() = globals::comms()
                .bind()
                .get_current_adapter_conn_str()
                .to_string();
        }
        match reason {
            REASON_DUPLICATE_IDENTITY | REASON_PLATFORM_BANNED => {
                self.attempts.set(0);
                self.stop.set(true);
                self.pulse_outage.set(false);
                self.ended.set(Some(reason));
                if !self.loading.get() {
                    self.announce_ended(reason);
                }
                return;
            }
            REASON_KICKED => {
                self.pulse_outage.set(false);
                self.stop.set(true);
                self.banned.set(true);
                if !self.loading.get() {
                    notice("banned");
                }
                return;
            }
            _ => {}
        }
        if self.pulse_outage.get() {
            return;
        }
        self.attempts.set(self.attempts.get() + 1);
        self.stop.set(false);
        if self.attempts.get() < MAX_RECONNECT_ATTEMPTS && !self.adapter.borrow().is_empty() {
            let me = self.clone();
            let generation = self.generation.get();
            spawn_local(async move {
                sleep(RETRY_PAUSE_SECONDS).await;
                let adapter = me.adapter.borrow().clone();
                if !me.stop.get() && generation == me.generation.get() && !adapter.is_empty() {
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
