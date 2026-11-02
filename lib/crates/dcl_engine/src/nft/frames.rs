//! The picture frames of an NFTShape, by the SDK's `NftFrameType`.

use godot::classes::{Material, PackedScene};
use godot::prelude::*;

/// SDK `NftFrameType` values, in order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FrameStyle {
    Classic,
    BaroqueOrnament,
    DiamondOrnament,
    MinimalWide,
    MinimalGrey,
    Blocky,
    GoldEdges,
    GoldCarved,
    GoldWide,
    GoldRounded,
    MetalMedium,
    MetalWide,
    MetalSlim,
    MetalRounded,
    Pins,
    MinimalBlack,
    MinimalWhite,
    Tape,
    WoodSlim,
    WoodWide,
    WoodTwigs,
    Canvas,
    None,
}

const STYLES: [FrameStyle; 23] = [
    FrameStyle::Classic,
    FrameStyle::BaroqueOrnament,
    FrameStyle::DiamondOrnament,
    FrameStyle::MinimalWide,
    FrameStyle::MinimalGrey,
    FrameStyle::Blocky,
    FrameStyle::GoldEdges,
    FrameStyle::GoldCarved,
    FrameStyle::GoldWide,
    FrameStyle::GoldRounded,
    FrameStyle::MetalMedium,
    FrameStyle::MetalWide,
    FrameStyle::MetalSlim,
    FrameStyle::MetalRounded,
    FrameStyle::Pins,
    FrameStyle::MinimalBlack,
    FrameStyle::MinimalWhite,
    FrameStyle::Tape,
    FrameStyle::WoodSlim,
    FrameStyle::WoodWide,
    FrameStyle::WoodTwigs,
    FrameStyle::Canvas,
    FrameStyle::None,
];

impl FrameStyle {
    /// The style of an SDK value; None for one out of range.
    pub fn from_i32(value: i32) -> Option<FrameStyle> {
        usize::try_from(value)
            .ok()
            .and_then(|i| STYLES.get(i).copied())
    }

    pub fn to_i32(self) -> i32 {
        STYLES.iter().position(|s| *s == self).unwrap_or(0) as i32
    }

    pub fn asset_path(self) -> &'static str {
        match self {
            FrameStyle::Classic => "res://assets/nftshape/Classic.glb",
            FrameStyle::BaroqueOrnament => "res://assets/nftshape/Baroque_Ornament.glb",
            FrameStyle::DiamondOrnament => "res://assets/nftshape/Diamond_Ornament.glb",
            FrameStyle::MinimalWide => "res://assets/nftshape/Minimal_Wide.glb",
            FrameStyle::MinimalGrey => "res://assets/nftshape/Minimal_Grey.glb",
            FrameStyle::Blocky => "res://assets/nftshape/Blocky.glb",
            FrameStyle::GoldEdges => "res://assets/nftshape/Gold_Edges.glb",
            FrameStyle::GoldCarved => "res://assets/nftshape/Gold_Carved.glb",
            FrameStyle::GoldWide => "res://assets/nftshape/Gold_Wide.glb",
            FrameStyle::GoldRounded => "res://assets/nftshape/Gold_Rounded.glb",
            FrameStyle::MetalMedium => "res://assets/nftshape/Metal_Medium.glb",
            FrameStyle::MetalWide => "res://assets/nftshape/Metal_Wide.glb",
            FrameStyle::MetalSlim => "res://assets/nftshape/Metal_Slim.glb",
            FrameStyle::MetalRounded => "res://assets/nftshape/Metal_Rounded.glb",
            FrameStyle::Pins => "res://assets/nftshape/Pins.glb",
            FrameStyle::MinimalBlack => "res://assets/nftshape/Minimal_Black.glb",
            FrameStyle::MinimalWhite => "res://assets/nftshape/Minimal_White.glb",
            FrameStyle::Tape => "res://assets/nftshape/Tape.glb",
            FrameStyle::WoodSlim => "res://assets/nftshape/Wood_Slim.glb",
            FrameStyle::WoodWide => "res://assets/nftshape/Wood_Wide.glb",
            FrameStyle::WoodTwigs => "res://assets/nftshape/Wood_Twigs.glb",
            FrameStyle::Canvas => "res://assets/nftshape/Canvas.glb",
            FrameStyle::None => "res://assets/nftshape/empty_frame.tscn",
        }
    }

    /// The material that holds the picture: a plane's own, else the frame's `PictureFrame`
    /// surface (the Classic frame's is `Mat_CK`).
    pub fn picture_material_name(self) -> &'static str {
        match self {
            FrameStyle::Classic => "Mat_CK",
            _ => "PictureFrame",
        }
    }
}

/// A fresh instance of the frame.
pub fn instantiate(style: FrameStyle) -> Option<Gd<Node3D>> {
    let scene = try_load::<PackedScene>(style.asset_path()).ok()?;
    let mut node = scene.instantiate()?.try_cast::<Node3D>().ok()?;
    if style == FrameStyle::Classic {
        // The Classic glTF is authored lying down.
        node.rotate_x(-std::f32::consts::FRAC_PI_2);
        node.rotate_y(std::f32::consts::PI);
    }
    Some(node)
}

/// The animated material shown while the picture downloads.
pub fn loading_material() -> Option<Gd<Material>> {
    try_load::<Material>("res://assets/nftshape/material_loading_animation.tres").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_values_map_in_order() {
        assert_eq!(FrameStyle::from_i32(0), Some(FrameStyle::Classic));
        assert_eq!(FrameStyle::from_i32(21), Some(FrameStyle::Canvas));
        assert_eq!(FrameStyle::from_i32(22), Some(FrameStyle::None));
        assert_eq!(FrameStyle::from_i32(23), None);
        assert_eq!(FrameStyle::from_i32(-1), None);
        for value in 0..=22 {
            assert_eq!(FrameStyle::from_i32(value).unwrap().to_i32(), value);
        }
    }

    #[test]
    fn only_the_classic_frame_uses_its_own_picture_material() {
        assert_eq!(FrameStyle::Classic.picture_material_name(), "Mat_CK");
        assert_eq!(FrameStyle::Pins.picture_material_name(), "PictureFrame");
    }

    #[test]
    fn every_style_has_an_asset() {
        for style in STYLES {
            assert!(style.asset_path().starts_with("res://assets/nftshape/"));
        }
    }
}
