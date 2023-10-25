use std::collections::HashMap;

use godot::{
    engine::{
        global::{HorizontalAlignment, VerticalAlignment},
        Label,
    },
    prelude::Gd,
};

use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::{
                common::TextAlignMode, PbUiTransform, YgAlign, YgDisplay, YgFlexDirection,
                YgJustify, YgPositionType, YgUnit, YgWrap,
            },
            SceneComponentId, SceneEntityId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_ui_background::DclUiBackground,
    scene_runner::scene::Scene,
};

// macro helpers to convert proto format to bevy format for val, size, rect
macro_rules! val {
    ($pb:ident, $u:ident, $v:ident, $d:expr, $t:ident) => {
        match $pb.$u() {
            YgUnit::YguUndefined => $d,
            YgUnit::YguAuto => taffy::style::$t::Auto,
            YgUnit::YguPoint => taffy::style::$t::Points($pb.$v),
            YgUnit::YguPercent => taffy::style::$t::Percent($pb.$v / 100.0),
        }
    };
}
macro_rules! val_a {
    ($pb:ident, $u:ident, $v:ident, $d:expr, $t:ident) => {
        match $pb.$u() {
            YgUnit::YguAuto | YgUnit::YguUndefined => $d,
            YgUnit::YguPoint => taffy::style::$t::Points($pb.$v),
            YgUnit::YguPercent => taffy::style::$t::Percent($pb.$v / 100.0),
        }
    };
}

macro_rules! size {
    ($pb:ident, $wu:ident, $w:ident, $hu:ident, $h:ident, $d:expr, $t:ident) => {{
        taffy::prelude::Size::<taffy::prelude::$t> {
            width: val!($pb, $wu, $w, $d, $t),
            height: val!($pb, $hu, $h, $d, $t),
        }
    }};
}

macro_rules! rect_a {
    ($pb:ident, $lu:ident, $l:ident, $ru:ident, $r:ident, $tu:ident, $t:ident, $bu:ident, $b:ident, $d:expr, $tt:ident) => {
        taffy::prelude::Rect::<taffy::prelude::$tt> {
            left: val_a!($pb, $lu, $l, $d, $tt),
            right: val_a!($pb, $ru, $r, $d, $tt),
            top: val_a!($pb, $tu, $t, $d, $tt),
            bottom: val_a!($pb, $bu, $b, $d, $tt),
        }
    };
}
macro_rules! rect {
    ($pb:ident, $lu:ident, $l:ident, $ru:ident, $r:ident, $tu:ident, $t:ident, $bu:ident, $b:ident, $d:expr, $tt:ident) => {
        taffy::prelude::Rect::<taffy::prelude::$tt> {
            left: val!($pb, $lu, $l, $d, $tt),
            right: val!($pb, $ru, $r, $d, $tt),
            top: val!($pb, $tu, $t, $d, $tt),
            bottom: val!($pb, $bu, $b, $d, $tt),
        }
    };
}

