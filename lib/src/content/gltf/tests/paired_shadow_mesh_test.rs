//! Fixture tests for `apply_shadow_mesh` — the per-Node3D-parent pass
//! that wires `ArrayMesh.shadow_mesh` from sibling `*collider*` MIs (with
//! a stride-decimated bake as fallback). Runs as `#[godot::test::itest]`
//! so it has access to a live Godot runtime for constructing `Node3D` /
//! `MeshInstance3D` / `ArrayMesh` instances.

use super::super::scene::apply_shadow_mesh;
use crate::framework::TestContext;
use godot::classes::geometry_instance_3d::ShadowCastingSetting;
use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{ArrayMesh, MeshInstance3D, Node3D};
use godot::prelude::*;

/// Build an indexed-triangle `ArrayMesh` with exactly `triangle_count`
/// triangles. Vertex positions are degenerate-but-valid (a flat strip)
/// — `apply_shadow_mesh` doesn't inspect them, only the index/vertex
/// stream sizes.
fn make_triangle_mesh(triangle_count: usize) -> Gd<ArrayMesh> {
    let vertex_count = triangle_count + 2;
    let mut verts = PackedVector3Array::new();
    for i in 0..vertex_count {
        verts.push(Vector3::new(i as f32, (i % 2) as f32, 0.0));
    }
    let mut indices = PackedInt32Array::new();
    for t in 0..triangle_count {
        indices.push(t as i32);
        indices.push((t + 1) as i32);
        indices.push((t + 2) as i32);
    }
    let mut arrays = VarArray::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(ArrayType::VERTEX.ord() as usize, &verts.to_variant());
    arrays.set(ArrayType::INDEX.ord() as usize, &indices.to_variant());

    let mut mesh = ArrayMesh::new_gd();
    mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
    mesh
}

/// Same as `make_triangle_mesh`, plus one named blend shape so the mesh
/// gets skipped by the morph-target guard in `apply_shadow_mesh`.
fn make_triangle_mesh_with_blend_shape(triangle_count: usize) -> Gd<ArrayMesh> {
    let mut mesh = make_triangle_mesh(triangle_count);
    mesh.add_blend_shape("morph");
    mesh
}

fn make_mi(name: &str, mesh: &Gd<ArrayMesh>) -> Gd<MeshInstance3D> {
    let mut mi = MeshInstance3D::new_alloc();
    mi.set_name(name);
    mi.set_mesh(&mesh.clone().upcast::<godot::classes::Mesh>());
    mi
}

fn make_parent(name: &str) -> Gd<Node3D> {
    let mut n = Node3D::new_alloc();
    n.set_name(name);
    n
}

fn total_index_count(mesh: &Gd<ArrayMesh>) -> usize {
    let mut total = 0usize;
    for s in 0..mesh.get_surface_count() {
        let arrays = mesh.surface_get_arrays(s);
        if let Ok(idx) = arrays
            .at(ArrayType::INDEX.ord() as usize)
            .try_to::<PackedInt32Array>()
        {
            total += idx.len();
        }
    }
    total
}

#[godot::test::itest]
fn pair_visible_with_sibling_collider(_ctx: &TestContext) {
    let mut root = make_parent("root");
    let mut parent = make_parent("parent");

    let visible_mesh = make_triangle_mesh(64);
    let collider_mesh = make_triangle_mesh(8);
    let visible_mi = make_mi("visible_mi", &visible_mesh);
    let collider_mi = make_mi("wall_collider", &collider_mesh);

    parent.add_child(&visible_mi.clone().upcast::<godot::classes::Node>());
    parent.add_child(&collider_mi.clone().upcast::<godot::classes::Node>());
    root.add_child(&parent.clone().upcast::<godot::classes::Node>());

    let (paired, fallback) = apply_shadow_mesh(&root);
    assert_eq!((paired, fallback), (1, 0));

    let am = visible_mi
        .get_mesh()
        .and_then(|m| m.try_cast::<ArrayMesh>().ok())
        .expect("visible mi keeps ArrayMesh");
    let shadow = am.get_shadow_mesh().expect("shadow_mesh assigned");
    assert_eq!(shadow.instance_id(), collider_mesh.instance_id());

    root.queue_free();
}

#[godot::test::itest]
fn fallback_when_no_collider_sibling(_ctx: &TestContext) {
    let mut root = make_parent("root");
    let mut parent = make_parent("parent");

    let visible_mesh = make_triangle_mesh(200);
    let visible_mi = make_mi("visible_mi", &visible_mesh);
    parent.add_child(&visible_mi.clone().upcast::<godot::classes::Node>());
    root.add_child(&parent.clone().upcast::<godot::classes::Node>());

    let source_indices = total_index_count(&visible_mesh);
    assert_eq!(source_indices, 600);

    let (paired, fallback) = apply_shadow_mesh(&root);
    assert_eq!((paired, fallback), (0, 1));

    let am = visible_mi
        .get_mesh()
        .and_then(|m| m.try_cast::<ArrayMesh>().ok())
        .expect("visible mi keeps ArrayMesh");
    let shadow = am.get_shadow_mesh().expect("shadow_mesh baked");
    let shadow_am = shadow
        .try_cast::<ArrayMesh>()
        .expect("shadow_mesh is ArrayMesh");
    let kept = total_index_count(&shadow_am);
    // stride=4: keep 1 of every 4 triangles. 200 tris → 50 tris → 150 indices.
    assert!(
        (140..=160).contains(&kept),
        "expected ~150 indices, got {kept}"
    );

    root.queue_free();
}

