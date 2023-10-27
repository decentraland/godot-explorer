use crate::dcl::components::{
    proto_components::sdk::components::{
        PbUiTransform, PointerFilterMode, YgAlign, YgDisplay, YgFlexDirection, YgJustify,
        YgOverflow, YgPositionType, YgUnit, YgWrap,
    },
    SceneEntityId,
};

// macro helpers to convert proto format to bevy format for val, size, rect
macro_rules! val {
    ($pb:ident, $u:ident, $v:ident, $d:expr, $t:ident) => {
        match $pb.$u() {
            YgUnit::YguUndefined => $d,
            YgUnit::YguAuto => taffy::style::$t::Auto,
            YgUnit::YguPoint => taffy::style::$t::Length($pb.$v),
            YgUnit::YguPercent => taffy::style::$t::Percent($pb.$v / 100.0),
        }
    };
}
macro_rules! val_a {
    ($pb:ident, $u:ident, $v:ident, $d:expr, $t:ident) => {
        match $pb.$u() {
            YgUnit::YguAuto | YgUnit::YguUndefined => $d,
            YgUnit::YguPoint => taffy::style::$t::Length($pb.$v),
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

#[derive(Default)]
pub struct UiTransform {
    pub parent: SceneEntityId,
    pub right_of: SceneEntityId,
    pub overflow: YgOverflow,
    pub pointer_filter_mode: PointerFilterMode,
    pub taffy_style: taffy::style::Style,
}

impl From<&PbUiTransform> for UiTransform {
    fn from(value: &PbUiTransform) -> Self {
        Self {
            parent: SceneEntityId::from_i32(value.parent),
            right_of: SceneEntityId::from_i32(value.right_of),
            overflow: value.overflow(),
            pointer_filter_mode: value.pointer_filter(),
            taffy_style: taffy::style::Style {
                overflow: match value.overflow() {
                    YgOverflow::YgoVisible => taffy::geometry::Point::<taffy::style::Overflow> {
                        x: taffy::style::Overflow::Visible,
                        y: taffy::style::Overflow::Visible,
                    },
                    YgOverflow::YgoScroll => taffy::geometry::Point::<taffy::style::Overflow> {
                        x: taffy::style::Overflow::Scroll,
                        y: taffy::style::Overflow::Scroll,
                    },
                    YgOverflow::YgoHidden => taffy::geometry::Point::<taffy::style::Overflow> {
                        x: taffy::style::Overflow::Hidden,
                        y: taffy::style::Overflow::Hidden,
                    },
                },
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
                    YgFlexDirection::YgfdColumnReverse => {
                        taffy::style::FlexDirection::ColumnReverse
                    }
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
                    taffy::style::LengthPercentageAuto::Length(0.0),
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
                    taffy::style::LengthPercentage::Length(0.0),
                    LengthPercentage
                ),
                ..Default::default()
            },
        }
    }
}
