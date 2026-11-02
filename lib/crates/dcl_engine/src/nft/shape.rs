//! `NftShape`: the frame node of one NFTShape entity. It swaps frames when the style changes,
//! shows the loading material while the asset downloads, then puts the picture in the frame and
//! scales the node to the picture's aspect.

use godot::classes::INode3D;
use godot::classes::{Mesh, MeshInstance3D, PrimitiveMesh, StandardMaterial3D, Texture2D};
use godot::prelude::*;

use super::fetcher::{fetch_nft, NftAsset};
use super::frames::{instantiate, loading_material, FrameStyle};
use super::urn::NftUrn;
use crate::engine::local_task::{await_promise, spawn_local};
use crate::godot_classes::promise::PromiseError;

/// A request for an asset, kept while another is loading.
#[derive(Clone)]
struct LoadRequest {
    urn: String,
    style: FrameStyle,
    background: Color,
}

#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct NftShape {
    current_style: Option<FrameStyle>,
    loading: bool,
    /// Requests are debounced: while one loads only the latest is kept and runs after it.
    scheduled: Option<LoadRequest>,
    current_frame: Option<Gd<Node3D>>,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for NftShape {}

impl NftShape {
    /// Loads the asset of `urn` into a frame of `style`; the background color tints the frame's
    /// background while it loads.
    pub fn load_nft(this: &Gd<NftShape>, urn: String, style: FrameStyle, background: Color) {
        spawn_local(run_requests(
            this.clone(),
            LoadRequest {
                urn,
                style,
                background,
            },
        ));
    }
}

async fn run_requests(shape: Gd<NftShape>, first: LoadRequest) {
    let mut request = first;
    loop {
        if !shape.is_instance_valid() {
            return;
        }
        {
            let mut handle = shape.clone();
            let mut this = handle.bind_mut();
            if this.loading {
                this.scheduled = Some(request);
                return;
            }
            this.loading = true;
            this.load_frame_style(request.style, request.background);
        }
        load_asset(&shape, &request).await;
        if !shape.is_instance_valid() {
            return;
        }
        let next = {
            let mut handle = shape.clone();
            let mut this = handle.bind_mut();
            this.loading = false;
            this.scheduled.take()
        };
        match next {
            Some(next) => request = next,
            None => return,
        }
    }
}

async fn load_asset(shape: &Gd<NftShape>, request: &LoadRequest) {
    let Some(urn) = NftUrn::parse(&request.urn) else {
        godot_error!("NftShape::load_nft Error, invalid urn: {}", request.urn);
        return;
    };
    let result = await_promise(fetch_nft(&urn)).await;
    if let Ok(error) = result.try_to::<Gd<PromiseError>>() {
        godot_error!(
            "NftShape::load_nft Error on fetching nft: {}",
            error.bind().get_error()
        );
        return;
    }
    let Ok(asset) = result.try_to::<Gd<NftAsset>>() else {
        return;
    };
    if !shape.is_instance_valid() {
        return;
    }
    // The frame this shape holds now: swapping frees the old ones, so the one that existed
    // before the fetch is not carried across it.
    let mut handle = shape.clone();
    let mut this = handle.bind_mut();
    let Some(frame) = this.current_frame.clone() else {
        return;
    };
    let texture = asset.bind().texture.clone();
    this.set_asset_texture(&frame, request.style, texture);
}

fn mesh_instance_of(frame: &Gd<Node3D>) -> Option<Gd<MeshInstance3D>> {
    let root = frame.find_child_ex("RootNode").recursive(false).done()?;
    root.get_children()
        .iter_shared()
        .find_map(|child| child.try_cast::<MeshInstance3D>().ok())
}

fn surface_materials(mesh: &Gd<Mesh>) -> Vec<(i32, Gd<StandardMaterial3D>)> {
    (0..mesh.get_surface_count())
        .filter_map(|i| {
            mesh.surface_get_material(i)
                .and_then(|m| m.try_cast::<StandardMaterial3D>().ok())
                .map(|m| (i, m))
        })
        .collect()
}

/// The surface whose material name contains `name`, the first surface otherwise.
fn surface_index_by_name(mesh: &Gd<Mesh>, name: &str) -> i32 {
    surface_materials(mesh)
        .into_iter()
        .find(|(_, m)| m.get_name().to_string().contains(name))
        .map_or(0, |(i, _)| i)
}