#[godot::test::itest]
fn skip_blend_shape_meshes(_ctx: &TestContext) {
    let mut root = make_parent("root");
    let mut parent = make_parent("parent");

    let visible_mesh = make_triangle_mesh_with_blend_shape(64);
    let visible_mi = make_mi("visible_mi", &visible_mesh);
    parent.add_child(&visible_mi.clone().upcast::<godot::classes::Node>());
    root.add_child(&parent.clone().upcast::<godot::classes::Node>());

    let (paired, fallback) = apply_shadow_mesh(&root);
    assert_eq!((paired, fallback), (0, 0));

    let am = visible_mi
        .get_mesh()
        .and_then(|m| m.try_cast::<ArrayMesh>().ok())
        .expect("visible mi keeps ArrayMesh");
    assert!(am.get_shadow_mesh().is_none(), "shadow_mesh must stay null");

    root.queue_free();
}

#[godot::test::itest]
fn visible_mi_keeps_cast_shadow_on(_ctx: &TestContext) {
    let mut root = make_parent("root");
    let mut parent = make_parent("parent");

    let visible_mesh = make_triangle_mesh(64);
    let collider_mesh = make_triangle_mesh(8);
    let mut visible_mi = make_mi("visible_mi", &visible_mesh);
    visible_mi.set_cast_shadows_setting(ShadowCastingSetting::OFF);
    let collider_mi = make_mi("wall_collider", &collider_mesh);

    parent.add_child(&visible_mi.clone().upcast::<godot::classes::Node>());
    parent.add_child(&collider_mi.clone().upcast::<godot::classes::Node>());
    root.add_child(&parent.clone().upcast::<godot::classes::Node>());

    let _ = apply_shadow_mesh(&root);
    assert_eq!(
        visible_mi.get_cast_shadows_setting(),
        ShadowCastingSetting::ON
    );

    root.queue_free();
}

#[godot::test::itest]
fn multiple_visibles_share_one_collider_mesh(_ctx: &TestContext) {
    let mut root = make_parent("root");
    let mut parent = make_parent("parent");

    let mesh_a = make_triangle_mesh(20);
    let mesh_b = make_triangle_mesh(30);
    let collider_mesh = make_triangle_mesh(6);
    let visible_a = make_mi("visible_a", &mesh_a);
    let visible_b = make_mi("visible_b", &mesh_b);
    let collider = make_mi("single_collider", &collider_mesh);

    parent.add_child(&visible_a.clone().upcast::<godot::classes::Node>());
    parent.add_child(&visible_b.clone().upcast::<godot::classes::Node>());
    parent.add_child(&collider.clone().upcast::<godot::classes::Node>());
    root.add_child(&parent.clone().upcast::<godot::classes::Node>());

    let (paired, fallback) = apply_shadow_mesh(&root);
    assert_eq!((paired, fallback), (2, 0));

    for mi in [&visible_a, &visible_b] {
        let am = mi
            .get_mesh()
            .and_then(|m| m.try_cast::<ArrayMesh>().ok())
            .expect("mesh");
        let sm = am.get_shadow_mesh().expect("shadow assigned");
        assert_eq!(sm.instance_id(), collider_mesh.instance_id());
    }

    root.queue_free();
}

#[godot::test::itest]
fn apply_shadow_mesh_skips_collider_mis_themselves(_ctx: &TestContext) {
    let mut root = make_parent("root");
    let mut parent = make_parent("parent");

    let collider_mesh = make_triangle_mesh(8);
    let collider_mi = make_mi("wall_collider", &collider_mesh);
    parent.add_child(&collider_mi.clone().upcast::<godot::classes::Node>());
    root.add_child(&parent.clone().upcast::<godot::classes::Node>());

    let (paired, fallback) = apply_shadow_mesh(&root);
    assert_eq!((paired, fallback), (0, 0));

    let am = collider_mi
        .get_mesh()
        .and_then(|m| m.try_cast::<ArrayMesh>().ok())
        .expect("collider mesh");
    assert!(am.get_shadow_mesh().is_none());

    root.queue_free();
}

#[godot::test::itest]
fn apply_shadow_mesh_recurses_into_descendants(_ctx: &TestContext) {
    let mut root = make_parent("root");
    let mut group_a = make_parent("a");
    let mut group_b = make_parent("b");

    let v_a = make_mi("visible_a", &make_triangle_mesh(40));
    let c_a = make_mi("collider_a", &make_triangle_mesh(8));
    let v_b = make_mi("visible_b", &make_triangle_mesh(40));
    let c_b = make_mi("collider_b", &make_triangle_mesh(8));

    group_a.add_child(&v_a.clone().upcast::<godot::classes::Node>());
    group_a.add_child(&c_a.clone().upcast::<godot::classes::Node>());
    group_b.add_child(&v_b.clone().upcast::<godot::classes::Node>());
    group_b.add_child(&c_b.clone().upcast::<godot::classes::Node>());
    root.add_child(&group_a.clone().upcast::<godot::classes::Node>());
    root.add_child(&group_b.clone().upcast::<godot::classes::Node>());

    let (paired, fallback) = apply_shadow_mesh(&root);
    assert_eq!((paired, fallback), (2, 0));

    for mi in [&v_a, &v_b] {
        let am = mi
            .get_mesh()
            .and_then(|m| m.try_cast::<ArrayMesh>().ok())
            .expect("mesh");
        assert!(am.get_shadow_mesh().is_some());
    }

    root.queue_free();
}
