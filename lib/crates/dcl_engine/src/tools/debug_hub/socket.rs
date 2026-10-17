//! The WebSocket of the debug hub: sends inspector data to a dedicated target and receives its
//! commands, reconnecting with exponential backoff. Plain shared state, polled by the bridge's
//! node: a node that polls and answers from inside the same call would borrow itself twice.

use std::cell::{Cell, RefCell};

use godot::classes::web_socket_peer::State;
use godot::classes::WebSocketPeer;
use godot::prelude::*;

use super::frames::{parse_packet, Command};

/// A 64 MB outbound buffer: the connections are local, and messages are never dropped.
const OUTBOUND_BUFFER_SIZE: i32 = 64 * 1024 * 1024;
const RECONNECT_INITIAL_DELAY: f64 = 1.0;
const RECONNECT_MAX_DELAY: f64 = 30.0;

pub enum Event {
    Command(Command),
    /// The socket opened.
    Connected,
    /// The socket dropped: capture can be gated off.
    Disconnected,
}

pub struct Socket {
    peer: RefCell<Gd<WebSocketPeer>>,
    target_url: RefCell<String>,
    reconnect_delay: Cell<f64>,
    reconnect_timer: Cell<f64>,
    was_open: Cell<bool>,
}

impl Default for Socket {
    fn default() -> Self {
        Self {
            peer: RefCell::new(WebSocketPeer::new_gd()),
            target_url: RefCell::default(),
            reconnect_delay: Cell::new(RECONNECT_INITIAL_DELAY),
            reconnect_timer: Cell::new(0.0),
            was_open: Cell::new(false),
        }
    }
}

impl Socket {
    pub fn connect_to(&self, url: &str) {
        // Switching to another URL closes the old connection.
        if !self.target_url.borrow().is_empty() && *self.target_url.borrow() != url {
            self.peer.borrow_mut().close();
            *self.peer.borrow_mut() = WebSocketPeer::new_gd();
        }
        *self.target_url.borrow_mut() = url.to_string();
        self.reconnect_delay.set(RECONNECT_INITIAL_DELAY);
        self.reconnect_timer.set(0.0);
        self.was_open.set(false);
        let mut peer = self.peer.borrow().clone();
        peer.set_outbound_buffer_size(OUTBOUND_BUFFER_SIZE);
        peer.connect_to_url(url);
    }

    pub fn is_open(&self) -> bool {
        self.peer.borrow().get_ready_state() == State::OPEN
    }

    /// Sends a text frame: a pre-serialized JSON string for the hot paths (the inspector
    /// batches), which skips building and stringifying a dictionary.
    pub fn send_text(&self, text: &str) {
        if self.is_open() {
            self.peer.borrow().clone().send_text(text);
        }
    }

    /// Reads what arrived and keeps the connection alive; every event goes to `handler`.
    pub fn poll(&self, delta: f64, handler: &dyn Fn(Event)) {
        let mut peer = self.peer.borrow().clone();
        peer.poll();
        while peer.get_available_packet_count() > 0 {
            let packet = peer.get_packet();
            for command in parse_packet(&String::from_utf8_lossy(packet.as_slice())) {
                handler(Event::Command(command));
            }
        }
        let state = peer.get_ready_state();
        if state == State::OPEN {
            if !self.was_open.replace(true) {
                handler(Event::Connected);
            }
            self.reconnect_delay.set(RECONNECT_INITIAL_DELAY);
            self.reconnect_timer.set(0.0);
            return;
        }
        if self.was_open.replace(false) {
            handler(Event::Disconnected);
        }
        let target = self.target_url.borrow().clone();
        if state == State::CLOSED && !target.is_empty() {
            self.reconnect_timer.set(self.reconnect_timer.get() + delta);
            if self.reconnect_timer.get() >= self.reconnect_delay.get() {
                self.reconnect_timer.set(0.0);
                peer.connect_to_url(target.as_str());
                self.reconnect_delay
                    .set((self.reconnect_delay.get() * 2.0).min(RECONNECT_MAX_DELAY));
            }
        }
    }
}
