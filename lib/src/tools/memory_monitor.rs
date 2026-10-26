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
//!     `budget(MemTotal) - (RSS + VmSwap)`, both from `/proc` (no precise per-app
//!     headroom call exists, and JNI is not thread-safe to call here; `/proc`
//!     files are readable from any thread). `MemAvailable` tracks how full the
//!     device is because of everything *else* running on it, which the budget
//!     alone cannot see; the budget counts swapped-out anonymous memory so a
//!     device that pages our cold pages to zram does not read as a recovery.
//!
//! ### What Android does *not* let us see (measured, do not retry)
//!
//! Headroom in MB answers "how much is left", never "is reclaim still keeping
//! up" — and that second question is the one that predicts a kill. On a Galaxy
//! A54 (Android 16) lowmemorykiller killed the app while `MemAvailable` still
//! read 510-570MB, i.e. LEVEL_OK by the bands below, because what ran out was
//! the free-page watermark while reclaim fell behind. The signals that would
//! have shown it coming are all denied to an unprivileged app by SELinux —
//! verified from inside the sandbox with `run-as`:
//!
//! * `/proc/pressure/memory` (PSI, the signal lmkd itself acts on) — denied,
//!   labelled `proc_pressure_mem`. The denial is `dontaudit`, so it does not
//!   even show up as an avc line: the read just fails.
//! * `/proc/vmstat` (direct-reclaim / allocstall counters) — denied.
//! * `/proc/zoneinfo` (the kernel watermarks lmkd compares against) — denied.
//!
//! `ComponentCallbacks2.onTrimMemory` reaches GDScript through the Android
//! plugin's `memory_trim` signal, but since Android 14 the system no longer
//! delivers the `TRIM_MEMORY_RUNNING_*` levels, so it cannot warn of a kill.
//! `ActivityManager.getMemoryInfo()` was measured too: its `availMem` is
//! `MemAvailable` again, and `lowMemory` only compares that to a fixed line.
//!
//! So on 8 GB+ devices the level comes from our *own* footprint
//! (`RssAnon + VmSwap`, the figure Android 17's Memory Limiter caps) against
//! that tier's limit, and `MemAvailable` is ignored: on those phones it sits
//! under 300 MB for minutes while lmkd still has gigabytes of cached apps to
//! kill first (issue #3150). Smaller tiers keep the headroom bands below, as
//! they are the ones that really get killed.
//!
//! What is left to us is `/proc/meminfo` and our own `/proc/self/status`. That
//! is enough to see *our own* growth honestly — the case this monitor exists for
//! — but it cannot anticipate a kill driven by system-wide pressure from other
//! apps. Do not add a signal here without checking it is readable under
//! `run-as` first.
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
use std::sync::Mutex;

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

/// True while the OS has the app paused (screen off, task switch). Set from the
/// main thread in `SceneManager::on_notification`. The heartbeat legitimately
/// stops while paused, so the stall detector has to hold instead of reporting:
/// without this gate a locked screen reads as one continuous "main thread
/// UNRESPONSIVE" for as long as the screen stays off (measured: ~321 s on a
/// Galaxy A54, reported every 2 s).
pub static APP_PAUSED: AtomicBool = AtomicBool::new(false);

/// Our own footprint in MB: `RssAnon + VmSwap` on Android, `phys_footprint` on
/// iOS. Drives the 8 GB+ budget and the footprint record. -1 elsewhere.
pub static OWN_FOOTPRINT_MB: AtomicI32 = AtomicI32::new(-1);

/// GPU memory in MB as last sampled by the Android plugin (`summary.graphics`).
/// Recorded for telemetry only; -1 until set.
pub static GRAPHICS_MB: AtomicI32 = AtomicI32::new(-1);

/// Total device RAM in MB, read once at [`start`]. -1 when unknown.
pub static TOTAL_RAM_MB: AtomicI32 = AtomicI32::new(-1);

/// Session id written into the footprint record, so the next launch can tie
/// the record to the session that produced it.
static SESSION_ID: Mutex<String> = Mutex::new(String::new());