/// The shape's scale for a picture: the longer side half a meter, the aspect kept.
pub fn picture_scale(width: i32, height: i32) -> Vector3 {
    let factor = width.max(height).max(1) as f32;
    Vector3::new(
        0.5 * width as f32 / factor,
        0.5 * height as f32 / factor,
        0.5,
    )
}

impl NftShape {
    fn load_frame_style(&mut self, style: FrameStyle, background: Color) {
        if self.current_style == Some(style) {
            return;
        }
        self.current_style = Some(style);
        for mut child in self.base().get_children().iter_shared() {
            self.base_mut().remove_child(&child);
            child.queue_free();
        }
        let Some(frame) = instantiate(style) else {
            godot_error!("NftShape: no frame for {style:?}");
            self.current_frame = None;
            return;
        };
        self.set_loading_material(style, &frame, background);
        self.base_mut().add_child(&frame);
        self.current_frame = Some(frame);
    }

    fn set_loading_material(&self, style: FrameStyle, frame: &Gd<Node3D>, background: Color) {
        let Some(mut mesh_instance) = mesh_instance_of(frame) else {
            godot_error!("set nft mesh_instance_3d is null");
            return;
        };
        let Some(mesh) = mesh_instance.get_mesh() else {
            return;
        };
        if let Some(loading) = loading_material() {
            if style == FrameStyle::None {
                mesh_instance.set_surface_override_material(0, &loading);
            } else {
                let surface = surface_index_by_name(&mesh, "PictureFrame");
                mesh_instance.set_surface_override_material(surface, &loading);
            }
        }
        if let Some((_, mut material)) = surface_materials(&mesh)
            .into_iter()
            .find(|(_, m)| m.get_name().to_string().starts_with("Background"))
        {
            material.set_albedo(background);
        }
    }

    fn set_asset_texture(
        &mut self,
        frame: &Gd<Node3D>,
        style: FrameStyle,
        texture: Option<Gd<Texture2D>>,
    ) {
        let Some(mut mesh_instance) = mesh_instance_of(frame) else {
            godot_error!("set nft mesh_instance_3d is null");
            return;
        };
        let (width, height) = texture
            .as_ref()
            .map_or((256, 256), |t| (t.get_width(), t.get_height()));
        self.base_mut().set_scale(picture_scale(width, height));

        let Some(mesh) = mesh_instance.get_mesh() else {
            return;
        };
        if style == FrameStyle::None {
            // A plane: its own material carries the picture.
            let Some(material) = mesh
                .try_cast::<PrimitiveMesh>()
                .ok()
                .and_then(|m| m.get_material())
                .and_then(|m| m.try_cast::<StandardMaterial3D>().ok())
            else {
                return;
            };
            let mut material: Gd<StandardMaterial3D> = material.duplicate_resource();
            material.set_texture(
                godot::classes::base_material_3d::TextureParam::ALBEDO,
                texture.as_ref(),
            );
            mesh_instance.set_surface_override_material(0, &material);
        } else {
            let surface = surface_index_by_name(&mesh, style.picture_material_name());
            let Some(material) = mesh
                .surface_get_material(surface)
                .and_then(|m| m.try_cast::<StandardMaterial3D>().ok())
            else {
                return;
            };
            let mut material: Gd<StandardMaterial3D> = material.duplicate_resource();
            material.set_texture(
                godot::classes::base_material_3d::TextureParam::ALBEDO,
                texture.as_ref(),
            );
            mesh_instance.set_surface_override_material(surface, &material);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_longer_side_is_half_a_meter() {
        assert_eq!(picture_scale(512, 512), Vector3::new(0.5, 0.5, 0.5));
        assert_eq!(picture_scale(512, 256), Vector3::new(0.5, 0.25, 0.5));
        assert_eq!(picture_scale(100, 400), Vector3::new(0.125, 0.5, 0.5));
    }

    #[test]
    fn an_empty_texture_does_not_divide_by_zero() {
        let scale = picture_scale(0, 0);
        assert!(scale.x.is_finite() && scale.y.is_finite());
    }
}
