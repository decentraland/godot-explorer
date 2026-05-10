//! Texture2DArray-backed material atlas.

use std::collections::HashMap;

use godot::classes::image::Format;
use godot::classes::{Image, ImageTexture, Texture2D, Texture2DArray};
use godot::prelude::*;

pub const CELL_SIZE: i32 = 512;
pub const MAX_LAYERS: usize = 256;

#[derive(Clone, Copy, Debug)]
pub struct LayerParams {
    pub albedo_factor: Color,
    pub metallic: f32,
    pub roughness: f32,
    pub emissive_intensity: f32,
    pub alpha_cutoff: f32,
}

impl Default for LayerParams {
    fn default() -> Self {
        Self {
            albedo_factor: Color::from_rgba(1.0, 1.0, 1.0, 1.0),
            metallic: 0.0,
            roughness: 1.0,
            emissive_intensity: 0.0,
            alpha_cutoff: 0.5,
        }
    }
}

pub struct MaterialAtlas {
    layer_albedos: Vec<Gd<Image>>,
    layer_params: Vec<LayerParams>,
    texture_to_layer: HashMap<i64, u32>,

    pub albedo_array: Gd<Texture2DArray>,
    pub params_tex: Gd<ImageTexture>,
    pub colors_tex: Gd<ImageTexture>,
    pub layer_count: u32,
}

impl MaterialAtlas {
    pub fn new() -> Self {
        // Pre-allocate the entire array of blank layers up-front so we never
        // need to call create_from_images again — that copies every layer's
        // pixels to GPU each time and dominates load time. Per-layer fills
        // happen via RenderingServer::texture_2d_update() against the
        // texture's RID, which is cheap.
        let mut layer_albedos: Vec<Gd<Image>> = Vec::with_capacity(MAX_LAYERS);
        let mut images: Array<Gd<Image>> = Array::new();
        for _ in 0..MAX_LAYERS {
            let img = make_blank_albedo();
            images.push(&img);
            layer_albedos.push(img);
        }
        let mut array = Texture2DArray::new_gd();
        let _ = array.create_from_images(&images);

        let params = make_blank_param_texture(MAX_LAYERS as i32);
        let colors = make_blank_color_texture(MAX_LAYERS as i32);

        Self {
            layer_albedos,
            layer_params: vec![LayerParams::default(); MAX_LAYERS],
            texture_to_layer: HashMap::new(),
            albedo_array: array,
            params_tex: params,
            colors_tex: colors,
            layer_count: 1,
        }
    }

    pub fn allocate_layer(
        &mut self,
        source_albedo: Option<Gd<Texture2D>>,
        params: LayerParams,
    ) -> Option<u32> {
        if self.layer_count as usize >= MAX_LAYERS {
            return None;
        }

        if let Some(tex) = source_albedo.as_ref() {
            let key = tex.get_rid().to_u64() as i64;
            if let Some(&layer) = self.texture_to_layer.get(&key) {
                return Some(layer);
            }
        }

        let albedo_image = match source_albedo.as_ref().and_then(|t| t.get_image()) {
            Some(img) => resize_albedo(img),
            None => make_blank_albedo(),
        };

        let layer = self.layer_count;
        self.layer_albedos[layer as usize] = albedo_image.clone();
        self.layer_params[layer as usize] = params;
        if let Some(tex) = source_albedo {
            self.texture_to_layer
                .insert(tex.get_rid().to_u64() as i64, layer);
        }
        self.layer_count += 1;

        self.update_array_layer(layer, &albedo_image);
        self.upload_layer_params(layer, params);

        Some(layer)
    }

    fn update_array_layer(&self, layer: u32, image: &Gd<Image>) {
        // Update one slice of the Texture2DArray in place. Requires going
        // through RenderingServer because Texture2DArray itself only
        // exposes create_from_images (full rebuild). The RID stays the
        // same → ShaderMaterial doesn't need re-binding.
        let mut rs = godot::classes::RenderingServer::singleton();
        rs.texture_2d_update(self.albedo_array.get_rid(), image, layer as i32);
    }

    fn upload_layer_params(&mut self, layer: u32, params: LayerParams) {
        if let Some(mut params_img) = self.params_tex.get_image() {
            params_img.set_pixel(
                0,
                layer as i32,
                Color::from_rgba(
                    params.metallic,
                    params.roughness,
                    params.emissive_intensity,
                    params.alpha_cutoff,
                ),
            );
            self.params_tex.update(&params_img);
        }
        if let Some(mut colors_img) = self.colors_tex.get_image() {
            colors_img.set_pixel(0, layer as i32, params.albedo_factor);
            self.colors_tex.update(&colors_img);
        }
    }
}

fn make_blank_albedo() -> Gd<Image> {
    // use_mipmaps=true so Texture2DArray.create_from_images allocates the
    // mipmap chain. Subsequent per-layer updates via texture_2d_update must
    // also include mipmaps or the size assertion fails:
    // "Required size for texture update (1048576) does not match data
    // supplied size (1398100)" — 1398100 ≈ 512×512×4 × 4/3 (mipmap chain).
    let mut img = Image::create(CELL_SIZE, CELL_SIZE, true, Format::RGBA8)
        .expect("Image::create should succeed for atlas blank");
    img.fill(Color::from_rgba(1.0, 1.0, 1.0, 1.0));
    img.generate_mipmaps();
    img
}

fn make_blank_param_texture(rows: i32) -> Gd<ImageTexture> {
    let mut img =
        Image::create(1, rows, false, Format::RGBAF).expect("param texture must allocate");
    img.fill(Color::from_rgba(0.0, 1.0, 0.0, 0.5));
    ImageTexture::create_from_image(&img).unwrap_or_else(ImageTexture::new_gd)
}

fn make_blank_color_texture(rows: i32) -> Gd<ImageTexture> {
    let mut img =
        Image::create(1, rows, false, Format::RGBAF).expect("color texture must allocate");
    img.fill(Color::from_rgba(1.0, 1.0, 1.0, 1.0));
    ImageTexture::create_from_image(&img).unwrap_or_else(ImageTexture::new_gd)
}

fn resize_albedo(mut img: Gd<Image>) -> Gd<Image> {
    // Android imports GLTF textures as ETC2/ASTC. resize/convert/generate_mipmaps
    // all error out on compressed formats. Decompress first so the rest of the
    // chain works on raw RGBA8.
    if img.is_compressed() {
        let err = img.decompress();
        if err != godot::global::Error::OK {
            tracing::warn!(
                "material_atlas: Image::decompress failed ({:?}); returning blank albedo",
                err
            );
            return make_blank_albedo();
        }
    }
    let w = img.get_width();
    let h = img.get_height();
    if w != CELL_SIZE || h != CELL_SIZE {
        img.resize(CELL_SIZE, CELL_SIZE);
    }
    if img.get_format() != Format::RGBA8 {
        img.convert(Format::RGBA8);
    }
    img.generate_mipmaps();
    img
}