impl From<&PbUiTransform> for taffy::style::Style {
    fn from(value: &PbUiTransform) -> Self {
        Self {
            // ui_transform.right_of: i32,

            // ui_transform.overflow: i32,
            // overflow: match value.overflow() {
            //     YgOverflow::YgoVisible => Overflow::DEFAULT,
            //     YgOverflow::YgoHidden => Overflow::clip(),
            //     YgOverflow::YgoScroll => {
            //         // TODO: map to scroll area
            //         warn!("ui overflow scroll not implemented");
            //         Overflow::clip()
            //     }
            // },
            display: match value.display() {
                YgDisplay::YgdFlex => taffy::style::Display::Flex,
                YgDisplay::YgdNone => taffy::style::Display::None,
            },
            align_content: match value.align_content() {
                YgAlign::YgaBaseline | YgAlign::YgaAuto => None, // baseline is invalid for align content
                YgAlign::YgaFlexStart => Some(taffy::style::AlignContent::FlexStart),
                YgAlign::YgaCenter => Some(taffy::style::AlignContent::Center),
                YgAlign::YgaFlexEnd => Some(taffy::style::AlignContent::FlexEnd),
                YgAlign::YgaStretch => Some(taffy::style::AlignContent::Stretch),
                YgAlign::YgaSpaceBetween => Some(taffy::style::AlignContent::SpaceBetween),
                YgAlign::YgaSpaceAround => Some(taffy::style::AlignContent::SpaceAround),
            },
            align_items: match value.align_items() {
                YgAlign::YgaAuto |
                YgAlign::YgaSpaceBetween | // invalid
                YgAlign::YgaSpaceAround => None,
                YgAlign::YgaStretch => Some(taffy::style::AlignItems::Stretch),
                YgAlign::YgaFlexStart => Some(taffy::style::AlignItems::FlexStart),
                YgAlign::YgaCenter => Some(taffy::style::AlignItems::Center),
                YgAlign::YgaFlexEnd => Some(taffy::style::AlignItems::FlexEnd),
                YgAlign::YgaBaseline => Some(taffy::style::AlignItems::Baseline),
            },
            flex_grow: value.flex_grow,
            flex_wrap: match value.flex_wrap() {
                YgWrap::YgwNoWrap => taffy::style::FlexWrap::NoWrap,
                YgWrap::YgwWrap => taffy::style::FlexWrap::Wrap,
                YgWrap::YgwWrapReverse => taffy::style::FlexWrap::WrapReverse,
            },
            flex_shrink: value.flex_shrink.unwrap_or(1.0),
            position: match value.position_type() {
                YgPositionType::YgptRelative => taffy::style::Position::Relative,
                YgPositionType::YgptAbsolute => taffy::style::Position::Absolute,
            },
            align_self: match value.align_self() {
                YgAlign::YgaSpaceBetween | // invalid
                YgAlign::YgaSpaceAround | // invalid
                YgAlign::YgaAuto => None,
                YgAlign::YgaFlexStart => Some(taffy::style::AlignSelf::FlexStart),
                YgAlign::YgaCenter => Some(taffy::style::AlignSelf::Center),
                YgAlign::YgaFlexEnd => Some(taffy::style::AlignSelf::FlexEnd),
                YgAlign::YgaStretch => Some(taffy::style::AlignSelf::Stretch),
                YgAlign::YgaBaseline => Some(taffy::style::AlignSelf::Baseline),
            },
            flex_direction: match value.flex_direction() {
                YgFlexDirection::YgfdRow => taffy::style::FlexDirection::Row,
                YgFlexDirection::YgfdColumn => taffy::style::FlexDirection::Column,
                YgFlexDirection::YgfdColumnReverse => taffy::style::FlexDirection::ColumnReverse,
                YgFlexDirection::YgfdRowReverse => taffy::style::FlexDirection::RowReverse,
            },
            justify_content: match value.justify_content() {
                YgJustify::YgjFlexStart => Some(taffy::style::JustifyContent::FlexStart),
                YgJustify::YgjCenter => Some(taffy::style::JustifyContent::Center),
                YgJustify::YgjFlexEnd => Some(taffy::style::JustifyContent::FlexEnd),
                YgJustify::YgjSpaceBetween => Some(taffy::style::JustifyContent::SpaceBetween),
                YgJustify::YgjSpaceAround => Some(taffy::style::JustifyContent::SpaceAround),
                YgJustify::YgjSpaceEvenly => Some(taffy::style::JustifyContent::SpaceEvenly),
            },
            flex_basis: val!(
                value,
                flex_basis_unit,
                flex_basis,
                taffy::style::Dimension::Auto,
                Dimension
            ),
            size: size!(
                value,
                width_unit,
                width,
                height_unit,
                height,
                taffy::style::Dimension::Auto,
                Dimension
            ),
            min_size: size!(
                value,
                min_width_unit,
                min_width,
                min_height_unit,
                min_height,
                taffy::style::Dimension::Auto,
                Dimension
            ),
            max_size: size!(
                value,
                max_width_unit,
                max_width,
                max_height_unit,
                max_height,
                taffy::style::Dimension::Auto,
                Dimension
            ),
            inset: rect!(
                value,
                position_left_unit,
                position_left,
                position_right_unit,
                position_right,
                position_top_unit,
                position_top,
                position_bottom_unit,
                position_bottom,
                taffy::style::LengthPercentageAuto::Auto,
                LengthPercentageAuto
            ),
            margin: rect!(
                value,
                margin_left_unit,
                margin_left,
                margin_right_unit,
                margin_right,
                margin_top_unit,
                margin_top,
                margin_bottom_unit,
                margin_bottom,
                taffy::style::LengthPercentageAuto::Points(0.0),
                LengthPercentageAuto
            ),
            padding: rect_a!(
                value,
                padding_left_unit,
                padding_left,
                padding_right_unit,
                padding_right,
                padding_top_unit,
                padding_top,
                padding_bottom_unit,
                padding_bottom,
                taffy::style::LengthPercentage::Points(0.0),
                LengthPercentage
            ),
            ..Default::default()
        }
    }
}

