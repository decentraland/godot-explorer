//! Behavioural fixtures for the cheap-pbr import hook.
//!
//! Registered with the godot-rust `#[itest]` framework, so they run inside a
//! live Godot scene tree (where `Gd<T>` instantiation works) via
//! `cargo run -- run --itest` — i.e. Phase 3 ("Integration tests") of
//! `cargo run -- full-tests`. Plain `cargo test` does NOT pick them up.

#![cfg(debug_assertions)]
#![allow(dead_code)]

use godot::builtin::Color;
use godot::classes::base_material_3d::{ShadingMode, TextureParam};
use godot::classes::{BaseMaterial3D, GltfState, ImageTexture, ShaderMaterial, StandardMaterial3D};
use godot::meta::ToGodot;
use godot::obj::{Gd, NewGd};
use godot::test::itest;

use super::common::apply_pre_generate_material_overrides;

fn make_state_with_materials(materials: Vec<Gd<godot::classes::Material>>) -> Gd<GltfState> {
    let mut state = GltfState::new_gd();
    let mut arr = godot::builtin::Array::<Gd<godot::classes::Material>>::new();
    for m in materials {
        arr.push(&m);
    }
    state.set_materials(&arr);
    state
}

fn fresh_standard_material() -> Gd<StandardMaterial3D> {
    let mat = StandardMaterial3D::new_gd();
    assert_eq!(
        mat.clone().upcast::<BaseMaterial3D>().get_shading_mode(),
        ShadingMode::PER_PIXEL,
        "fixture assumption: a freshly-constructed StandardMaterial3D defaults to PER_PIXEL"
    );
    mat
}

#[itest]
fn apply_pre_generate_material_overrides_sets_per_vertex_on_every_base_material() {
    let materials: Vec<Gd<godot::classes::Material>> = (0..4)
        .map(|_| fresh_standard_material().upcast::<godot::classes::Material>())
        .collect();
    let mut state = make_state_with_materials(materials);

    apply_pre_generate_material_overrides(&mut state);

    let after = state.get_materials();
    for i in 0..after.len() {
        let m = after.at(i);
        let base = m
            .try_cast::<BaseMaterial3D>()
            .expect("StandardMaterial3D upcasts to BaseMaterial3D");
        assert_eq!(base.get_shading_mode(), ShadingMode::PER_VERTEX);
    }
}

#[itest]
fn apply_pre_generate_material_overrides_preserves_textures_and_albedo_color() {
    let mat = fresh_standard_material();
    let texture = ImageTexture::new_gd();
    let albedo = Color::from_rgba(0.2, 0.5, 0.8, 1.0);
    let texture2d = texture.clone().upcast::<godot::classes::Texture2D>();
    mat.clone()
        .upcast::<BaseMaterial3D>()
        .set_texture(TextureParam::ALBEDO, &texture2d);
    mat.clone().upcast::<BaseMaterial3D>().set_albedo(albedo);

    let mut state = make_state_with_materials(vec![mat.upcast::<godot::classes::Material>()]);
    apply_pre_generate_material_overrides(&mut state);

    let after = state.get_materials();
    let m = after.at(0);
    let base = m.try_cast::<BaseMaterial3D>().unwrap();
    assert_eq!(base.get_shading_mode(), ShadingMode::PER_VERTEX);
    assert_eq!(base.get_albedo(), albedo);
    assert!(
        base.get_texture(TextureParam::ALBEDO).is_some(),
        "albedo texture should still be bound after the override"
    );
}

#[itest]
fn apply_pre_generate_material_overrides_skips_shader_materials() {
    let shader_mat = ShaderMaterial::new_gd();
    let pre = shader_mat.to_variant();

    let mut state =
        make_state_with_materials(vec![shader_mat.upcast::<godot::classes::Material>()]);
    apply_pre_generate_material_overrides(&mut state);

    let after = state.get_materials();
    let m = after.at(0);
    assert!(
        m.clone().try_cast::<BaseMaterial3D>().is_err(),
        "ShaderMaterial must not be replaced by a BaseMaterial3D"
    );
    assert!(
        m.to_variant() == pre,
        "ShaderMaterial instance identity must be preserved"
    );
}

#[itest]
fn apply_pre_generate_material_overrides_is_idempotent() {
    let mat = fresh_standard_material();
    mat.clone()
        .upcast::<BaseMaterial3D>()
        .set_shading_mode(ShadingMode::PER_VERTEX);

    let mut state = make_state_with_materials(vec![mat.upcast::<godot::classes::Material>()]);
    apply_pre_generate_material_overrides(&mut state);
    apply_pre_generate_material_overrides(&mut state);

    let after = state.get_materials();
    let base = after.at(0).try_cast::<BaseMaterial3D>().unwrap();
    assert_eq!(base.get_shading_mode(), ShadingMode::PER_VERTEX);
}
