#[no_mangle]
extern "C" fn crash_null_deref() {
    unsafe {
        #[allow(clippy::zero_ptr)]
        let p: *mut i32 = 0 as *mut i32;
        *p = 0;
    }
}

use godot::prelude::*;
#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct DclCrashGenerator {
    counter: i32,
}

#[godot_api]
impl INode for DclCrashGenerator {
    fn process(&mut self, _delta: f64) {
        self.counter += 1;
        if self.counter > 100 {
            crash_null_deref();
        }
    }
}

#[godot_api]
impl DclCrashGenerator {
    /// Triggers a native crash (SIGSEGV) - captured by Godot Sentry SDK.
    /// The crash report is written to disk and sent on next app launch.
    #[func]
    pub fn static_crash() {
        crash_null_deref();
    }

    /// Triggers a Rust panic that is captured by Sentry with full stack trace,
    /// then aborts the process (godot-rust normally catches panics).
    #[func]
    pub fn rust_panic() {
        // Capture the error in Sentry first
        sentry::capture_message(
            "User triggered Rust panic for testing via /rustpanic",
            sentry::Level::Fatal,
        );

        // Flush Sentry events before crashing (wait up to 2 seconds)
        if let Some(client) = sentry::Hub::current().client() {
            client.flush(Some(std::time::Duration::from_secs(2)));
        }

        // Now abort - this bypasses godot-rust's panic handler
        std::process::abort();
    }
}
