use crate::{
    dcl::{
        components::{
            proto_components::{common::Color3, sdk::components::PbAvatarNametag},
            SceneComponentId, SceneEntityId,
        },
        crdt::{
            last_write_wins::{LastWriteWins, LastWriteWinsComponentOperation},
            SceneCrdtState, SceneCrdtStateProtoComponents,
        },
        SceneId,
    },
    scene_runner::scene::{Scene, SceneType},
};
use std::time::{Duration, Instant};

use godot::{
    classes::{Node, Node3D},
    prelude::*,
};

/// Text color of this client's native nametag (`nickname_ui.tscn`), used when the
/// scene omits `label_color`.
pub fn native_label_color() -> Color {
    Color::from_rgb(1.0, 1.0, 1.0)
}

/// Background of this client's native nametag (`nickname_ui.tscn` panel).
pub fn native_background_color() -> Color {
    Color::from_rgb(0.129412, 0.129412, 0.129412)
}

/// How long a write to an entity that cannot back an avatar yet is retried: the
/// player avatar appears after connect and an `AvatarShape` after its load, so a
/// write that lands first must not be lost.
const PENDING_RETRY_WINDOW: Duration = Duration::from_secs(5);

/// One plate as the renderer draws it, with the protocol's fallbacks resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct NametagPlate {
    pub label: String,
    pub label_color: Color,
    pub background_color: Color,
    pub border_color: Color,
}

impl NametagPlate {
    pub fn from_proto(value: &PbAvatarNametag) -> Self {
        let background_color = color3_or_background(value.background_color.as_ref());
        Self {
            // Kept verbatim: an empty label draws a color-only plate, and a label of
            // spaces widens it (protocol#474).
            label: value.label.clone(),
            label_color: color3_or(value.label_color.as_ref(), native_label_color()),
            background_color,
            // Border defaults to the background so a plate carries no visible border.
            border_color: color3_or(value.border_color.as_ref(), background_color),
        }
    }
}

fn color3_or(color: Option<&Color3>, fallback: Color) -> Color {
    match color {
        Some(color) => Color::from_rgb(color.r, color.g, color.b),
        None => fallback,
    }
}

fn color3_or_background(color: Option<&Color3>) -> Color {
    color3_or(color, native_background_color())
}

/// `AvatarNametag` is honoured on the local player and on any entity the scene
/// backs with an `AvatarShape` (NPC); on every other entity it is a no-op.
pub fn is_nametag_target(entity: SceneEntityId, has_avatar_shape: bool) -> bool {
    entity == SceneEntityId::PLAYER || has_avatar_shape
}

/// Avatar node a plate is drawn on, if this entity can back one right now.
///
/// The caller passes the player avatar because binding the SceneManager from inside its
/// own update panics (`already bound`), and that panic kills the scene.
fn resolve_target_node(
    scene: &Scene,
    entity: &SceneEntityId,
    player_avatar_node: Option<&Gd<Node3D>>,
) -> Option<Gd<Node>> {
    let avatar_shape = scene
        .godot_dcl_scene
        .get_node_or_null_3d(entity)
        .and_then(|node_3d| node_3d.clone().try_get_node_as::<Node>("AvatarShape"));

    if !is_nametag_target(*entity, avatar_shape.is_some()) {
        return None;
    }

    if *entity == SceneEntityId::PLAYER {
        player_avatar_node.map(|node| node.clone().upcast::<Node>())
    } else {
        avatar_shape
    }
}

fn current_component_value(
    component: &LastWriteWins<PbAvatarNametag>,
    entity: &SceneEntityId,
) -> Option<PbAvatarNametag> {
    component.get(entity).and_then(|value| value.value.clone())
}

/// Draw a scene-authored plate above the native nametag (#2811, protocol#474).
///
/// Plates are local-only, so only the scene the player currently stands in may draw
/// one: leaving it drops the plate, and no other scene can repaint the shared player
/// avatar.
pub fn update_avatar_nametag(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    current_parcel_scene_id: &SceneId,
    player_avatar_node: Option<&Gd<Node3D>>,
) {
    let user_in_scene = match scene.scene_type {
        SceneType::Parcel => scene.scene_id == *current_parcel_scene_id,
        _ => false,
    };

    if !user_in_scene {
        // Writes made while the player is elsewhere never reach the shared avatar: the
        // whole component map is rescanned on re-entry, so the dirty list is dropped
        // instead of piling up (`current_dirty` is replaced every frame).
        scene
            .current_dirty
            .lww_components
            .remove(&SceneComponentId::AVATAR_NAMETAG);
        scene.avatar_nametags_synced = false;
        clear_plates(scene, player_avatar_node);
        return;
    }

    let nametag_component = SceneCrdtStateProtoComponents::get_avatar_nametag(crdt_state);

    if !scene.avatar_nametags_synced {
        // Became current: every plate this scene holds has to be drawn again, exactly
        // like the writes that landed while it was not current were never applied.
        scene.avatar_nametags_synced = true;
        let entities: Vec<SceneEntityId> = nametag_component.values.keys().copied().collect();
        for entity in entities {
            apply_entity(scene, nametag_component, &entity, player_avatar_node);
        }
    }

    let dirty = scene
        .current_dirty
        .lww_components
        .remove(&SceneComponentId::AVATAR_NAMETAG);

    if let Some(dirty) = dirty {
        for entity in dirty.iter() {
            apply_entity(scene, nametag_component, entity, player_avatar_node);
        }
    }

    retry_pending(scene, nametag_component, player_avatar_node);
}

