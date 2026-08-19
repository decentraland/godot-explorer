//! Background memory-pressure monitor (issue #2002).
//!
//! Why a dedicated OS thread: on mobile the app is killed abruptly when it
//! exceeds its per-process memory limit (iOS jetsam / Android lowmemorykiller).
//! The dangerous moment — loading a heavy scene — is exactly when the *main
//! thread freezes* (a big synchronous upload blocks the frame, or the kernel
//! compresses/pages memory). A watchdog living on the SceneTree / main loop
//! cannot react while the main thread is frozen.
//!
//! This monitor runs on its own thread, sampling memory every `POLL_MS`
//! independently of the main thread, so detection keeps working even during a
//! freeze. It publishes the result into atomics that the main loop
//! (`SceneManager::handle_memory_pressure`) consumes to evict scenes / show the
//! low-memory modal the instant it gets a slice of CPU.
//!
//! ## Memory signal (two metrics, deliberately)
//!
//! * **Primary — headroom:** how many MB the app can still allocate before the
//!   OS kills it.
//!   - iOS: `os_proc_available_memory()` — bytes left before jetsam. Cheap,
//!     off-thread-safe, and already reflects the *dynamic* per-device limit.
//!   - Android: the smaller of the kernel's `MemAvailable` and a per-app budget
//!     `budget(MemTotal) - RSS`, both from `/proc` (no precise per-app headroom
//!     call exists, and JNI is not thread-safe to call here; `/proc` files are
//!     readable from any thread). `MemAvailable` is the signal that tracks what
//!     lowmemorykiller acts on — the budget alone cannot see a device that is
//!     already full because of everything *else* running on it.
//!
//! * **Secondary hard-stop — `phys_footprint`** (iOS only, via
//!   `proc_pid_rusage`). This is the value jetsam actually compares against and,
//!   unlike `os_proc_available_memory`, it *includes* GPU / IOSurface / texture
//!   memory. On GPU-heavy scenes `os_proc_available_memory` was measured to
//!   over-report headroom by ~180 MB on an iPhone 13 (that GPU memory is
//!   invisible to it). So we also read `phys_footprint` and force CRITICAL once
//!   it crosses a device-scaled ceiling — catching the GPU-driven OOM that the
//!   headroom signal alone would miss.
//!
//! On desktop there is no OOM killer to anticipate, so the monitor is not
//! started and both readers return -1.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU8, Ordering};

/// Available headroom in MB (bytes the app can still allocate before the OS
/// kills it). -1 when unknown / not yet sampled.
pub static AVAILABLE_MB: AtomicI32 = AtomicI32::new(-1);

/// `phys_footprint` in MB — the real total the OS charges the process, GPU
/// memory included (the value jetsam compares against). iOS only; -1 elsewhere
/// or before the first sample.
pub static FOOTPRINT_MB: AtomicI32 = AtomicI32::new(-1);

/// Pressure level published by the monitor thread: 0 = OK, 1 = WARNING, 2 = CRITICAL.
pub static PRESSURE_LEVEL: AtomicU8 = AtomicU8::new(0);

/// Incremented every physics frame on the main thread (top of
/// `SceneManager::physics_process`). The monitor thread watches it: if it stops
/// advancing while the app is running, the main thread is frozen — exactly the
/// Genesis Plaza "stuck in loading" symptom, which we now know is NOT memory.
pub static MAIN_THREAD_HEARTBEAT: AtomicU32 = AtomicU32::new(0);

/// When true, the monitor logs EVERY sample (not just under pressure / every
/// ~2s). Enabled when the user taps "Continue anyway" on the low-memory warning,
/// so we capture the headroom trajectory at full resolution right up to a
/// potential OOM crash — to learn how far a heavy scene gets before dying.
pub static VERBOSE: AtomicBool = AtomicBool::new(false);

/// `phys_footprint` ceiling in MB above which we force CRITICAL, computed at
/// [`start`] from device RAM so it scales across devices. 0 = disabled (unknown
/// RAM / non-iOS): the headroom signal governs alone.
#[cfg(any(target_os = "ios", target_os = "android"))]
static FOOTPRINT_CRITICAL_MB: AtomicI32 = AtomicI32::new(0);

#[cfg(any(target_os = "ios", target_os = "android"))]
static MONITOR_STARTED: AtomicBool = AtomicBool::new(false);

