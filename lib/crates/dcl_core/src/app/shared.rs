use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
};

use super::{AppEvents, EngineCommand, EngineSink, State};

/// Forwards the app's events to every registered listener (the shell's, the host's). Events fire
/// while the app lock is held, so a listener must not call back into the app synchronously.
#[derive(Default)]
pub struct HostEvents {
    listeners: RwLock<Vec<(u64, Arc<dyn AppEvents>)>>,
    next_id: AtomicU64,
}

impl HostEvents {
    /// Registers a listener; the id removes it again.
    pub fn add_listener(&self, listener: Arc<dyn AppEvents>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.listeners
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push((id, listener));
        id
    }

    pub fn remove_listener(&self, id: u64) {
        self.listeners
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|(listener_id, _)| *listener_id != id);
    }

    /// A listener that panics (a shell whose JS runtime was reloaded leaves a stale handle) is
    /// dropped and logged; it must not stop the others from hearing the event.
    fn each(&self, f: impl Fn(&dyn AppEvents)) {
        let listeners = self
            .listeners
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        for (id, listener) in listeners {
            let outcome =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(listener.as_ref())));
            if outcome.is_err() {
                tracing::error!("app listener {id} failed and was removed");
                self.remove_listener(id);
            }
        }
    }
}

impl AppEvents for HostEvents {
    fn state_changed(&self, state: &State) {
        self.each(|l| l.state_changed(state));
    }
    fn engine_required(&self) {
        self.each(|l| l.engine_required());
    }
    fn avatar_preview_mode(&self, on: bool) {
        self.each(|l| l.avatar_preview_mode(on));
    }
    fn scene_crashed(&self) {
        self.each(|l| l.scene_crashed());
    }
    fn auth_error(&self, code: &str) {
        self.each(|l| l.auth_error(code));
    }
    fn open_url(&self, request: &crate::OpenUrlRequest) {
        self.each(|l| l.open_url(request));
    }
    fn settings_changed(&self, entry: &crate::settings::SettingEntry) {
        self.each(|l| l.settings_changed(entry));
    }
    fn hud(&self, snapshot: &super::HudSnapshot) {
        self.each(|l| l.hud(snapshot));
    }
    fn chat(&self, line: &super::ChatLine) {
        self.each(|l| l.chat(line));
    }
    fn unread_chat(&self, count: u32) {
        self.each(|l| l.unread_chat(count));
    }
    fn ui_state(&self, state: &super::UiState) {
        self.each(|l| l.ui_state(state));
    }
    fn open_backpack(&self, emotes: bool) {
        self.each(|l| l.open_backpack(emotes));
    }
    fn loading_detail(&self, detail: &super::LoadingDetail) {
        self.each(|l| l.loading_detail(detail));
    }
    fn avatar_changed(&self, avatar: &super::AvatarState) {
        self.each(|l| l.avatar_changed(avatar));
    }
    fn scene_request(
        &self,
        id: crate::scene_requests::RequestId,
        request: &crate::scene_requests::SceneRequest,
    ) {
        self.each(|l| l.scene_request(id, request));
    }
    fn engine_notice(&self, notice: &super::EngineNotice) {
        self.each(|l| l.engine_notice(notice));
    }
    fn report(&self, payload: &crate::hub::ReportPayload) {
        self.each(|l| l.report(payload));
    }
    fn report_tag(&self, key: &str, value: Option<&str>) {
        self.each(|l| l.report_tag(key, value));
    }
}

/// An `EngineSink` that any thread can push to and the engine's main thread drains each frame.
#[derive(Default)]
pub struct CommandQueue {
    commands: Mutex<VecDeque<EngineCommand>>,
}

impl CommandQueue {
    pub fn drain(&self) -> Vec<EngineCommand> {
        self.commands
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .drain(..)
            .collect()
    }
}

impl EngineSink for CommandQueue {
    fn send(&self, command: EngineCommand) {
        self.commands
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push_back(command);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Rect;

    #[test]
    fn the_queue_drains_in_order_and_empties() {
        let queue = CommandQueue::default();
        queue.send(EngineCommand::ExitWorld);
        queue.send(EngineCommand::ShowAvatarPreview(Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        }));
        queue.send(EngineCommand::SetRendering(false));
        let drained = queue.drain();
        assert_eq!(drained.len(), 3);
        assert_eq!(drained[0], EngineCommand::ExitWorld);
        assert!(queue.drain().is_empty());
    }

    #[test]
    fn a_panicking_listener_is_removed_and_the_others_still_hear_the_event() {
        struct Panics;
        impl AppEvents for Panics {
            fn state_changed(&self, _: &State) {
                panic!("stale handle");
            }
            fn engine_required(&self) {}
            fn avatar_preview_mode(&self, _: bool) {}
            fn scene_crashed(&self) {}
        }
        struct Count(Mutex<u32>);
        impl AppEvents for Count {
            fn state_changed(&self, _: &State) {
                *self.0.lock().unwrap() += 1;
            }
            fn engine_required(&self) {}
            fn avatar_preview_mode(&self, _: bool) {}
            fn scene_crashed(&self) {}
        }

        let events = HostEvents::default();
        let count = Arc::new(Count(Mutex::new(0)));
        events.add_listener(Arc::new(Panics));
        events.add_listener(count.clone());
        events.state_changed(&State::SignedIn);
        events.state_changed(&State::SignedIn);
        assert_eq!(*count.0.lock().unwrap(), 2);
        assert_eq!(events.listeners.read().unwrap().len(), 1);
    }

    #[test]
    fn events_reach_every_listener_until_it_is_removed() {
        struct Count(Mutex<u32>);
        impl AppEvents for Count {
            fn state_changed(&self, _: &State) {
                *self.0.lock().unwrap() += 1;
            }
            fn engine_required(&self) {}
            fn avatar_preview_mode(&self, _: bool) {}
            fn scene_crashed(&self) {}
        }

        let events = HostEvents::default();
        events.state_changed(&State::SignedIn);
        let (shell, host) = (
            Arc::new(Count(Mutex::new(0))),
            Arc::new(Count(Mutex::new(0))),
        );
        let shell_id = events.add_listener(shell.clone());
        events.add_listener(host.clone());
        events.state_changed(&State::SignedIn);
        events.remove_listener(shell_id);
        events.state_changed(&State::SignedIn);
        assert_eq!(*shell.0.lock().unwrap(), 1);
        assert_eq!(*host.0.lock().unwrap(), 2);
    }

    #[test]
    fn what_the_engine_tells_the_player_reaches_the_listeners() {
        #[derive(Default)]
        struct Seen(Mutex<Vec<String>>);
        impl AppEvents for Seen {
            fn state_changed(&self, _: &State) {}
            fn engine_required(&self) {}
            fn avatar_preview_mode(&self, _: bool) {}
            fn scene_crashed(&self) {}
            fn engine_notice(&self, notice: &super::super::EngineNotice) {
                self.0.lock().unwrap().push(format!("{notice:?}"));
            }
            fn scene_request(
                &self,
                id: crate::scene_requests::RequestId,
                _: &crate::scene_requests::SceneRequest,
            ) {
                self.0.lock().unwrap().push(format!("request {id}"));
            }
        }

        let events = HostEvents::default();
        let seen = Arc::new(Seen::default());
        events.add_listener(seen.clone());
        events.engine_notice(&super::super::EngineNotice::Banned);
        events.scene_request(
            4,
            &crate::scene_requests::SceneRequest::OpenExternalUrl { url: "u".into() },
        );
        assert_eq!(*seen.0.lock().unwrap(), vec!["Banned", "request 4"]);
    }
}
