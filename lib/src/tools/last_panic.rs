use std::sync::Mutex;

static LAST_PANIC: Mutex<Option<String>> = Mutex::new(None);

/// Chains onto the current panic hook (install after `log_panics::init`) and
/// remembers the message and location of the latest panic, so a scene killed by
/// the poisoned lock it left behind can report what panicked.
pub fn install_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic payload".to_string());
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown location".to_string());
        if let Ok(mut last) = LAST_PANIC.lock() {
            *last = Some(format!("{message} @ {location}"));
        }
        previous(info);
    }));
}

pub fn take() -> Option<String> {
    LAST_PANIC.lock().ok().and_then(|mut last| last.take())
}
