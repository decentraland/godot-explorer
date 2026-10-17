//! Scaffolding for the `uiBackground` `wrapMode` visual regression test (issue #2506).
//!
//! The unit tests in `dcl_ui_background` pin the wrap-mode *mapping*; they cannot catch
//! the actual bug, which was that nothing ever applied the mapping to the node that
//! samples the texture. Only a rendered diff covers that wiring.
//!
//! This lives on the Rust side because `DclUiBackground::change_value` takes a protobuf
//! struct and is not callable from GDScript, but the capture itself has to be driven from
//! GDScript: rendering needs `await RenderingServer.frame_post_draw`, and an `#[itest]`
//! runs inside a single frame with no way to yield to the main loop (a `ColorRect` added
//! from one does not draw either, so a snapshot taken there is always blank).

use godot::classes::{
    image::Format as ImageFormat, Control, IControl, Image, ImageTexture, Texture2D,
};
use godot::prelude::*;

use crate::{
    content::{
        content_mapping::{ContentMappingAndUrl, ContentMappingAndUrlRef},
        texture::TextureEntry,
    },
    dcl::{
        common::content_entity::TypedIpfsRef,
        components::proto_components::{
            common::{texture_union::Tex, Texture, TextureUnion, TextureWrapMode},
            sdk::components::{BackgroundTextureMode, PbUiBackground},
        },
    },
    godot_classes::{
        dcl_config::TextureQuality, dcl_global::DclGlobal, dcl_ui_background::DclUiBackground,
        promise::Promise,
    },
};

/// Power-of-two, uncompressed: the snapshot is crisp and identical across renderers, so
/// the committed baseline is stable in CI (unlike an ETC2/GPU-decoded texture would be).
const ATLAS_WIDTH: i32 = 128;
const ATLAS_HEIGHT: i32 = 64;

/// Arbitrary; the texture is pre-seeded into the promise cache under this key, so it is
/// never used to hit a content server.
const ATLAS_HASH: &str = "dcltest-uibackground-repeat-atlas";
const ATLAS_FILE: &str = "atlas.png";

/// Four saturated vertical stripes plus a horizontal marker band. Chosen so that
/// "tiled 1.5 times" (R G B Y R G) and "last column smeared" (R G B Y Y Y) are
/// unmistakably different images rather than a subtle filtering difference.
fn build_atlas_image() -> Gd<Image> {
    const STRIPES: [[u8; 3]; 4] = [
        [255, 0, 0],   // red
        [0, 200, 40],  // green
        [30, 80, 255], // blue
        [255, 220, 0], // yellow
    ];

    let mut pixels = PackedByteArray::new();
    for y in 0..ATLAS_HEIGHT {
        for x in 0..ATLAS_WIDTH {
            // The sampled UV band is only the top ~20% of the atlas, so the marker band
            // has to live inside it to be visible at all.
            let marker = (4..8).contains(&y);
            let stripe = STRIPES[(x * 4 / ATLAS_WIDTH) as usize];
            if marker {
                pixels.push(255);
                pixels.push(255);
                pixels.push(255);
            } else {
                for channel in stripe {
                    pixels.push(channel);
                }
            }
            pixels.push(255);
        }
    }

    Image::create_from_data(
        ATLAS_WIDTH,
        ATLAS_HEIGHT,
        false,
        ImageFormat::RGBA8,
        &pixels,
    )
    .expect("failed to build the deterministic atlas image")
}

/// Resolves the texture offline by pre-seeding the exact cache entry that
/// `fetch_texture_by_hash_with_quality` looks up first, so `change_value` takes its
/// normal path but never downloads anything.
fn seed_texture_cache(texture: Gd<Texture2D>, image: Gd<Image>) -> Gd<Promise> {
    let entry = Gd::from_init_fn(|_base| TextureEntry {
        original_size: image.get_size(),
        image,
        texture,
        failed: false,
    });

    let promise = Promise::from_resolved(entry.to_variant());
    let quality = TextureQuality::Source;
    let cache_key = format!("{}_q{}", ATLAS_HASH, quality.to_i32());

    DclGlobal::singleton()
        .bind()
        .get_content_provider()
        .bind_mut()
        .cache_promise(cache_key, &promise);

    promise
}

fn content_mapping() -> ContentMappingAndUrlRef {
    std::sync::Arc::new(ContentMappingAndUrl::from_base_url_and_content(
        // Never dereferenced: the pre-seeded cache short-circuits before any fetch.
        "http://localhost.invalid/".to_string(),
        vec![TypedIpfsRef {
            file: ATLAS_FILE.to_string(),
            hash: ATLAS_HASH.to_string(),
        }],
    ))
}

/// Hosts a real `DclUiBackground` so a GDScript client test can render and diff it.
/// Doubles as the Control parent that `DclUiBackground::ready` requires.
#[derive(GodotClass)]
#[class(base=Control)]
pub struct DclUiBackgroundTestProbe {
    base: Base<Control>,
    /// The cache only stores the promise's `InstanceId`; a freed promise silently misses,
    /// so the probe owns it for the lifetime of the test.
    _texture_promise: Option<Gd<Promise>>,
}

#[godot_api]
impl IControl for DclUiBackgroundTestProbe {
    fn init(base: Base<Control>) -> Self {
        Self {
            base,
            _texture_promise: None,
        }
    }
}

#[godot_api]
impl DclUiBackgroundTestProbe {
    /// Builds a `uiBackground` with `wrapMode: repeat` and the UVs from the bug report.
    /// Those UVs are non-axis-aligned, so `has_custom_uvs` is true and the shader path is
    /// taken; the 1.5 is what has to tile.
    #[func]
    fn build_repeat_background(&mut self, size: Vector2) {
        let image = build_atlas_image();
        let texture: Gd<Texture2D> = ImageTexture::create_from_image(&image)
            .expect("failed to create the atlas texture")
            .upcast();
        self._texture_promise = Some(seed_texture_cache(texture, image));

        self.base_mut().set_size(size);

        let mut background = DclUiBackground::new_alloc();
        self.base_mut().add_child(&background);

        // In the real UI tree the layout engine sizes this node; nothing does so here, and
        // a zero-sized Control draws nothing.
        background.set_anchors_preset(godot::classes::control::LayoutPreset::FULL_RECT);
        background.set_size(size);

        background.bind_mut().change_value(
            PbUiBackground {
                texture: Some(TextureUnion {
                    tex: Some(Tex::Texture(Texture {
                        src: ATLAS_FILE.to_string(),
                        wrap_mode: Some(TextureWrapMode::TwmRepeat as i32),
                        ..Default::default()
                    })),
                }),
                texture_mode: BackgroundTextureMode::Stretch as i32,
                uvs: vec![0.0, 0.8, 0.0, 1.0, 1.5, 1.0, 1.5, 0.8],
                ..Default::default()
            },
            content_mapping(),
        );
    }
}