pub const LEVEL_OK: u8 = 0;
pub const LEVEL_WARNING: u8 = 1;
pub const LEVEL_CRITICAL: u8 = 2;

/// Sampling period of the monitor thread.
#[cfg(any(target_os = "ios", target_os = "android"))]
const POLL_MS: u64 = 250;

/// Headroom thresholds in MB. Tuned conservatively for a ~4GB iPhone 13; the
/// bands are wide so eviction (not instantaneous) has time to recover headroom
/// before the OS limit is hit. `CRITICAL_MB` sits well above the ~180MB
/// effective death point measured on GPU-heavy scenes, absorbing the GPU memory
/// that `os_proc_available_memory` does not count (the `phys_footprint`
/// hard-stop covers the rest).
pub const WARNING_MB: i32 = 500;
pub const CRITICAL_MB: i32 = 300;

/// Fraction of total device RAM used as the `phys_footprint` CRITICAL ceiling.
/// On an iPhone 13 (~3.7GB RAM) jetsam strikes around ~2.1GB (~0.56 of RAM), so
/// 0.50 fires with headroom to spare and scales up on higher-RAM devices where
/// a healthy footprint can legitimately be larger.
#[cfg(target_os = "ios")]
const FOOTPRINT_CRITICAL_FRACTION: f32 = 0.50;

/// Fraction of total RAM assumed to be the per-app budget on Android, where no
/// precise headroom call exists. Approximate.
#[cfg(target_os = "android")]
const ANDROID_BUDGET_FRACTION: f32 = 0.55;

/// Start the background monitor (idempotent). No-op on platforms without an OOM
/// killer we need to anticipate.
pub fn start() {
    // Whole body is mobile-only: on desktop there is no OOM killer to anticipate,
    // so `start()` is a no-op. Keeping the mobile logic inside one cfg block also
    // keeps the idempotency early-return from becoming a trailing no-op on desktop.
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        if MONITOR_STARTED.swap(true, Ordering::SeqCst) {
            return;
        }

        #[cfg(target_os = "ios")]
        {
            // Scale the phys_footprint ceiling to device RAM so the backstop is
            // not hard-coded to one model. sysctl-based, safe to call here (main
            // thread, no Godot API touched).
            let total_ram_mb =
                crate::godot_classes::dcl_ios_plugin::DclIosPlugin::get_total_ram_mb();
            if total_ram_mb > 0 {
                let ceiling = (total_ram_mb as f32 * FOOTPRINT_CRITICAL_FRACTION) as i32;
                FOOTPRINT_CRITICAL_MB.store(ceiling, Ordering::Relaxed);
                emit_log(&format!(
                    "[MemMonitor] total_ram={}MB -> phys_footprint CRITICAL ceiling={}MB",
                    total_ram_mb, ceiling
                ));
            }
        }

        #[cfg(target_os = "android")]
        {
            // Record the device's memory shape once, so a logcat capture is
            // self-contained: total RAM, the per-app budget, and the headroom
            // each level fires at. Reading /proc is safe from any thread.
            let total_ram_mb = read_meminfo_mb("MemTotal:");
            if total_ram_mb > 0 {
                emit_log(&format!(
                    "[MemMonitor] total_ram={}MB app_budget={}MB WARNING<={}MB CRITICAL<={}MB",
                    total_ram_mb,
                    (total_ram_mb as f32 * ANDROID_BUDGET_FRACTION) as i32,
                    WARNING_MB,
                    CRITICAL_MB
                ));
            }
        }

        let _ = std::thread::Builder::new()
            .name("dcl-mem-monitor".into())
            .spawn(monitor_loop);
        emit_log("[MemMonitor] background memory monitor started");
    }
}

/// Toggle full-resolution logging (see [`VERBOSE`]).
pub fn set_verbose(on: bool) {
    VERBOSE.store(on, Ordering::Relaxed);
    emit_log(&format!(
        "[MemMonitor] verbose logging {}",
        if on { "ON" } else { "off" }
    ));
}

/// Current available headroom in MB, or -1 when it cannot be determined.
/// Safe to call from any thread.
pub fn available_memory_mb() -> i32 {
    AVAILABLE_MB.load(Ordering::Relaxed)
}

