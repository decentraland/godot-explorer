//! Bridges the Scene Inspector entries of the Rust dispatcher to a dedicated WebSocket target, and
//! serves the commands of external tools over it (the `SCENE_INSPECTOR_CMD` protocol): the single
//! transport of the live client. Inspection verbs go through `commands::run`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use godot::classes::{INode, Node, Os};
use godot::prelude::*;
use serde_json::{json, Map, Value};

use super::commands::{self, FocusTracker};
use super::socket::{Event, Socket};
use crate::fetcher::api::DclNavigator;
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;

/// Debug builds with no explicit target dial a local hub over loopback, so a plain editor deploy
/// auto-dials with no `--scene-inspector` argument. Android reaches it through
/// `adb reverse tcp:9231 tcp:9231`; the desktop hits it directly.
const DEFAULT_DEBUG_TARGET: &str = "ws://127.0.0.1:9231";

/// Commands served on production builds: inspection and stream settings only. Anything that
/// changes client state or runs code is refused there.
const PRODUCTION_COMMANDS: [&str; 17] = [
    "get_status",
    "set_perf_interval",
    "set_lifecycle_verbose",
    "set_include_bin_payload",
    "node_query",
    "subscribe",
    "unsubscribe",
    "ping",
    "focus",
    "scenes",
    "scene",
    "entity",
    "ui_scene",
    "ui_entity",
    "avatars",
    "avatar",
    "app_ui",
];

/// The state of the bridge. Shared, so the socket's events and the dispatcher's batches are
/// handled without borrowing the node that polls them.
#[derive(Default)]
struct Core {
    socket: Socket,
    focus: FocusTracker,
    session_id: RefCell<String>,
    target: RefCell<String>,
    socket_started: Cell<bool>,
}

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct SceneInspectorBridge {
    base: Base<Node>,
    core: Rc<Core>,
}

#[godot_api]
impl INode for SceneInspectorBridge {
    fn process(&mut self, delta: f64) {
        let core = self.core.clone();
        if Os::singleton().is_debug_build() && !DclGlobal::is_production() {
            core.focus.poll();
        }
        if core.socket_started.get() {
            core.socket.poll(delta, &|event| core.on_event(event));
        }
    }
}

impl SceneInspectorBridge {
    /// Called once by the boot, before the bridge enters the tree.
    ///
    /// `--scene-inspector=ws://…` (baked into iOS dev builds, passed on the desktop) dials at once,
    /// and debug builds fall back to the default target. In debug builds this also arms the
    /// bounded boot-log ring, so the startup logs are buffered and flushed on the first
    /// `subscribe`. A `?scene-inspector=` deeplink target is dialed only on non-production builds
    /// and only on the local network: a deeplink is a tappable URL, not device access, so
    /// production builds take a target from the CLI flag or the baked key alone.
    pub fn activate_from_config(&mut self) {
        let mut target = globals::cli().bind().scene_inspector.to_string();
        if target.is_empty() && Os::singleton().is_debug_build() && !DclGlobal::is_production() {
            target = DEFAULT_DEBUG_TARGET.to_string();
        }
        if !target.is_empty() {
            self.core.start(&target, &self.to_gd());
        }
        let me = self.to_gd();
        let router = DclGlobal::singleton().bind().deep_link_router.clone();
        let core = self.core.clone();
        let handler_me = me.clone();
        router
            .signals()
            .deep_link_received()
            .connect_other(&me, move |_: &mut Self| core.on_deep_link(&handler_me));
        let core = self.core.clone();
        crate::engine::local_task::defer(move || core.on_deep_link(&me));
    }
}

impl Core {
    /// Starts dialing `target`; a second target (a deeplink after the config) redials.
    fn start(self: &Rc<Self>, target: &str, bridge: &Gd<SceneInspectorBridge>) {
        if *self.target.borrow() == target {
            return;
        }
        let first = self.target.borrow().is_empty();
        *self.target.borrow_mut() = target.to_string();
        if !first {
            self.connect_socket(target);
            return;
        }
        DclGlobal::singleton().bind_mut().scene_inspector_active = true;
        let dispatcher = globals::scene_inspector_dispatcher();
        if Os::singleton().is_debug_build() {
            dispatcher.clone().bind_mut().set_early_log_capture(true);
        }
        godot_print!("SceneInspectorBridge: activating -> {target}");
        let core = self.clone();
        let target = target.to_string();
        let bridge = bridge.clone();
        crate::engine::local_task::defer(move || core.setup(&target, &bridge));
    }

    fn setup(self: &Rc<Self>, target: &str, bridge: &Gd<SceneInspectorBridge>) {
        let dispatcher = globals::scene_inspector_dispatcher();
        *self.session_id.borrow_mut() = dispatcher.bind().get_session_id().to_string();
        self.connect_socket(target);
        let core = self.clone();
        dispatcher.signals().scene_inspector_batch().connect_other(
            bridge,
            move |_: &mut SceneInspectorBridge, entries: GString| {
                core.on_batch(&entries.to_string());
            },
        );
    }