fn apply_entity(
    scene: &mut Scene,
    component: &LastWriteWins<PbAvatarNametag>,
    entity: &SceneEntityId,
    player_avatar_node: Option<&Gd<Node3D>>,
) {
    let Some(value) = current_component_value(component, entity) else {
        clear_plate(scene, entity, player_avatar_node);
        return;
    };

    match resolve_target_node(scene, entity, player_avatar_node) {
        Some(mut node) => {
            apply_plate(&mut node, &NametagPlate::from_proto(&value));
            scene.avatar_nametags.insert(*entity);
            scene.pending_avatar_nametags.remove(entity);
        }
        None => {
            scene
                .pending_avatar_nametags
                .entry(*entity)
                .or_insert_with(|| Instant::now() + PENDING_RETRY_WINDOW);
        }
    }
}

fn apply_plate(node: &mut Gd<Node>, plate: &NametagPlate) {
    node.call(
        "set_scene_nametag",
        &[
            plate.label.to_variant(),
            plate.label_color.to_variant(),
            plate.background_color.to_variant(),
            plate.border_color.to_variant(),
        ],
    );
}

fn clear_plate(scene: &mut Scene, entity: &SceneEntityId, player_avatar_node: Option<&Gd<Node3D>>) {
    scene.pending_avatar_nametags.remove(entity);
    if !scene.avatar_nametags.remove(entity) {
        return;
    }
    if let Some(mut node) = resolve_target_node(scene, entity, player_avatar_node) {
        node.call("clear_scene_nametag", &[]);
    }
}

fn clear_plates(scene: &mut Scene, player_avatar_node: Option<&Gd<Node3D>>) {
    scene.pending_avatar_nametags.clear();
    let applied = std::mem::take(&mut scene.avatar_nametags);
    for entity in applied.iter() {
        if let Some(mut node) = resolve_target_node(scene, entity, player_avatar_node) {
            node.call("clear_scene_nametag", &[]);
        }
    }
}

/// Plates whose target avatar did not exist when the scene wrote the component.
fn retry_pending(
    scene: &mut Scene,
    component: &LastWriteWins<PbAvatarNametag>,
    player_avatar_node: Option<&Gd<Node3D>>,
) {
    if scene.pending_avatar_nametags.is_empty() {
        return;
    }

    let now = Instant::now();
    let pending = std::mem::take(&mut scene.pending_avatar_nametags);
    for (entity, deadline) in pending {
        if now >= deadline {
            continue;
        }

        let Some(value) = current_component_value(component, &entity) else {
            continue;
        };

        match resolve_target_node(scene, &entity, player_avatar_node) {
            Some(mut node) => {
                apply_plate(&mut node, &NametagPlate::from_proto(&value));
                scene.avatar_nametags.insert(entity);
            }
            None => {
                scene.pending_avatar_nametags.insert(entity, deadline);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color3(r: f32, g: f32, b: f32) -> Option<Color3> {
        Some(Color3 { r, g, b })
    }

    #[test]
    fn falls_back_to_native_colors_when_every_field_is_absent() {
        let plate = NametagPlate::from_proto(&PbAvatarNametag {
            label: String::from("Club Owner"),
            ..Default::default()
        });

        assert_eq!(plate.label_color, native_label_color());
        assert_eq!(plate.background_color, native_background_color());
        assert_eq!(plate.border_color, native_background_color());
    }

    #[test]
    fn border_falls_back_to_the_authored_background() {
        let plate = NametagPlate::from_proto(&PbAvatarNametag {
            background_color: color3(0.2, 0.4, 0.6),
            ..Default::default()
        });

        assert_eq!(plate.background_color, Color::from_rgb(0.2, 0.4, 0.6));
        assert_eq!(plate.border_color, Color::from_rgb(0.2, 0.4, 0.6));
        assert_eq!(plate.label_color, native_label_color());
    }

    #[test]
    fn keeps_every_authored_color() {
        let plate = NametagPlate::from_proto(&PbAvatarNametag {
            label: String::from("VIP"),
            label_color: color3(1.0, 0.8, 0.0),
            background_color: color3(0.1, 0.1, 0.1),
            border_color: color3(1.0, 0.8, 0.0),
        });

        assert_eq!(plate.label_color, Color::from_rgb(1.0, 0.8, 0.0));
        assert_eq!(plate.background_color, Color::from_rgb(0.1, 0.1, 0.1));
        assert_eq!(plate.border_color, Color::from_rgb(1.0, 0.8, 0.0));
    }

    #[test]
    fn empty_and_space_labels_are_kept_verbatim() {
        let empty = NametagPlate::from_proto(&PbAvatarNametag::default());
        assert_eq!(empty.label, "");

        let spaces = NametagPlate::from_proto(&PbAvatarNametag {
            label: String::from("   "),
            ..Default::default()
        });
        assert_eq!(spaces.label, "   ");
    }

    #[test]
    fn only_player_and_avatar_shape_entities_are_targets() {
        assert!(is_nametag_target(SceneEntityId::PLAYER, false));
        assert!(is_nametag_target(SceneEntityId::new(42, 0), true));
        assert!(!is_nametag_target(SceneEntityId::new(42, 0), false));
        assert!(!is_nametag_target(SceneEntityId::CAMERA, false));
    }
}
