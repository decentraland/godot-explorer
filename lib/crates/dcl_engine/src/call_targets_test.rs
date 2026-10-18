//! Calls by name (`call("x")`, `call_deferred("x")`, `callable("x")`) are checked by Godot only
//! when they run, so a renamed or removed target, or a static `#[func]` called on an instance,
//! fails silently or panics in the field. This scans the sources and fails on any such target
//! that is neither an instance `#[func]` / `#[signal]` of ours nor listed as external.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

/// Names that belong to the engine fork or the native plugins (JNI / Swift singletons), which
/// this crate cannot see. Engine builtins are called typed, never by name.
const EXTERNAL: &[&str] = &[
    // native plugins
    "GetDuration",
    "GetPosition",
    "GetVideoHeight",
    "GetVideoWidth",
    "HasVideoSizeChanged",
    "InitSurface",
    "IsPlaying",
    "Pause",
    "Play",
    "Release",
    "SetLooping",
    "SetPlaybackRate",
    "SetPosition",
    "SetSourceLocal",
    "SetSourceUrl",
    "SetVolume",
    "Stop",
    "add_calendar_event",
    "can_make_payments",
    "current_environment",
    "debugBlockUiThread",
    // engine fork RenderingServer (decentraland/godotengine#27), probed in engine/engine_fork.rs
    "enable_async_shader_compilation",
    "finish_transaction",
    "getDeviceAnchorId",
    "get_apns_token",
    "get_deeplink_url",
    "get_device_anchor_id",
    "get_mobile_device_info",
    "get_mobile_metrics",
    "has_notification_permission",
    // engine fork RenderingServer (decentraland/godotengine#27), probed in engine/engine_fork.rs
    "instance_geometry_is_draw_ready",
    "load_products",
    "open_auth_url",
    "open_safari_auth_url",
    "open_webview_url",
    "ping",
    "purchase",
    "resolve_environment",
    "share_text",
    "share_text_with_image",
    "start_listening",
    "test_logging",
    "version",
];

/// Signals of the native plugins, connected by name.
const EXTERNAL_SIGNALS: &[&str] = &[
    "apns_token_ready",
    "deeplink_received",
    "fcm_token_ready",
    "fcm_token_refreshed",
    "firebase_app_instance_id_ready",
    "memory_trim",
];

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// `#[func]` and `#[signal]` functions by name; `true` when callable on an instance.
fn declared(text: &str, found: &mut HashMap<String, bool>) {
    let attr = Regex::new(
        r"#\[(?:func|signal)[^\]]*\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([a-z]+\))?\s+)?(?:async\s+)?fn\s+(\w+)\s*\(([^)]*)\)",
    )
    .unwrap();
    for caps in attr.captures_iter(text) {
        let on_instance = caps[2].trim_start().starts_with('&') || caps[2].contains("self");
        let entry = found.entry(caps[1].to_string()).or_insert(false);
        *entry |= on_instance;
    }
}

#[test]
fn call_targets_exist_on_instances() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root, &mut files);

    let mut funcs: HashMap<String, bool> = HashMap::new();
    let texts: Vec<(PathBuf, String)> = files
        .into_iter()
        .filter(|p| !p.ends_with("call_targets_test.rs"))
        .filter_map(|p| fs::read_to_string(&p).ok().map(|t| (p, t)))
        .collect();
    for (_, text) in &texts {
        declared(text, &mut funcs);
    }

    let call = Regex::new(r#"(?:call_deferred|\.call|callv|callable)\(\s*"(\w+)""#).unwrap();
    let mut problems = BTreeSet::new();
    for (path, text) in &texts {
        for caps in call.captures_iter(text) {
            let name = &caps[1];
            if EXTERNAL.contains(&name) {
                continue;
            }
            match funcs.get(name) {
                Some(true) => {}
                Some(false) => {
                    problems.insert(format!(
                        "{name}: a static #[func] cannot be called on an instance ({})",
                        path.display()
                    ));
                }
                None => {
                    problems.insert(format!(
                        "{name}: no #[func] or #[signal] of that name ({})",
                        path.display()
                    ));
                }
            }
        }
    }
    // Signals are typed (`signals().name()`): a string-named emit never, a string-named connect
    // only to a plugin's signal.
    let emit = Regex::new(r#"emit_signal\(\s*""#).unwrap();
    let connect = Regex::new(r#"\.(?:connect|connect_flags|has_signal)\(\s*"(\w+)""#).unwrap();
    for (path, text) in &texts {
        if emit.is_match(text) {
            problems.insert(format!("emit_signal by name ({})", path.display()));
        }
        for caps in connect.captures_iter(text) {
            let name = &caps[1];
            if !EXTERNAL_SIGNALS.contains(&name) {
                problems.insert(format!(
                    "{name}: a signal connected by name ({})",
                    path.display()
                ));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "call targets that do not resolve:\n{}",
        problems.into_iter().collect::<Vec<_>>().join("\n")
    );
}