    fn connect_socket(&self, target: &str) {
        if !(target.starts_with("ws://") || target.starts_with("wss://")) {
            godot_error!(
                "SceneInspectorBridge: unsupported target {target} (expected ws:// or wss://)"
            );
            return;
        }
        // The session start and the hot-connect CRDT snapshot are sent once the socket is open:
        // sending them before the handshake completes would drop them.
        self.socket.connect_to(target);
        self.socket_started.set(true);
        godot_print!("SceneInspectorBridge: Dedicated WebSocket channel -> {target}");
    }

    fn on_deep_link(self: &Rc<Self>, bridge: &Gd<SceneInspectorBridge>) {
        let link = DclGlobal::singleton().bind().deep_link_obj.clone();
        let target = link.bind().scene_inspector.to_string();
        if target.is_empty() || *self.target.borrow() == target {
            return;
        }
        if DclGlobal::is_production() {
            godot_print!("SceneInspectorBridge: ignoring deeplink target on production builds");
            return;
        }
        if !dcl_core::deep_link_route::is_local_network(&target) {
            godot_print!("SceneInspectorBridge: ignoring deeplink target {target}");
            return;
        }
        self.start(&target, bridge);
    }

    fn on_event(&self, event: Event) {
        let dispatcher = globals::scene_inspector_dispatcher();
        match event {
            Event::Connected => {
                // A consumer attached: open the capture gate. The classic streams flow by default;
                // logs and network stay opt-in through `subscribe`.
                let mut dispatcher = dispatcher;
                {
                    let mut dispatcher = dispatcher.bind_mut();
                    dispatcher.set_consumer_connected(true);
                    dispatcher.emit_session_start();
                }
                self.send_crdt_snapshot();
            }
            // The consumer is gone: nothing keeps buffering without a peer.
            Event::Disconnected => dispatcher.clone().bind_mut().set_consumer_connected(false),
            Event::Command(command) => {
                self.on_command(&command.cmd, &command.args, &command.request_id)
            }
        }
    }

    /// `entries_json` is already valid JSON from Rust: splicing it into the envelope avoids a
    /// parse and a stringify of up to 500 entries every frame.
    fn send_entries(&self, entries_json: &str) {
        let envelope = format!(
            r#"{{"type":"SCENE_INSPECTOR","payload":{{"sessionId":{},"entries":{entries_json}}}}}"#,
            json!(*self.session_id.borrow())
        );
        self.socket.send_text(&envelope);
    }

    fn on_batch(&self, entries_json: &str) {
        if self.socket.is_open() {
            self.send_entries(entries_json);
        }
    }

    fn send_crdt_snapshot(&self) {
        let snapshot = globals::scene_inspector_dispatcher()
            .bind()
            .get_crdt_snapshot_json()
            .to_string();
        if !snapshot.is_empty() && self.socket.is_open() {
            self.send_entries(&snapshot);
        }
    }

    fn send_ack(&self, request_id: &str, ok: bool, data: Value) {
        if request_id.is_empty() || !self.socket.is_open() {
            return;
        }
        let ack =
            json!({"type": "SCENE_INSPECTOR_CMD_ACK", "id": request_id, "ok": ok, "data": data});
        self.socket.send_text(&ack.to_string());
    }

    fn on_command(&self, command: &str, args: &Map<String, Value>, request_id: &str) {
        if DclGlobal::is_production() && !PRODUCTION_COMMANDS.contains(&command) {
            let error = format!("{command} is not available on production builds");
            self.send_ack(request_id, false, json!({"error": error}));
            return;
        }
        let result = self.run(command, args);
        match result {
            Ok(data) => self.send_ack(request_id, true, data),
            Err(error) => self.send_ack(request_id, false, json!({"error": error})),
        }
    }

