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
    scene_runner::scene_manager::Scene,
};
use godot::{
    engine::{
        node::InternalMode, packed_scene::GenEditState, Material, MeshInstance3D,
        StandardMaterial3D,
    },
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
}

pub fn update_material(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_components = &scene.current_dirty.components;
    let material_component = SceneCrdtStateProtoComponents::get_material(crdt_state);

    if let Some(material_dirty) = dirty_components.get(&SceneComponentId::MATERIAL) {
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
                if let Some(material) = material.material {
                    Some(material)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(proto_material) = new_value {
                let mut godot_material = if node.material.is_some() {
                    node.material.as_ref().clone().unwrap().share()
                } else {
                    node.material = Some(StandardMaterial3D::new());
                    node.material.as_ref().clone().unwrap().share()
                };
                match proto_material {
                    pb_material::Material::Unlit(_unlit_material) => {
                        //     message UnlitMaterial {
                        //       optional decentraland.common.TextureUnion texture = 1; // default = null
                        //       optional float alpha_test = 2; // default = 0.5. range value: from 0 to 1
                        //       optional bool cast_shadows = 3; // default =  true
                        //       optional decentraland.common.Color4 diffuse_color = 4; // default = white;
                        //     }
                        godot_material.set_albedo(Color::from_rgb(0.0, 1.0, 0.0));
                    }
                    pb_material::Material::Pbr(pbr_material) => {
                        // message PbrMaterial {
                        //   optional decentraland.common.TextureUnion texture = 1; // default = null

                        //   optional float alpha_test = 2; // default = 0.5. range value: from 0 to 1
                        //   optional bool cast_shadows = 3; // default =  true

                        //   optional decentraland.common.TextureUnion alpha_texture = 4; // default = null
                        //   optional decentraland.common.TextureUnion emissive_texture = 5; // default = null
                        //   optional decentraland.common.TextureUnion bump_texture = 6; // default = null

                        //   optional decentraland.common.Color4 albedo_color = 7; // default = white;
                        //   optional decentraland.common.Color3 emissive_color = 8; // default = black;
                        //   optional decentraland.common.Color3 reflectivity_color = 9; // default = white;

                        //   optional MaterialTransparencyMode transparency_mode = 10; // default = TransparencyMode.Auto

                        //   optional float metallic = 11; // default = 0.5
                        //   optional float roughness = 12; // default = 0.5
                        //   optional float glossiness = 13; // default = 1

                        //   optional float specular_intensity = 14; // default = 1
                        //   optional float emissive_intensity = 15; // default = 2
                        //   optional float direct_intensity = 16; // default = 1
                        // }
                        godot_material.set_metallic(pbr_material.metallic.unwrap_or(0.5) as f64);
                        godot_material.set_roughness(pbr_material.roughness.unwrap_or(0.5) as f64);
                        godot_material
                            .set_specular(pbr_material.specular_intensity.unwrap_or(1.0) as f64);

                        godot_material.set_emission_intensity(
                            pbr_material.emissive_intensity.unwrap_or(2.0) as f64,
                        );
                        // godot_material.set_emission(
                        //     pbr_material
                        //         .emissive_color
                        //         .unwrap_or(Color3::white())
                        //         .to_godot(),
                        // );
                        godot_material.set_emission(
                            pbr_material
                                .emissive_color
                                .unwrap_or(Color3::white())
                                .to_godot(),
                        );
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
            } else {
                if let Some(mut mesh_renderer) = mesh_renderer {
                    mesh_renderer.call(
                        "set_surface_override_material".into(),
                        &[0.to_variant(), Variant::nil()],
                    );
                    node.material.take();
                }
            }
        }
    }
}