/// Set when a field the throttle does not watch changed (the session id), so
/// the next allowed write happens even without a footprint move.
static RECORD_DIRTY: AtomicBool = AtomicBool::new(false);

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

/// Smallest `MemTotal` (MB) of the 8 GB tier in Android 17's Memory Limiter
/// table. From here up the level follows our own footprint.
#[cfg(any(test, target_os = "android"))]
const OWN_BUDGET_MIN_TOTAL_MB: i32 = 6800;

/// Own-footprint percentage of the budget at which WARNING / CRITICAL fire.
#[cfg(any(test, target_os = "ios", target_os = "android"))]
const OWN_BUDGET_WARNING_PCT: i64 = 85;
#[cfg(any(test, target_os = "ios", target_os = "android"))]
const OWN_BUDGET_CRITICAL_PCT: i64 = 100;

/// Samples a higher raw level must hold before it is published (3 s at
/// POLL_MS): a single load spike must not raise the modal.
#[cfg(any(test, target_os = "ios", target_os = "android"))]
const LEVEL_RAISE_SAMPLES: u32 = 12;
/// Samples a lower raw level must hold before it is published (2 s), so a
/// level hovering on a band edge cannot drop to OK and re-arm the modal.
#[cfg(any(test, target_os = "ios", target_os = "android"))]
const LEVEL_LOWER_SAMPLES: u32 = 8;

/// The footprint record is rewritten only when the footprint or the peak moved
/// this much since the last write, or the level changed…
#[cfg(any(test, target_os = "ios", target_os = "android"))]
const RECORD_MIN_DELTA_MB: i32 = 32;
/// …and never more often than this. The file is overwritten, never appended.
#[cfg(any(test, target_os = "ios", target_os = "android"))]
const RECORD_MIN_INTERVAL_MS: u64 = 5000;

/// Own-footprint budget in MB for a device with `total_mb` of RAM: the visible
/// app limit of Android 17's Memory Limiter (`memory-limiter-config.xml`).
/// 0 below the 8 GB tier, where the headroom bands decide instead.
#[cfg(any(test, target_os = "android"))]
fn own_budget_mb(total_mb: i32) -> i32 {
    match total_mb {
        t if t >= 14336 => 10240,
        t if t >= 9216 => 8192,
        t if t >= OWN_BUDGET_MIN_TOTAL_MB => 5120,
        _ => 0,
    }
}