    fn run(&self, command: &str, args: &Map<String, Value>) -> Result<Value, String> {
        let dispatcher = globals::scene_inspector_dispatcher();
        let flag =
            |key: &str, fallback: bool| args.get(key).and_then(Value::as_bool).unwrap_or(fallback);
        match command {
            "pause" | "resume" => {
                let paused = command == "pause";
                // Pausing stops the scenes' JS; the inspector stream continues.
                set_all_scenes_paused(paused);
                dispatcher.clone().bind_mut().set_paused(paused);
                Ok(json!({}))
            }
            "reload_scene" => {
                DclNavigator::reload_current("on_reload".into());
                Ok(json!({}))
            }
            "get_status" => {
                let d = dispatcher.bind();
                Ok(json!({
                    "paused": d.is_paused(),
                    "session_id": *self.session_id.borrow(),
                    "scene_inspector_active": DclGlobal::singleton().bind().scene_inspector_active,
                    "file_logging": d.is_file_logging(),
                    "entry_count": d.get_entry_count(),
                    "perf_interval": d.get_perf_interval(),
                }))
            }
            "set_file_logging" => {
                dispatcher
                    .clone()
                    .bind_mut()
                    .set_file_logging(flag("enabled", false));
                Ok(json!({}))
            }
            "set_perf_interval" => {
                let seconds = args.get("seconds").and_then(Value::as_f64).unwrap_or(2.0);
                dispatcher.clone().bind_mut().set_perf_interval(seconds);
                Ok(json!({}))
            }
            // Suppresses the per-tick lifecycle entries while keeping CRDT, op and log entries.
            "set_lifecycle_verbose" => {
                dispatcher
                    .clone()
                    .bind_mut()
                    .set_lifecycle_verbose(flag("enabled", true));
                Ok(json!({}))
            }
            // Opt-in to always attaching the raw hex payload: redundant when the JSON decodes, and
            // it doubles the size of every entry.
            "set_include_bin_payload" => {
                dispatcher
                    .clone()
                    .bind_mut()
                    .set_include_bin_payload(flag("enabled", false));
                Ok(json!({}))
            }
            "node_query" => Ok(node_query(args)),
            "subscribe" | "unsubscribe" => {
                apply_subscribe(args, command == "subscribe");
                Ok(json!({"streams": args.get("streams").cloned().unwrap_or(json!([]))}))
            }
            _ => commands::run(command, args, &self.focus),
        }
    }
}

/// Opts in to (or out of) the high-volume streams and the per-tick lifecycle firehose. The CRDT,
/// op and perf streams flow whenever a consumer is connected, so they are not toggled here.
fn apply_subscribe(args: &Map<String, Value>, enable: bool) {
    let mut dispatcher = globals::scene_inspector_dispatcher();
    let mut dispatcher = dispatcher.bind_mut();
    for stream in args
        .get("streams")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        match stream.as_str().unwrap_or_default() {
            "log" | "logs" => dispatcher.set_stream_logs(enable),
            "network" => dispatcher.set_stream_network(enable),
            "lifecycle" => dispatcher.set_lifecycle_verbose(enable),
            _ => {}
        }
    }
}

fn set_all_scenes_paused(paused: bool) {
    let mut runner = globals::scene_runner();
    for child in runner.get_children().iter_shared() {
        if let Ok(scene) = child.try_cast::<crate::godot_classes::dcl_scene_node::DclSceneNode>() {
            let id = scene.bind().get_scene_id();
            runner.bind_mut().set_scene_is_paused(id, paused);
        }
    }
}

/// A generic live node probe: nodes found by a substring of their name, with the properties asked
/// for (`props`) and, off production, no-argument or any-argument method calls (`calls`).
fn node_query(args: &Map<String, Value>) -> Value {
    let query = args
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();
    let props: Vec<&str> = args
        .get("props")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let calls: Vec<&Vec<Value>> = if DclGlobal::is_production() {
        Vec::new()
    } else {
        args.get("calls")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_array).collect())
            .unwrap_or_default()
    };
    let max = args.get("max").and_then(Value::as_i64).unwrap_or(5).max(0) as usize;
    let Some(root) = DclGlobal::singleton()
        .get_tree_or_null()
        .and_then(|t| t.get_root())
    else {
        return json!({"count": 0, "nodes": []});
    };
    let found: Vec<Gd<Node>> = root
        .find_children_ex("*")
        .recursive(true)
        .owned(false)
        .done()
        .iter_shared()
        .filter(|n| n.get_name().to_string().to_lowercase().contains(&query))
        .collect();
    let nodes: Vec<Value> = found
        .iter()
        .take(max)
        .map(|node| {
            let mut entry = json!({
                "name": node.get_name().to_string(),
                "path": node.get_path().to_string(),
                "class": node.get_class().to_string(),
            });
            for prop in &props {
                entry[*prop] = json!(godot::global::var_to_str(
                    &node.clone().upcast::<Object>().get(*prop)
                )
                .to_string());
            }
            for call in &calls {
                let Some(method) = call.first().and_then(Value::as_str) else {
                    continue;
                };
                let mut object = node.clone().upcast::<Object>();
                if object.has_method(method) {
                    let arguments: Vec<Variant> = call[1..].iter().map(json_to_variant).collect();
                    let result =
                        object.callv(method, &arguments.iter().cloned().collect::<VarArray>());
                    entry[format!("call:{method}")] =
                        json!(godot::global::var_to_str(&result).to_string());
                }
            }
            entry
        })
        .collect();
    json!({"count": found.len(), "nodes": nodes})
}

fn json_to_variant(value: &Value) -> Variant {
    match value {
        Value::Null => Variant::nil(),
        Value::Bool(b) => b.to_variant(),
        Value::Number(n) => match n.as_i64() {
            Some(i) => i.to_variant(),
            None => n.as_f64().unwrap_or_default().to_variant(),
        },
        Value::String(s) => s.as_str().to_variant(),
        other => other.to_string().as_str().to_variant(),
    }
}
