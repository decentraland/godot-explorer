//! Bridge to the profiler zones of the godotengine fork (Tracy / Perfetto / Instruments backends).
//! The three engine functions are resolved through the GDExtension interface when the library
//! loads; on an engine built without a profiler every call here is a no-op.

use std::ffi::{c_char, CString};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

type ZoneBeginFn = unsafe extern "C" fn(*const c_char, *const c_char);
type ZoneEndFn = unsafe extern "C" fn();
type IsEnabledFn = unsafe extern "C" fn() -> godot::sys::GDExtensionBool;

static ENABLED: AtomicBool = AtomicBool::new(false);
static ZONE_BEGIN: AtomicPtr<()> = AtomicPtr::new(std::ptr::null_mut());
static ZONE_END: AtomicPtr<()> = AtomicPtr::new(std::ptr::null_mut());

/// Resolves the engine entry points. Called from the extension entry symbol, before godot-rust
/// initializes, with the loader Godot hands to the library.
///
/// # Safety
/// `get_proc_address` must be the loader passed by Godot to the extension entry point.
pub unsafe fn init(get_proc_address: godot::sys::GDExtensionInterfaceGetProcAddress) {
    let Some(get_proc_address) = get_proc_address else {
        return;
    };
    let begin = get_proc_address(c"profiler_zone_begin".as_ptr());
    let end = get_proc_address(c"profiler_zone_end".as_ptr());
    let is_enabled = get_proc_address(c"profiler_is_enabled".as_ptr());
    let (Some(begin), Some(end), Some(is_enabled)) = (begin, end, is_enabled) else {
        return;
    };
    let is_enabled: IsEnabledFn = std::mem::transmute(is_enabled);
    if is_enabled() == 0 {
        return;
    }
    ZONE_BEGIN.store(begin as *mut (), Ordering::Relaxed);
    ZONE_END.store(end as *mut (), Ordering::Relaxed);
    ENABLED.store(true, Ordering::Release);
}

#[inline]
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Opens a zone on the calling thread. Pair with [`zone_end`] on the same thread, strictly nested.
pub fn zone_begin(name: &str, text: &str) {
    if !enabled() {
        return;
    }
    let name = CString::new(name).unwrap_or_default();
    let text = CString::new(text).unwrap_or_default();
    // SAFETY: the pointer was stored by `init` from the engine's interface table.
    unsafe {
        let f: ZoneBeginFn = std::mem::transmute(ZONE_BEGIN.load(Ordering::Relaxed));
        f(name.as_ptr(), text.as_ptr());
    }
}

pub fn zone_end() {
    if !enabled() {
        return;
    }
    // SAFETY: see `zone_begin`.
    unsafe {
        let f: ZoneEndFn = std::mem::transmute(ZONE_END.load(Ordering::Relaxed));
        f();
    }
}

/// Zero-length zone: a point marker on the timeline.
pub fn mark(name: &str, text: &str) {
    zone_begin(name, text);
    zone_end();
}

/// Scoped zone, closed on drop. Not `Send`: it must end on the thread that opened it, so never
/// hold one across an `.await`.
pub struct Zone {
    active: bool,
    _thread_bound: PhantomData<*const ()>,
}

impl Zone {
    pub fn new(name: &str, text: &str) -> Self {
        if !enabled() {
            return Self::inactive();
        }
        zone_begin(name, text);
        Self {
            active: true,
            _thread_bound: PhantomData,
        }
    }

    /// Like [`Zone::new`], formatting the (name, text) only when a profiler is attached.
    pub fn lazy(label: impl FnOnce() -> (String, String)) -> Self {
        if !enabled() {
            return Self::inactive();
        }
        let (name, text) = label();
        Self::new(&name, &text)
    }

    fn inactive() -> Self {
        Self {
            active: false,
            _thread_bound: PhantomData,
        }
    }
}

impl Drop for Zone {
    fn drop(&mut self) {
        if self.active {
            zone_end();
        }
    }
}