pub fn update_scene_ui(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let ui_transform_component = SceneCrdtStateProtoComponents::get_ui_transform(crdt_state);
    let ui_background_component = SceneCrdtStateProtoComponents::get_ui_background(crdt_state);
    let ui_text_component = SceneCrdtStateProtoComponents::get_ui_text(crdt_state);

    let need_skip = dirty_lww_components
        .get(&SceneComponentId::UI_TRANSFORM)
        .is_none()
        && dirty_lww_components
            .get(&SceneComponentId::UI_BACKGROUND)
            .is_none()
        && dirty_lww_components
            .get(&SceneComponentId::UI_TEXT)
            .is_none();
    if need_skip {
        return;
    }

    if let Some(dirty_transform) = dirty_lww_components.get(&SceneComponentId::UI_TRANSFORM) {
        for entity in dirty_transform {
            let new_parent = if let Some(entry) = ui_transform_component.get(*entity) {
                SceneEntityId::from_i32(entry.value.as_ref().unwrap().parent)
            } else {
                SceneEntityId::ROOT
            };

            let node = godot_dcl_scene.ensure_node_ui(entity);
            node.parent_ui = new_parent;
        }
    }
    let mut root_node_ui = godot_dcl_scene
        .root_node_ui
        .clone()
        .upcast::<godot::prelude::Node>();

    if let Some(dirty_ui_background) = dirty_lww_components.get(&SceneComponentId::UI_BACKGROUND) {
        for entity in dirty_ui_background {
            let value = if let Some(entry) = ui_background_component.get(*entity) {
                entry.value.clone()
            } else {
                None
            };

            let existing_ui_background = godot_dcl_scene
                .ensure_node_ui(entity)
                .base_ui
                .as_mut()
                .unwrap();

            if value.is_none() {
                if let Some(node) = existing_ui_background.base_control.get_node("bkg".into()) {
                    existing_ui_background.base_control.remove_child(node);
                }

                continue;
            }

            let value = value.as_ref().unwrap();

            let mut existing_ui_background_control = if let Some(node) = existing_ui_background
                .base_control
                .get_node_or_null("bkg".into())
            {
                node.cast::<DclUiBackground>()
            } else {
                // let mut node = Gd::ne<DclUiBackground>::new_alloc();
                let mut node: Gd<DclUiBackground> = Gd::new_default();
                node.set_name("bkg".into());
                node.set_anchors_preset(godot::engine::control::LayoutPreset::PRESET_FULL_RECT);

                existing_ui_background
                    .base_control
                    .add_child(node.clone().upcast());
                existing_ui_background
                    .base_control
                    .move_child(node.clone().upcast(), 0);
                node
            };

            existing_ui_background_control
                .bind_mut()
                .change_value(value.clone(), &scene.content_mapping);
        }
    }

    if let Some(dirty_ui_text) = dirty_lww_components.get(&SceneComponentId::UI_TEXT) {
        for entity in dirty_ui_text {
            let value = if let Some(entry) = ui_text_component.get(*entity) {
                entry.value.clone()
            } else {
                None
            };

            let existing_ui_text = godot_dcl_scene
                .ensure_node_ui(entity)
                .base_ui
                .as_mut()
                .unwrap();

            if value.is_none() {
                if let Some(node) = existing_ui_text.base_control.get_node("text".into()) {
                    existing_ui_text.base_control.remove_child(node);
                }

                continue;
            }

            let value = value.as_ref().unwrap();

            let mut existing_ui_text_control = if let Some(node) = existing_ui_text
                .base_control
                .get_node_or_null("text".into())
            {
                node.cast::<Label>()
            } else {
                let mut node = Label::new_alloc();
                node.set_name("text".into());
                node.set_anchors_preset(godot::engine::control::LayoutPreset::PRESET_FULL_RECT);

                existing_ui_text
                    .base_control
                    .add_child(node.clone().upcast());
                existing_ui_text
                    .base_control
                    .move_child(node.clone().upcast(), 1);
                node
            };
            existing_ui_text_control
                .add_theme_font_size_override("font_size".into(), value.font_size.unwrap_or(10));
            let font_color = value
                .color
                .as_ref()
                .map(|v| godot::prelude::Color {
                    r: v.r,
                    g: v.g,
                    b: v.b,
                    a: v.a,
                })
                .unwrap_or(godot::prelude::Color::WHITE);
            existing_ui_text_control.add_theme_color_override("font_color".into(), font_color);

            let text_align = value
                .text_align
                .map(TextAlignMode::from_i32)
                .unwrap_or(Some(TextAlignMode::TamMiddleCenter))
                .unwrap();

            let (hor_align, vert_align) = match text_align {
                TextAlignMode::TamTopLeft => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_LEFT,
                    VerticalAlignment::VERTICAL_ALIGNMENT_TOP,
                ),
                TextAlignMode::TamTopCenter => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_CENTER,
                    VerticalAlignment::VERTICAL_ALIGNMENT_TOP,
                ),
                TextAlignMode::TamTopRight => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_RIGHT,
                    VerticalAlignment::VERTICAL_ALIGNMENT_TOP,
                ),
                TextAlignMode::TamMiddleLeft => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_LEFT,
                    VerticalAlignment::VERTICAL_ALIGNMENT_CENTER,
                ),
                TextAlignMode::TamMiddleCenter => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_CENTER,
                    VerticalAlignment::VERTICAL_ALIGNMENT_CENTER,
                ),
                TextAlignMode::TamMiddleRight => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_RIGHT,
                    VerticalAlignment::VERTICAL_ALIGNMENT_CENTER,
                ),
                TextAlignMode::TamBottomLeft => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_LEFT,
                    VerticalAlignment::VERTICAL_ALIGNMENT_BOTTOM,
                ),
                TextAlignMode::TamBottomCenter => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_CENTER,
                    VerticalAlignment::VERTICAL_ALIGNMENT_BOTTOM,
                ),
                TextAlignMode::TamBottomRight => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_RIGHT,
                    VerticalAlignment::VERTICAL_ALIGNMENT_BOTTOM,
                ),
            };

            existing_ui_text_control.set_vertical_alignment(vert_align);
            existing_ui_text_control.set_horizontal_alignment(hor_align);
            existing_ui_text_control.set_text(value.value.clone().into());
        }
    }

    let mut unprocessed_uis = godot_dcl_scene.ui_entities.clone();
    let mut processed_nodes = HashMap::with_capacity(unprocessed_uis.len());
    let mut processed_nodes_sorted = Vec::with_capacity(unprocessed_uis.len());

    let width = 800.0;
    let height = 600.0;
    let mut taffy = taffy::Taffy::new();

    let viewport_style = taffy::style::Style {
        display: taffy::style::Display::Grid,
        // Note: Taffy percentages are floats ranging from 0.0 to 1.0.
        // So this is setting width:100% and height:100%
        size: taffy::geometry::Size {
            width: taffy::style::Dimension::Percent(1.0),
            height: taffy::style::Dimension::Percent(1.0),
        },
        align_items: Some(taffy::style::AlignItems::Start),
        justify_items: Some(taffy::style::JustifyItems::Start),
        ..Default::default()
    };

    let root_node = taffy
        .new_leaf(viewport_style)
        .expect("failed to create root node");

    processed_nodes.insert(SceneEntityId::ROOT, root_node);

    let mut modified = true;
    while modified && !unprocessed_uis.is_empty() {
        modified = false;
        unprocessed_uis.retain(|entity| {
            let Some(ui_transform) = ui_transform_component.values.get(entity) else {
                return true;
            };
            let Some(ui_transform) = ui_transform.value.as_ref() else {
                return true;
            };

            // if our rightof is not added, we can't process this node
            if !processed_nodes.contains_key(&SceneEntityId::from_i32(ui_transform.right_of)) {
                return true;
            }

            // if our parent is not added, we can't process this node
            let Some(parent) = processed_nodes.get(&SceneEntityId::from_i32(ui_transform.parent))
            else {
                return true;
            };

            let child = taffy
                .new_leaf(ui_transform.into())
                .expect("failed to create node");

            taffy.add_child(*parent, child).unwrap();
            processed_nodes.insert(*entity, child);
            processed_nodes_sorted.push((*entity, child));

            // mark to continue and remove from unprocessed
            modified = true;
            false
        })
    }

    let size = taffy::prelude::Size {
        width: taffy::style::AvailableSpace::Definite(width),
        height: taffy::style::AvailableSpace::Definite(height),
    };

    taffy
        .compute_layout(root_node, size)
        .expect("failed to compute layout");

    tracing::debug!("number of node to process {}", processed_nodes.len());

    let mut idx = 0;
    for (entity, key_node) in processed_nodes_sorted {
        let ui_node = godot_dcl_scene.get_godot_entity_node(&entity).unwrap();
        let parent_position =
            if let Some(parent) = godot_dcl_scene.get_godot_entity_node(&ui_node.parent_ui) {
                parent.base_ui.as_ref().unwrap().base_control.get_position()
            } else {
                godot::prelude::Vector2::new(0.0, 0.0)
            };

        let ui_node = ui_node.base_ui.as_ref().unwrap();
        let mut control = ui_node.base_control.clone();

        let layout = taffy.layout(key_node).unwrap();
        let computed_position = parent_position
            + godot::prelude::Vector2 {
                x: layout.location.x,
                y: layout.location.y,
            };

        control.set_position(computed_position);
        control.set_size(godot::prelude::Vector2 {
            x: layout.size.width,
            y: layout.size.height,
        });
        if entity != SceneEntityId::ROOT {
            root_node_ui.move_child(control.upcast(), idx);
            idx += 1;
        }
    }
}