/// Current process resident memory in MB (RSS on Linux/Android/macOS,
/// `phys_footprint` on iOS), or -1 when it cannot be determined (e.g. Windows,
/// where the caller should fall back to another source).
///
/// This is the ONE process-memory reader in the codebase: the mobile pressure
/// monitor loop and `benchmark_report` both build on it — extend it here
/// instead of adding another platform-specific copy.
///
/// Synchronous, cheap, and safe to call from the main thread every frame. Unlike
/// Godot's `Performance.MEMORY_STATIC` / `OS.get_static_memory_usage()` — which
/// only report Godot's OWN allocator and are compiled out of release / mobile
/// export templates (returning 0 there) — this reads the OS's real figure for
/// the whole process, so it stays correct on mobile release builds.
pub fn used_memory_mb() -> i32 {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // /proc/self/statm field 1 (0-indexed) = resident set size in pages.
        let Ok(statm) = std::fs::read_to_string("/proc/self/statm") else {
            return -1;
        };
        let Some(rss_pages) = statm
            .split_whitespace()
            .nth(1)
            .and_then(|v| v.parse::<i64>().ok())
        else {
            return -1;
        };
        if rss_pages <= 0 {
            return -1;
        }
        let page_size: i64 = 4096;
        return (rss_pages * page_size / (1024 * 1024)) as i32;
    }

    #[cfg(target_os = "ios")]
    {
        // phys_footprint via proc_pid_rusage — the real charge the OS tracks,
        // GPU/IOSurface memory included (same value jetsam and Xcode report).
        let mut info = std::mem::MaybeUninit::<libc::rusage_info_v2>::zeroed();
        // SAFETY: proc_pid_rusage fills a v2 rusage struct for our own pid; the
        // buffer is a correctly-sized, zeroed rusage_info_v2.
        let ret = unsafe {
            libc::proc_pid_rusage(
                std::process::id() as libc::c_int,
                libc::RUSAGE_INFO_V2,
                info.as_mut_ptr() as *mut libc::rusage_info_t,
            )
        };
        if ret != 0 {
            return -1;
        }
        // SAFETY: proc_pid_rusage returned 0, so `info` is initialized.
        let info = unsafe { info.assume_init() };
        return (info.ri_phys_footprint / (1024 * 1024)) as i32;
    }

    #[cfg(target_os = "macos")]
    {
        // Resident size via mach_task_basic_info for the current task.
        #[repr(C)]
        struct MachTaskBasicInfo {
            virtual_size: u64,
            resident_size: u64,
            resident_size_max: u64,
            user_time: u64,
            system_time: u64,
            policy: i32,
            suspend_count: i32,
        }

        extern "C" {
            fn mach_task_self() -> u32;
            fn task_info(
                target_task: u32,
                flavor: i32,
                task_info_out: *mut MachTaskBasicInfo,
                task_info_out_cnt: *mut u32,
            ) -> i32;
        }

        const MACH_TASK_BASIC_INFO: i32 = 20;
        const MACH_TASK_BASIC_INFO_COUNT: u32 =
            (std::mem::size_of::<MachTaskBasicInfo>() / std::mem::size_of::<u32>()) as u32;

        // SAFETY: task_info fills a MachTaskBasicInfo for the current task; the
        // buffer is correctly sized and `count` matches its u32-word length.
        unsafe {
            let mut info = std::mem::MaybeUninit::<MachTaskBasicInfo>::uninit();
            let mut count = MACH_TASK_BASIC_INFO_COUNT;
            let result = task_info(
                mach_task_self(),
                MACH_TASK_BASIC_INFO,
                info.as_mut_ptr(),
                &mut count,
            );
            if result == 0 {
                let info = info.assume_init();
                return (info.resident_size / (1024 * 1024)) as i32;
            }
        }
        return -1;
    }

    // Windows / other: no reader wired — caller falls back to another source.
    #[allow(unreachable_code)]
    {
        -1
    }
}

/// Write a line to stdout AND stderr, both flushed immediately. The Godot
/// `print()` path does not reach the iOS `devicectl` console (and is lost when
/// the app freezes), but native writes to fd 1/2 do — this is the only logging
/// that survives the freeze we are trying to diagnose. Cheap; safe off-thread.
#[cfg(not(target_os = "android"))]
pub fn emit_log(line: &str) {
    use std::io::Write;
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(line.as_bytes());
    let _ = stdout.write_all(b"\n");
    let _ = stdout.flush();
    let mut stderr = std::io::stderr();
    let _ = stderr.write_all(line.as_bytes());
    let _ = stderr.write_all(b"\n");
    let _ = stderr.flush();
}

