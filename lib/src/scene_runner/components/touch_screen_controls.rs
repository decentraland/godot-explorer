use godot::prelude::*;

use crate::{
    dcl::{
        components::{
            proto_components::{common::texture_union::Tex, sdk::components::common::InputAction},
            SceneComponentId, SceneEntityId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        SceneId,
    },
    godot_classes::dcl_global::DclGlobal,
    scene_runner::{
        input::input_action_to_godot_action,
        scene::{Scene, SceneType},
    },
};

/// Applies the `PBTouchScreenControls` component (set on the RootEntity of the current
/// parcel scene) to the native on-screen controls via `DclGlobal`:
/// - `hide_joystick` hides the native virtual joystick
/// - `touch_inputs` lists per-button overrides (hide / custom icon) — buttons not listed
///   keep their default: shown for most, but the joypad hides the two overflow actions
///   ia_action_5 / ia_action_6 ("3" / "4") by default so the "+" combo shows 2, not 4; a
///   scene re-shows either by listing it with hide=false (see joypad.gd DEFAULT_HIDDEN)
/// - `main_action` chooses which action the large central button triggers
///
/// When the scene sets no component the controls are reset to their defaults (the joypad's
/// default-hidden overflow applied, jump as the central button), so legacy scenes are unaffected.
pub fn update_touch_screen_controls(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    current_parcel_scene_id: &SceneId,
) {
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let is_current_parcel_scene = scene.scene_id == *current_parcel_scene_id;

    // Mirror InputModifier processing rules: current parcel scene every tick (so entering
    // a scene applies it); global scenes only when dirty on ROOT.
    let should_process = match scene.scene_type {
        SceneType::Parcel => is_current_parcel_scene,
        SceneType::Global(_) => dirty_lww_components
            .get(&SceneComponentId::TOUCH_SCREEN_CONTROLS)
            .is_some_and(|dirty| dirty.contains(&SceneEntityId::ROOT)),
    };
    if !should_process {
        return;
    }

    let component = SceneCrdtStateProtoComponents::get_touch_screen_controls(crdt_state);
    let value = component.get(&SceneEntityId::ROOT);

    let Some(mut global) = DclGlobal::try_singleton() else {
        return;
    };
    let mut global_bind = global.bind_mut();

    match value.and_then(|entry| entry.value.as_ref()) {
        Some(controls) => {
            let mut inputs = VarArray::new();
            for input in controls.touch_inputs.iter() {
                let Some(action) = InputAction::from_i32(input.input_action) else {
                    continue;
                };
                let Some(action_name) = input_action_to_godot_action(action) else {
                    continue;
                };
                let mut dict = VarDictionary::new();
                dict.set("action", GString::from(action_name));
                dict.set("hide", input.hide);
                // Resolve the scene-relative icon path to a content hash + URL so the
                // (scene-less) global joypad can fetch it. Unresolved -> empty (built-in glyph).
                // Only the `Texture` variant of TextureUnion is supported for button icons;
                // avatar / video textures fall back to the built-in glyph (with a warning).
                let icon_path = match input.icon.as_ref().and_then(|tex| tex.tex.as_ref()) {
                    Some(Tex::Texture(texture)) => texture.src.as_str(),
                    Some(Tex::AvatarTexture(_)) | Some(Tex::VideoTexture(_)) => {
                        godot_warn!(
                            "PBTouchScreenControls: avatar/video textures are not supported for button icons; using the built-in glyph"
                        );
                        ""
                    }
                    None => "",
                };
                let (icon_hash, icon_url) = match scene.content_mapping.get_hash(icon_path) {
                    Some(hash) => (
                        hash.clone(),
                        format!("{}{}", scene.content_mapping.base_url, hash),
                    ),
                    None => (String::new(), String::new()),
                };
                dict.set("icon_hash", GString::from(icon_hash.as_str()));
                dict.set("icon_url", GString::from(icon_url.as_str()));
                inputs.push(&dict.to_variant());
            }

            let main_action = controls
                .main_action
                .and_then(InputAction::from_i32)
                .and_then(input_action_to_godot_action)
                .map(GString::from)
                .unwrap_or_default();

            global_bind.touch_controls_active = true;
            global_bind.touch_controls_hide_joystick = controls.hide_joystick;
            global_bind.touch_controls_hide_crosshair = controls.hide_crosshair;
            global_bind.touch_controls_main_action = main_action;
            global_bind.touch_controls_inputs = inputs;
        }
        None => {
            // Component removed or not set: reset to defaults (only for current parcel scene)
            if is_current_parcel_scene {
                global_bind.reset_touch_controls();
            }
        }
    }
}
