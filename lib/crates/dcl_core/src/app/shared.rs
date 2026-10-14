use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, RwLock},
};

use super::{AppEvents, EngineCommand, EngineSink, State};

/// Forwards the app's events to whoever registered last (the FFI callback interface). Events
/// fire while the app lock is held, so a listener must not call back into the app synchronously.
#[derive(Default)]
pub struct HostEvents {
    listener: RwLock<Option<Arc<dyn AppEvents>>>,
}

impl HostEvents {
    pub fn set_listener(&self, listener: Arc<dyn AppEvents>) {
        *self.listener.write().unwrap_or_else(|e| e.into_inner()) = Some(listener);
    }

    fn with(&self, f: impl FnOnce(&dyn AppEvents)) {
        if let Some(listener) = self
            .listener
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            f(listener.as_ref());
        }
    }
}

impl AppEvents for HostEvents {
    fn state_changed(&self, state: &State) {
        self.with(|l| l.state_changed(state));
    }
    fn engine_required(&self) {
        self.with(|l| l.engine_required());
    }
    fn avatar_preview_mode(&self, on: bool) {
        self.with(|l| l.avatar_preview_mode(on));
    }
    fn scene_crashed(&self) {
        self.with(|l| l.scene_crashed());
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
    fn events_without_a_listener_are_dropped_and_with_one_are_forwarded() {
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
        let count = Arc::new(Count(Mutex::new(0)));
        events.set_listener(count.clone());
        events.state_changed(&State::SignedIn);
        assert_eq!(*count.0.lock().unwrap(), 1);
    }
}