/// Android routes a process's stdout/stderr to /dev/null, so the fd 1/2 writes
/// used everywhere else reach nothing on device — every `[MemMonitor]` and
/// `[MainThreadStall]` line was invisible in logcat, warnings and evictions
/// included. Write through liblog instead. It is thread-safe and, unlike Godot's
/// `print()`, safe to call from this thread: the monitor exists to keep
/// reporting while the main thread is frozen, so it must not touch the Godot API.
/// The `godot` tag matches the engine's own so these lines land in the same
/// logcat stream as the rest of the app's output.
#[cfg(target_os = "android")]
pub fn emit_log(line: &str) {
    // <android/log.h>: ANDROID_LOG_INFO
    const ANDROID_LOG_INFO: i32 = 4;

    #[link(name = "log")]
    extern "C" {
        fn __android_log_write(
            prio: i32,
            tag: *const std::os::raw::c_char,
            text: *const std::os::raw::c_char,
        ) -> i32;
    }

    // Interior NULs cannot be logged; drop the line rather than panic.
    let (Ok(tag), Ok(msg)) = (
        std::ffi::CString::new("godot"),
        std::ffi::CString::new(line),
    ) else {
        return;
    };

    // SAFETY: both pointers are NUL-terminated CStrings that outlive the call.
    unsafe {
        __android_log_write(ANDROID_LOG_INFO, tag.as_ptr(), msg.as_ptr());
    }
}

/// Compute the pressure level from the two metrics. The `phys_footprint`
/// hard-stop takes precedence: it sees GPU memory the headroom signal misses.
#[cfg(any(target_os = "ios", target_os = "android"))]
fn level_for(available_mb: i32, footprint_mb: i32) -> u8 {
    let ceiling = FOOTPRINT_CRITICAL_MB.load(Ordering::Relaxed);
    if ceiling > 0 && footprint_mb >= 0 && footprint_mb >= ceiling {
        return LEVEL_CRITICAL;
    }

    if available_mb < 0 {
        // Headroom unknown and footprint below ceiling (or unavailable): can't
        // assert pressure.
        return LEVEL_OK;
    }
    if available_mb <= CRITICAL_MB {
        LEVEL_CRITICAL
    } else if available_mb <= WARNING_MB {
        LEVEL_WARNING
    } else {
        LEVEL_OK
    }
}

#[cfg(any(target_os = "ios", target_os = "android"))]
fn monitor_loop() {
    let mut tick: u64 = 0;

    // Main-thread stall detection state.
    let mut last_hb: u32 = 0;
    let mut seen_running = false; // main loop has ticked at least once
    let mut stall_ms: u64 = 0;
    let mut stall_reported = false;
    let mut last_report_ms: u64 = 0;

    loop {
        let available = read_available_mb();
        let footprint = read_footprint_mb();
        let level = level_for(available, footprint);

        AVAILABLE_MB.store(available, Ordering::Relaxed);
        FOOTPRINT_MB.store(footprint, Ordering::Relaxed);
        PRESSURE_LEVEL.store(level, Ordering::Relaxed);

        // Log every sample under pressure or when verbose; otherwise ~every 2s,
        // so we can watch the trajectory right up to (and through) a main-thread
        // freeze or an eventual OOM after "Continue anyway".
        if level > 0 || VERBOSE.load(Ordering::Relaxed) || tick % 8 == 0 {
            emit_log(&format!(
                "[MemMonitor] available={}MB footprint={}MB level={}",
                available, footprint, level
            ));
        }

        // Main-thread stall detection: independent of memory, since the Genesis
        // Plaza freeze happens with plenty of headroom (a CPU/GPU/lock stall, not
        // an OOM). This survives the freeze because it runs off the main thread.
        let hb = MAIN_THREAD_HEARTBEAT.load(Ordering::Relaxed);
        if hb != last_hb {
            if stall_reported {
                emit_log(&format!(
                    "[MainThreadStall] RECOVERED after ~{}ms (available={}MB)",
                    stall_ms, available
                ));
            }
            last_hb = hb;
            stall_ms = 0;
            stall_reported = false;
            last_report_ms = 0;
            seen_running = true;
        } else if seen_running {
            stall_ms += POLL_MS;
            // Report once we cross ~1s frozen, then re-report ~every 2s so we can
            // see how long the freeze lasts and the memory headroom during it.
            if stall_ms >= 1000 && (!stall_reported || stall_ms - last_report_ms >= 2000) {
                emit_log(&format!(
                    "[MainThreadStall] main thread UNRESPONSIVE ~{}ms (available={}MB footprint={}MB level={})",
                    stall_ms, available, footprint, level
                ));
                stall_reported = true;
                last_report_ms = stall_ms;
            }
        }

        tick = tick.wrapping_add(1);
        std::thread::sleep(std::time::Duration::from_millis(POLL_MS));
    }
}

