use godot::prelude::*;

use crate::godot_classes::dcl_global::DclGlobal;

/// Deliberate SIGSEGV at address 0, the shape of the top Android crash family.
/// Used by the `/instantcrash` (alias `/nativecrash`) and `/delayedcrash` chat
/// commands to exercise the whole crash pipeline on a device: NDK handler,
/// tombstone, server-side symbolication and the exit-reason report on the
/// next launch. `write_volatile` keeps the optimizer from lowering the UB
/// into a trap instruction, which would surface as SIGILL instead.
#[no_mangle]
extern "C" fn crash_null_deref() {
    // SAFETY: intentionally unsound; the crash is the point.
    unsafe { std::ptr::write_volatile(std::ptr::null_mut::<i32>(), 0) }
}

/// The chat commands are gated in GDScript too; this is the last line of
/// defence so a production build can never be crashed on purpose.
fn crash_allowed() -> bool {
    if DclGlobal::is_production() {
        tracing::warn!("debug crash ignored in production builds");
        return false;
    }
    true
}

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct DclCrashGenerator {
    counter: i32,
}

#[godot_api]
impl INode for DclCrashGenerator {
    fn process(&mut self, _delta: f64) {
        self.counter += 1;
        if self.counter > 100 && crash_allowed() {
            crash_null_deref();
        }
    }
}

#[godot_api]
impl DclCrashGenerator {
    #[func]
    pub fn static_crash() {
        if crash_allowed() {
            crash_null_deref();
        }
    }
}
