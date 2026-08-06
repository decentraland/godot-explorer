//! Guards issue #2578: HUD overlays layered above the SDK scene's UI
//! (`SceneUIContainer` in `explorer.tscn`) must be click-through wherever they
//! don't draw interactive content.
//!
//! In Godot only `MOUSE_FILTER_IGNORE` (2) lets an event reach a *lower
//! sibling*: `PASS` (1) — the default for every `*Container` — still claims the
//! hit and only bubbles to ancestors. Because the whole HUD and the scene UI
//! live in one Control tree ordered by sibling index, any container above
//! `SceneUIContainer` without an explicit `mouse_filter = 2` is an invisible
//! input wall over the scene, even when it draws nothing (the bug: the debug
//! panel's collapsed body still blocked a 500x280 region of scene UI).
//!
//! These checks parse the `.tscn` files as text so they run in plain
//! `cargo test`, with no Godot runtime.

use std::collections::HashMap;

struct TscnNode {
    name: String,
    parent: Option<String>,
    props: HashMap<String, String>,
}

fn parse_tscn(relative_path: &str) -> Vec<TscnNode> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

    let mut nodes: Vec<TscnNode> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("[node ") {
            nodes.push(TscnNode {
                name: extract_attribute(line, "name").expect("[node] without name"),
                parent: extract_attribute(line, "parent"),
                props: HashMap::new(),
            });
        } else if line.starts_with('[') {
            // A [sub_resource], [connection], etc. section ends the current node block.
            nodes.push(TscnNode {
                name: String::new(),
                parent: None,
                props: HashMap::new(),
            });
        } else if let Some((key, value)) = line.split_once(" = ") {
            if let Some(node) = nodes.last_mut() {
                node.props.insert(key.to_string(), value.to_string());
            }
        }
    }
    nodes.retain(|n| !n.name.is_empty());
    nodes
}

fn extract_attribute(header: &str, attribute: &str) -> Option<String> {
    let marker = format!("{attribute}=\"");
    let start = header.find(&marker)? + marker.len();
    let end = header[start..].find('"')? + start;
    Some(header[start..end].to_string())
}

fn find<'a>(nodes: &'a [TscnNode], name: &str, parent: &str) -> &'a TscnNode {
    nodes
        .iter()
        .find(|n| n.name == name && n.parent.as_deref() == Some(parent))
        .unwrap_or_else(|| panic!("node {name} with parent {parent} not found — was it renamed?"))
}

fn assert_click_through(node: &TscnNode, scene: &str) {
    assert_eq!(
        node.props.get("mouse_filter").map(String::as_str),
        Some("2"),
        "{scene}: node '{}' (parent '{}') must declare mouse_filter = 2 (IGNORE). \
         Containers default to PASS, which still swallows clicks meant for the \
         SDK scene UI below it (issue #2578).",
        node.name,
        node.parent.as_deref().unwrap_or(""),
    );
}

/// The debug panel body is a right-aligned VBox whose `Control_Panel` child
/// reserves 500x280 even while the console is collapsed (`Button_ShowHide`
/// only toggles the inner `TabContainer`). The VBox and the top-buttons row
/// must be IGNORE so only the actual buttons/console claim input.
#[test]
fn debug_panel_body_does_not_block_scene_ui() {
    let nodes = parse_tscn("../godot/src/ui/components/organisms/debug_panel/debug_panel.tscn");
    assert_click_through(
        find(&nodes, "VBoxContainer", "MarginContainer"),
        "debug_panel",
    );
    assert_click_through(
        find(
            &nodes,
            "HBoxContainer_TopButtons",
            "MarginContainer/VBoxContainer",
        ),
        "debug_panel",
    );
}

/// The navbar's top-right corner Control hosts an 80x80 profile bubble but
/// measures 120x136; both it and its full-rect toggle Button stole roughly
/// 2.5x the drawn area from the scene UI underneath.
#[test]
fn navbar_profile_hitbox_matches_visible_bubble() {
    let nodes = parse_tscn("../godot/src/ui/components/organisms/navbar/navbar.tscn");
    assert_click_through(find(&nodes, "Control", "."), "navbar");

    let button = find(&nodes, "Button", "Control");
    assert_eq!(
        button.props.get("custom_minimum_size").map(String::as_str),
        Some("Vector2(80, 80)"),
        "navbar: the profile toggle Button hitbox must match the visible 80x80 \
         Panel_Profile bubble, not the whole 120x136 corner Control (issue #2578).",
    );
    assert_ne!(
        button.props.get("anchors_preset").map(String::as_str),
        Some("15"),
        "navbar: the profile toggle Button must not be full-rect over the \
         corner Control (issue #2578).",
    );
}

/// Later siblings draw and hit-test on top. The debug tools row (console +
/// scene stats) must beat scene UI for input, but render *below* the
/// Friends/Notifications/Settings panels and the navbar — so it has to sit
/// after `SceneUIContainer` and before `SafeAreaHud`.
#[test]
fn debug_tools_sit_between_scene_ui_and_right_panels() {
    let nodes = parse_tscn("../godot/src/ui/explorer.tscn");
    let ui_children: Vec<&str> = nodes
        .iter()
        .filter(|n| n.parent.as_deref() == Some("UI"))
        .map(|n| n.name.as_str())
        .collect();
    let idx = |name: &str| {
        ui_children
            .iter()
            .position(|n| *n == name)
            .unwrap_or_else(|| panic!("{name} not found under UI — was it renamed?"))
    };
    let scene_ui = idx("SceneUIContainer");
    let debug_tools = idx("SafeMarginContainerDebug");
    let hud_panels = idx("SafeAreaHud");
    assert!(
        scene_ui < debug_tools && debug_tools < hud_panels,
        "explorer: SafeMarginContainerDebug (debug console + scene stats) must come \
         after SceneUIContainer but before SafeAreaHud, so the debug \
         tools receive input over scene UI yet render below the Friends/\
         Notifications/Settings panels (issue #2578).",
    );
}

/// The bottom-center version/FPS strip is an HBoxContainer of Labels; the
/// Labels ignore input but the HBox itself defaulted to PASS.
#[test]
fn version_fps_strip_does_not_block_scene_ui() {
    let nodes = parse_tscn("../godot/src/ui/explorer.tscn");
    assert_click_through(
        find(
            &nodes,
            "HBoxContainer_VersionFPS",
            "UI/SafeAreaHud/InteractableHUD",
        ),
        "explorer",
    );
}