/// Raw (unheld) pressure level. The iOS `phys_footprint` ceiling wins: it sees
/// GPU memory the headroom signal misses. With an own budget (8 GB+ Android)
/// the device-wide headroom is ignored.
#[cfg(any(test, target_os = "ios", target_os = "android"))]
fn raw_level(
    available_mb: i32,
    footprint_mb: i32,
    footprint_ceiling_mb: i32,
    own_footprint_mb: i32,
    own_budget_mb: i32,
) -> u8 {
    if footprint_ceiling_mb > 0 && footprint_mb >= 0 && footprint_mb >= footprint_ceiling_mb {
        return LEVEL_CRITICAL;
    }

    if own_budget_mb > 0 {
        if own_footprint_mb < 0 {
            return LEVEL_OK;
        }
        let pct = own_footprint_mb as i64 * 100 / own_budget_mb as i64;
        return if pct >= OWN_BUDGET_CRITICAL_PCT {
            LEVEL_CRITICAL
        } else if pct >= OWN_BUDGET_WARNING_PCT {
            LEVEL_WARNING
        } else {
            LEVEL_OK
        };
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

/// Publishes a raw level only once it has held: raising needs
/// `LEVEL_RAISE_SAMPLES` consecutive samples above the current level, lowering
/// `LEVEL_LOWER_SAMPLES` below it. The new level is the least extreme one seen
/// during the streak.
#[cfg(any(test, target_os = "ios", target_os = "android"))]
#[derive(Default)]
struct LevelHold {
    held: u8,
    up_streak: u32,
    up_min: u8,
    down_streak: u32,
    down_max: u8,
}

#[cfg(any(test, target_os = "ios", target_os = "android"))]
impl LevelHold {
    /// Feeds one raw sample; returns true when the held level changed.
    fn update(&mut self, raw: u8) -> bool {
        if raw > self.held {
            self.down_streak = 0;
            self.up_min = if self.up_streak == 0 {
                raw
            } else {
                self.up_min.min(raw)
            };
            self.up_streak += 1;
            if self.up_streak >= LEVEL_RAISE_SAMPLES {
                self.held = self.up_min;
                self.up_streak = 0;
                return true;
            }
        } else if raw < self.held {
            self.up_streak = 0;
            self.down_max = if self.down_streak == 0 {
                raw
            } else {
                self.down_max.max(raw)
            };
            self.down_streak += 1;
            if self.down_streak >= LEVEL_LOWER_SAMPLES {
                self.held = self.down_max;
                self.down_streak = 0;
                return true;
            }
        } else {
            self.up_streak = 0;
            self.down_streak = 0;
        }
        false
    }
}

/// Decides when the footprint record is rewritten (see `RECORD_MIN_*`).
#[cfg(any(test, target_os = "ios", target_os = "android"))]
#[derive(Default)]
struct RecordThrottle {
    last_write_ms: Option<u64>,
    footprint_mb: i32,
    peak_mb: i32,
    level: u8,
}

#[cfg(any(test, target_os = "ios", target_os = "android"))]
impl RecordThrottle {
    fn should_write(
        &self,
        now_ms: u64,
        footprint_mb: i32,
        peak_mb: i32,
        level: u8,
        dirty: bool,
    ) -> bool {
        let Some(last) = self.last_write_ms else {
            return footprint_mb >= 0;
        };
        if now_ms.saturating_sub(last) < RECORD_MIN_INTERVAL_MS {
            return false;
        }
        dirty
            || (footprint_mb - self.footprint_mb).abs() >= RECORD_MIN_DELTA_MB
            || peak_mb >= self.peak_mb + RECORD_MIN_DELTA_MB
            || level != self.level
    }

    fn written(&mut self, now_ms: u64, footprint_mb: i32, peak_mb: i32, level: u8) {
        self.last_write_ms = Some(now_ms);
        self.footprint_mb = footprint_mb;
        self.peak_mb = peak_mb;
        self.level = level;
    }
}

/// Absolute path of the footprint record, set by [`start`].
#[cfg(any(target_os = "ios", target_os = "android"))]
static RECORD_PATH: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Start the background monitor (idempotent). No-op on platforms without an OOM
/// killer we need to anticipate. `record_path` is where the footprint record is
/// kept (an absolute path: the monitor thread cannot resolve `user://`).
pub fn start(record_path: String) {
    // Whole body is mobile-only: on desktop there is no OOM killer to anticipate,
    // so `start()` is a no-op. Keeping the mobile logic inside one cfg block also
    // keeps the idempotency early-return from becoming a trailing no-op on desktop.
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    let _ = record_path;
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        if MONITOR_STARTED.swap(true, Ordering::SeqCst) {
            return;
        }

        // The previous session's record is moved aside before this session's
        // first sample overwrites it; GDScript reports and deletes the `.prev`.
        if std::path::Path::new(&record_path).exists() {
            let _ = std::fs::rename(&record_path, format!("{record_path}.prev"));
        }
        let _ = RECORD_PATH.set(record_path);

        #[cfg(target_os = "ios")]
        {
            // Scale the phys_footprint ceiling to device RAM so the backstop is
            // not hard-coded to one model. sysctl-based, safe to call here (main
            // thread, no Godot API touched).
            let total_ram_mb =
                crate::godot_classes::dcl_ios_plugin::DclIosPlugin::get_total_ram_mb();
            if total_ram_mb > 0 {
                TOTAL_RAM_MB.store(total_ram_mb, Ordering::Relaxed);
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
                TOTAL_RAM_MB.store(total_ram_mb, Ordering::Relaxed);
                emit_log(&format!(
                    "[MemMonitor] total_ram={}MB app_budget={}MB WARNING<={}MB CRITICAL<={}MB own_budget={}MB swap_total={}MB",
                    total_ram_mb,
                    (total_ram_mb as f32 * ANDROID_BUDGET_FRACTION) as i32,
                    WARNING_MB,
                    CRITICAL_MB,
                    own_budget_mb(total_ram_mb),
                    read_meminfo_mb("SwapTotal:")
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

/// Session id stored in the footprint record.
pub fn set_session_id(session_id: &str) {
    if let Ok(mut id) = SESSION_ID.lock() {
        *id = session_id.to_string();
    }
    RECORD_DIRTY.store(true, Ordering::Relaxed);
}

/// GPU memory sampled by the Android plugin, stored in the footprint record.
pub fn set_graphics_mb(mb: i32) {
    GRAPHICS_MB.store(mb, Ordering::Relaxed);
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

/// Own-footprint budget for this device (0 = headroom bands decide).
#[cfg(any(target_os = "ios", target_os = "android"))]
fn current_own_budget_mb() -> i32 {
    #[cfg(target_os = "android")]
    {
        own_budget_mb(TOTAL_RAM_MB.load(Ordering::Relaxed))
    }
    #[cfg(target_os = "ios")]
    {
        0
    }
}

#[cfg(any(target_os = "ios", target_os = "android"))]
fn level_name(level: u8) -> &'static str {
    match level {
        LEVEL_CRITICAL => "critical",
        LEVEL_WARNING => "warning",
        _ => "ok",
    }
}

/// Replace the footprint record in one step: a kill mid-write leaves the
/// previous record intact instead of a truncated one.
#[cfg(any(target_os = "ios", target_os = "android"))]
fn write_record(path: &str, contents: &str) -> std::io::Result<()> {
    let tmp = format!("{path}.tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

/// Wall-clock ms since the epoch, comparable to `ApplicationExitInfo` timestamps.
#[cfg(any(target_os = "ios", target_os = "android"))]
fn epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
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

    // Held level changes (and only those) go through `tracing::warn!` so they
    // reach Godot's warning stream and become Sentry breadcrumbs: the memory
    // trajectory in the minutes before a crash.
    let mut hold = LevelHold::default();
    let mut throttle = RecordThrottle::default();
    let mut peak_mb: i32 = -1;
    let mut peak_ms: u64 = 0;
    let started = std::time::Instant::now();

    loop {
        let available = read_available_mb();
        let footprint = read_footprint_mb();

        #[cfg(target_os = "android")]
        let (rss_anon, swap) = (
            read_self_status_mb("RssAnon:"),
            read_self_status_mb("VmSwap:"),
        );
        #[cfg(target_os = "android")]
        let own = if rss_anon >= 0 {
            rss_anon + swap.max(0)
        } else {
            -1
        };
        #[cfg(target_os = "ios")]
        let (rss_anon, swap, own) = (-1, -1, footprint);

        #[cfg(target_os = "android")]
        SWAPPED_MB.store(swap, Ordering::Relaxed);

        let budget = current_own_budget_mb();
        let raw = raw_level(
            available,
            footprint,
            FOOTPRINT_CRITICAL_MB.load(Ordering::Relaxed),
            own,
            budget,
        );
        let previous = hold.held;
        let changed = hold.update(raw);
        let level = hold.held;

        AVAILABLE_MB.store(available, Ordering::Relaxed);
        FOOTPRINT_MB.store(footprint, Ordering::Relaxed);
        OWN_FOOTPRINT_MB.store(own, Ordering::Relaxed);
        PRESSURE_LEVEL.store(level, Ordering::Relaxed);

        if changed {
            tracing::warn!(
                "[MemMonitor] pressure {} -> {} (available={}MB footprint={}MB own={}MB budget={}MB)",
                level_name(previous),
                level_name(level),
                available,
                footprint,
                own,
                budget
            );
        }

        // ~Every 2s at any level, at once on a held change, and every sample
        // only after "Continue anyway" (VERBOSE): a CRITICAL session no longer
        // floods the log with four lines a second.
        if changed || VERBOSE.load(Ordering::Relaxed) || tick % 8 == 0 {
            #[cfg(target_os = "android")]
            let detail =
                format!(" swap={swap}MB rss_anon={rss_anon}MB own={own}MB budget={budget}MB");
            #[cfg(not(target_os = "android"))]
            let detail = "";
            emit_log(&format!(
                "[MemMonitor] available={}MB footprint={}MB{} level={} raw={}",
                available, footprint, detail, level, raw
            ));
        }

        let now_ms = epoch_ms();
        if own > peak_mb {
            peak_mb = own;
            peak_ms = now_ms;
        }
        let since_start_ms = started.elapsed().as_millis() as u64;
        if let Some(path) = RECORD_PATH.get() {
            let dirty = RECORD_DIRTY.load(Ordering::Relaxed);
            if throttle.should_write(since_start_ms, own, peak_mb, level, dirty) {
                RECORD_DIRTY.store(false, Ordering::Relaxed);
                let record = serde_json::json!({
                    "v": 1,
                    "written_ms": now_ms,
                    "session_id": SESSION_ID.lock().map(|id| id.clone()).unwrap_or_default(),
                    "total_ram_mb": TOTAL_RAM_MB.load(Ordering::Relaxed),
                    "rss_anon_mb": rss_anon,
                    "swap_mb": swap,
                    "graphics_mb": GRAPHICS_MB.load(Ordering::Relaxed),
                    "footprint_mb": own,
                    "peak_footprint_mb": peak_mb,
                    "peak_ms": peak_ms,
                    "level": level,
                    "budget_mb": budget,
                });
                if let Err(e) = write_record(path, &record.to_string()) {
                    emit_log(&format!("[MemMonitor] footprint record write failed: {e}"));
                }
                // Marked written even on failure so a broken path retries at
                // the throttled rate, not every sample.
                throttle.written(since_start_ms, own, peak_mb, level);
            }
        }

        // Main-thread stall detection: independent of memory, since the Genesis
        // Plaza freeze happens with plenty of headroom (a CPU/GPU/lock stall, not
        // an OOM). This survives the freeze because it runs off the main thread.
        let hb = MAIN_THREAD_HEARTBEAT.load(Ordering::Relaxed);
        if APP_PAUSED.load(Ordering::Relaxed) {
            // Paused by the OS: the main loop is stopped on purpose, so a frozen
            // heartbeat means nothing here. Hold the accounting rather than report
            // a "freeze" that lasts as long as the screen is off.
            last_hb = hb;
            stall_ms = 0;
            stall_reported = false;
            last_report_ms = 0;
        } else if hb != last_hb {
            if stall_reported {
                emit_log(&format!(
                    "[MainThreadStall] RECOVERED after ~{}ms (available={}MB)",
                    stall_ms, available
                ));
                tracing::warn!(
                    "[MainThreadStall] recovered after ~{}ms (available={}MB)",
                    stall_ms,
                    available
                );
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
                // Breadcrumb only on the first crossing; the re-reports stay
                // in the raw log so a long freeze does not flood the trail.
                if !stall_reported {
                    tracing::warn!(
                        "[MainThreadStall] main thread unresponsive ~{}ms (available={}MB footprint={}MB level={})",
                        stall_ms,
                        available,
                        footprint,
                        level
                    );
                }
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
    // Charge the budget for what the app *owns*, not for what is resident.
    // Android pushes cold anonymous pages to zram / RAM Plus, which drops RSS
    // without the app freeing anything: measured on a Galaxy A54, 2816MB of
    // allocations with ~800MB swapped out made this headroom "recover" from
    // 533MB to 1076MB with nothing released — a WARNING that disarms itself
    // while the app is still holding every byte. VmSwap puts them back on the
    // bill; it is 0 on a device without swap, leaving the old behaviour.
    let swap_mb = read_self_status_mb("VmSwap:").max(0);
    let budget = (total_mb as f32 * ANDROID_BUDGET_FRACTION) as i32;
    let budget_headroom = (budget - rss_mb - swap_mb).max(0);

    // MemAvailable is absent on pre-3.14 kernels; fall back to the budget alone.
    let system_headroom = read_meminfo_mb("MemAvailable:");
    if system_headroom < 0 {
        return budget_headroom;
    }
    budget_headroom.min(system_headroom)
}

/// Anonymous memory of ours the kernel has pushed to zram / RAM Plus, in MB.
/// Published so a log line stays self-explanatory: `footprint` is RSS (what lmkd
/// reports when it kills us) while the budget headroom also charges this, so
/// without it the two numbers cannot be reconciled from a capture. -1 elsewhere.
pub static SWAPPED_MB: AtomicI32 = AtomicI32::new(-1);

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
/// -1 when the file or the key is unavailable.
#[cfg(target_os = "android")]
fn read_meminfo_mb(key: &str) -> i32 {
    read_kb_field_mb("/proc/meminfo", key)
}

/// Read a `/proc/self/status` field (`"VmSwap:"`, `"VmRSS:"`, ...) in MB.
/// -1 when the file or the key is unavailable.
#[cfg(target_os = "android")]
fn read_self_status_mb(key: &str) -> i32 {
    read_kb_field_mb("/proc/self/status", key)
}

/// Read a `<key> <value> kB` field out of a /proc file, in MB. -1 when the file
/// or the key is unavailable. These files report kB.
#[cfg(target_os = "android")]
fn read_kb_field_mb(path: &str, key: &str) -> i32 {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return -1;
    };
    for line in contents.lines() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_budget_follows_memory_limiter_tiers() {
        assert_eq!(own_budget_mb(3725), 0);
        assert_eq!(own_budget_mb(6799), 0);
        assert_eq!(own_budget_mb(6800), 5120);
        assert_eq!(own_budget_mb(7468), 5120);
        assert_eq!(own_budget_mb(9216), 8192);
        assert_eq!(own_budget_mb(14336), 10240);
        assert_eq!(own_budget_mb(24000), 10240);
    }

    #[test]
    fn eight_gb_ignores_device_headroom() {
        // Pixel 7 at the #3150 popup: 291MB available, own footprint ~2.4GB.
        assert_eq!(raw_level(291, 2112, 0, 2450, 5120), LEVEL_OK);
        assert_eq!(raw_level(291, 2112, 0, 4352, 5120), LEVEL_WARNING);
        assert_eq!(raw_level(900, 2112, 0, 5120, 5120), LEVEL_CRITICAL);
        assert_eq!(raw_level(291, 2112, 0, -1, 5120), LEVEL_OK);
    }

    #[test]
    fn small_tiers_keep_headroom_bands() {
        assert_eq!(raw_level(1109, 569, 0, 600, 0), LEVEL_OK);
        assert_eq!(raw_level(493, 801, 0, 900, 0), LEVEL_WARNING);
        assert_eq!(raw_level(295, 880, 0, 950, 0), LEVEL_CRITICAL);
        assert_eq!(raw_level(-1, 880, 0, 950, 0), LEVEL_OK);
    }

    #[test]
    fn ios_footprint_ceiling_wins() {
        assert_eq!(raw_level(2400, 2900, 2843, 2900, 0), LEVEL_CRITICAL);
        assert_eq!(raw_level(2400, 1700, 2843, 1700, 0), LEVEL_OK);
    }

    #[test]
    fn single_spike_does_not_raise() {
        let mut hold = LevelHold::default();
        // run 3 on the Pixel: 484 -> 177 -> 212 -> 311MB within 0.5s
        for raw in [LEVEL_WARNING, LEVEL_CRITICAL, LEVEL_CRITICAL, LEVEL_WARNING] {
            assert!(!hold.update(raw));
        }
        assert_eq!(hold.held, LEVEL_OK);
    }

    #[test]
    fn sustained_level_raises_after_hold() {
        let mut hold = LevelHold::default();
        for _ in 0..LEVEL_RAISE_SAMPLES - 1 {
            assert!(!hold.update(LEVEL_CRITICAL));
        }
        assert!(hold.update(LEVEL_CRITICAL));
        assert_eq!(hold.held, LEVEL_CRITICAL);
    }

    #[test]
    fn mixed_streak_raises_to_least_extreme_level() {
        let mut hold = LevelHold::default();
        for i in 0..LEVEL_RAISE_SAMPLES {
            hold.update(if i % 3 == 0 {
                LEVEL_WARNING
            } else {
                LEVEL_CRITICAL
            });
        }
        assert_eq!(hold.held, LEVEL_WARNING);
    }

    #[test]
    fn flicker_between_warning_and_critical_never_reaches_ok() {
        let mut hold = LevelHold::default();
        for _ in 0..LEVEL_RAISE_SAMPLES {
            hold.update(LEVEL_CRITICAL);
        }
        for i in 0..200 {
            hold.update(if i % 2 == 0 {
                LEVEL_WARNING
            } else {
                LEVEL_CRITICAL
            });
            assert_ne!(hold.held, LEVEL_OK);
        }
    }

    #[test]
    fn recovery_lowers_after_hold() {
        let mut hold = LevelHold::default();
        for _ in 0..LEVEL_RAISE_SAMPLES {
            hold.update(LEVEL_CRITICAL);
        }
        for _ in 0..LEVEL_LOWER_SAMPLES - 1 {
            assert!(!hold.update(LEVEL_OK));
        }
        assert!(hold.update(LEVEL_OK));
        assert_eq!(hold.held, LEVEL_OK);
    }

    #[test]
    fn record_first_write_needs_a_footprint() {
        let t = RecordThrottle::default();
        assert!(!t.should_write(0, -1, -1, LEVEL_OK, false));
        assert!(t.should_write(0, 1500, 1500, LEVEL_OK, false));
    }

    #[test]
    fn record_is_throttled_and_needs_a_real_change() {
        let mut t = RecordThrottle::default();
        t.written(0, 1500, 1500, LEVEL_OK);
        // Big move, but too soon.
        assert!(!t.should_write(RECORD_MIN_INTERVAL_MS - 1, 2000, 2000, LEVEL_OK, false));
        // Late enough, but too small a move.
        assert!(!t.should_write(
            60_000,
            1500 + RECORD_MIN_DELTA_MB - 1,
            1500,
            LEVEL_OK,
            false
        ));
        // Late enough and a real move, a new peak, or a level change.
        assert!(t.should_write(60_000, 1500 + RECORD_MIN_DELTA_MB, 1500, LEVEL_OK, false));
        assert!(t.should_write(60_000, 1500, 1500 + RECORD_MIN_DELTA_MB, LEVEL_OK, false));
        assert!(t.should_write(60_000, 1500, 1500, LEVEL_WARNING, false));
    }

    #[test]
    fn deferred_level_change_is_written_once_the_interval_passes() {
        let mut t = RecordThrottle::default();
        t.written(0, 1500, 1500, LEVEL_OK);
        assert!(!t.should_write(1000, 1500, 1500, LEVEL_CRITICAL, false));
        assert!(t.should_write(RECORD_MIN_INTERVAL_MS, 1500, 1500, LEVEL_CRITICAL, false));
    }

    #[test]
    fn dirty_record_is_written_without_a_footprint_move_but_still_throttled() {
        let mut t = RecordThrottle::default();
        t.written(0, 1500, 1500, LEVEL_OK);
        assert!(!t.should_write(1000, 1500, 1500, LEVEL_OK, true));
        assert!(t.should_write(RECORD_MIN_INTERVAL_MS, 1500, 1500, LEVEL_OK, true));
    }
}