// ============================================================================
// Platform readers
// ============================================================================

#[cfg(target_os = "ios")]
fn read_available_mb() -> i32 {
    extern "C" {
        // iOS 13+. Returns bytes the app can still allocate before the OS
        // terminates it; 0 when not determinable (e.g. simulator).
        fn os_proc_available_memory() -> usize;
    }
    let bytes = unsafe { os_proc_available_memory() };
    if bytes == 0 {
        -1
    } else {
        (bytes / (1024 * 1024)) as i32
    }
}

#[cfg(target_os = "ios")]
fn read_footprint_mb() -> i32 {
    // On iOS the shared reader IS `phys_footprint` (see `used_memory_mb`).
    used_memory_mb()
}

#[cfg(target_os = "android")]
fn read_available_mb() -> i32 {
    // Two independent limits; the binding one is whichever is smaller.
    //
    // * System headroom (`MemAvailable`): what the kernel thinks it can hand out
    //   without reclaiming. This is the quantity lowmemorykiller actually acts
    //   on, and it moves with everything else running on the device.
    // * Per-app budget (`budget(MemTotal) - RSS`): our own share of total RAM, so
    //   one app still cannot grow without bound on an otherwise-idle device.
    //
    // The budget alone was blind to system pressure: it is a function of our own
    // RSS and a constant, so it cannot see a device that is already full. On a
    // 4GB phone it scored ~760MB of headroom (LEVEL_OK) at the instant
    // lowmemorykiller killed the app at ~1.3GB RSS — WARNING would have needed
    // ~1.5GB RSS and CRITICAL ~1.7GB, both *above* the point where the OS kills
    // us, so the monitor could never fire and no eviction or warning ever ran.
    let rss_mb = used_memory_mb();
    if rss_mb < 0 {
        return -1;
    }

    let total_mb = read_meminfo_mb("MemTotal:");
    if total_mb <= 0 {
        return -1;
    }
    let budget = (total_mb as f32 * ANDROID_BUDGET_FRACTION) as i32;
    let budget_headroom = (budget - rss_mb).max(0);

    // MemAvailable is absent on pre-3.14 kernels; fall back to the budget alone.
    let system_headroom = read_meminfo_mb("MemAvailable:");
    if system_headroom < 0 {
        return budget_headroom;
    }
    budget_headroom.min(system_headroom)
}

#[cfg(target_os = "android")]
fn read_footprint_mb() -> i32 {
    // No `phys_footprint` equivalent on Android, but RSS is what lowmemorykiller
    // reports when it kills us, so report it: it is the number the low-memory
    // warning shows the user and the one to compare against a kill line in
    // logcat. `FOOTPRINT_CRITICAL_MB` is never set on Android, so this stays
    // reporting-only and does not add a second CRITICAL trigger.
    used_memory_mb()
}

/// Read a `/proc/meminfo` field (`"MemTotal:"`, `"MemAvailable:"`, ...) in MB.
/// -1 when the file or the key is unavailable. Values there are in kB.
#[cfg(target_os = "android")]
fn read_meminfo_mb(key: &str) -> i32 {
    let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") else {
        return -1;
    };
    for line in meminfo.lines() {
        if let Some(rest) = line.strip_prefix(key) {
            if let Some(kb) = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<i64>().ok())
            {
                return (kb / 1024) as i32;
            }
        }
    }
    -1
}
