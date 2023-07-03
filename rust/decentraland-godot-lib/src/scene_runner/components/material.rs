use crate::{
    dcl::{
        components::{
            proto_components::{
                common::{Color3, Color4},
                sdk::components::pb_material,
            },
            SceneComponentId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};
use godot::{
    engine::{MeshInstance3D, StandardMaterial3D},
    prelude::*,
};

impl Color4 {
    pub fn black() -> Self {
        Self {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }
    }
    pub fn white() -> Self {
        Self {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }
    pub fn to_godot(&self) -> Color {
        Color::from_rgba(self.r, self.g, self.b, self.a)
    }

    pub fn multiply(&mut self, factor: f32) -> Self {
        Self {
            r: self.r * factor,
            g: self.g * factor,
            b: self.b * factor,
            a: self.a * factor,
        }
    }
}

impl Color3 {
    pub fn black() -> Self {
        Self {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        }
    }
    pub fn white() -> Self {
        Self {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        }
    }
    pub fn to_godot(&self) -> Color {
        Color::from_rgba(self.r, self.g, self.b, 1.0)
    }

    pub fn multiply(&mut self, factor: f32) -> Self {
        Self {
            r: self.r * factor,
            g: self.g * factor,
            b: self.b * factor,
        }
    }
}

pub fn update_material(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let material_component = SceneCrdtStateProtoComponents::get_material(crdt_state);

    if let Some(material_dirty) = dirty_lww_components.get(&SceneComponentId::MATERIAL) {
        for entity in material_dirty {
            let new_value = material_component.get(*entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let node = godot_dcl_scene.ensure_node_mut(entity);

            let mesh_renderer = node
                .base
                .try_get_node_as::<MeshInstance3D>(NodePath::from("MeshRenderer"));

            let new_value = if let Some(material) = new_value.value.clone() {
                material.material
            } else {
                None
            };

            if let Some(proto_material) = new_value {
                let mut godot_material = if node.material.is_some() {
                    node.material.as_ref().unwrap().share()
                } else {
                    node.material = Some(StandardMaterial3D::new());
                    node.material.as_ref().unwrap().share()
                };
                match proto_material {
                    pb_material::Material::Unlit(_unlit_material) => {
                        // TODO: unlit not implemented yet
                        godot_material.set_albedo(Color::from_rgb(0.0, 1.0, 0.0));
                    }
                    pb_material::Material::Pbr(pbr_material) => {
                        godot_material.set_metallic(pbr_material.metallic.unwrap_or(0.5) as f64);
                        godot_material.set_roughness(pbr_material.roughness.unwrap_or(0.5) as f64);

                        godot_material
                            .set_specular(pbr_material.specular_intensity.unwrap_or(1.0) as f64);

                        let emission = pbr_material
                            .emissive_color
                            .unwrap_or(Color3::white())
                            .multiply(pbr_material.emissive_intensity.unwrap_or(2.0));
                        godot_material.set_emission(emission.to_godot());

                        godot_material.set_albedo(
                            pbr_material
                                .albedo_color
                                .unwrap_or(Color4::black())
                                .to_godot(),
                        );
                    }
                }

                if let Some(mut mesh_renderer) = mesh_renderer {
                    mesh_renderer.set_surface_override_material(0, godot_material.upcast());
                }
            } else if let Some(mut mesh_renderer) = mesh_renderer {
                mesh_renderer.call(
                    "set_surface_override_material".into(),
                    &[0.to_variant(), Variant::nil()],
                );
                node.material.take();
            }
        }
    }
}
