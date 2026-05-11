// Integration tests for `apply_pre_generate_mesh_simplification`.
//
// Each `#[godot::test::itest]` is collected by `crate::framework` and runs
// via `cargo run -- run --itest`. These tests exercise the LOD-chain
// pre-generate hook against in-memory `ImporterMesh` fixtures wrapped in a
// `GltfState`, with no on-disk GLB roundtrip.

mod lod_chain_tests {
    use super::super::super::common::apply_pre_generate_mesh_simplification;
    use crate::framework::TestContext;
    use godot::classes::mesh::{ArrayFormat, ArrayType, PrimitiveType};
    use godot::classes::{GltfMesh, GltfState, ImporterMesh};
    use godot::prelude::*;

    /// Build a flat triangulated grid in the XZ plane with `rows * cols`
    /// vertices and `(rows-1)*(cols-1)*2` triangles. Returns a `VarArray`
    /// shaped like `surface_get_arrays` expects (length `ArrayType::MAX`).
    fn make_grid_arrays(rows: usize, cols: usize) -> VarArray {
        let mut verts = PackedVector3Array::new();
        let mut normals = PackedVector3Array::new();
        for r in 0..rows {
            for c in 0..cols {
                verts.push(Vector3::new(c as f32, 0.0, r as f32));
                normals.push(Vector3::new(0.0, 1.0, 0.0));
            }
        }
        let mut indices = PackedInt32Array::new();
        for r in 0..(rows - 1) {
            for c in 0..(cols - 1) {
                let i0 = (r * cols + c) as i32;
                let i1 = (r * cols + c + 1) as i32;
                let i2 = ((r + 1) * cols + c) as i32;
                let i3 = ((r + 1) * cols + c + 1) as i32;
                indices.push(i0);
                indices.push(i2);
                indices.push(i1);
                indices.push(i1);
                indices.push(i2);
                indices.push(i3);
            }
        }
        build_surface_arrays(verts, Some(normals), indices, None)
    }

    fn build_surface_arrays(
        verts: PackedVector3Array,
        normals: Option<PackedVector3Array>,
        indices: PackedInt32Array,
        bones: Option<PackedInt32Array>,
    ) -> VarArray {
        let mut arrays = VarArray::new();
        arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
        arrays.set(ArrayType::VERTEX.ord() as usize, &verts.to_variant());
        if let Some(n) = normals {
            arrays.set(ArrayType::NORMAL.ord() as usize, &n.to_variant());
        }
        if let Some(b) = bones {
            arrays.set(ArrayType::BONES.ord() as usize, &b.to_variant());
        }
        arrays.set(ArrayType::INDEX.ord() as usize, &indices.to_variant());
        arrays
    }

    fn state_with_importer(importer: Gd<ImporterMesh>) -> Gd<GltfState> {
        let mut gltf_mesh = GltfMesh::new_gd();
        gltf_mesh.set_mesh(&importer);
        let mut state = GltfState::new_gd();
        let meshes: Array<Gd<GltfMesh>> = Array::from(&[gltf_mesh]);
        state.set_meshes(&meshes);
        state
    }

    fn first_importer(state: &mut Gd<GltfState>) -> Gd<ImporterMesh> {
        state
            .get_meshes()
            .at(0)
            .get_mesh()
            .expect("GltfMesh should hold an ImporterMesh after pre-generate")
    }

    #[godot::test::itest]
    fn lod_chain_adds_three_lods_when_indices_above_min(_ctx: &TestContext) {
        // 11x11 grid → 100 quads → 200 triangles → 600 indices. Above
        // MIN_INDICES_FOR_LOD and decimatable.
        let arrays = make_grid_arrays(11, 11);
        let src_idx_len = arrays
            .at(ArrayType::INDEX.ord() as usize)
            .to::<PackedInt32Array>()
            .len() as f32;

        let mut importer = ImporterMesh::new_gd();
        importer
            .add_surface_ex(PrimitiveType::TRIANGLES, &arrays)
            .name("grid")
            .done();
        let mut state = state_with_importer(importer);

        apply_pre_generate_mesh_simplification(&mut state, 0.5);

        let importer = first_importer(&mut state);
        assert_eq!(
            importer.get_surface_lod_count(0),
            3,
            "expected three LOD levels"
        );

        let l0 = importer.get_surface_lod_indices(0, 0).len() as f32;
        let l1 = importer.get_surface_lod_indices(0, 1).len() as f32;
        let l2 = importer.get_surface_lod_indices(0, 2).len() as f32;
        // Allow generous slack — meshopt::simplify doesn't hit exact ratios.
        let in_range = |actual: f32, target: f32| {
            let lo = target * 0.5;
            let hi = target * 1.5;
            actual >= lo && actual <= hi
        };
        assert!(
            in_range(l0, src_idx_len * 0.5),
            "LOD1 index count {} not near 50% of {}",
            l0,
            src_idx_len
        );
        assert!(
            in_range(l1, src_idx_len * 0.25),
            "LOD2 index count {} not near 25% of {}",
            l1,
            src_idx_len
        );
        assert!(
            in_range(l2, src_idx_len * 0.1),
            "LOD3 index count {} not near 10% of {}",
            l2,
            src_idx_len
        );

        // SSE keys are increasing across LOD levels (further = larger error).
        let sse0 = importer.get_surface_lod_size(0, 0);
        let sse1 = importer.get_surface_lod_size(0, 1);
        let sse2 = importer.get_surface_lod_size(0, 2);
        assert!(
            sse0 < sse1 && sse1 < sse2,
            "SSE keys must be monotonically increasing"
        );
    }

    #[godot::test::itest]
    fn lod_chain_preserves_lod0_arrays_exactly(_ctx: &TestContext) {
        let arrays = make_grid_arrays(11, 11);
        let src_indices = arrays
            .at(ArrayType::INDEX.ord() as usize)
            .to::<PackedInt32Array>();
        let src_verts = arrays
            .at(ArrayType::VERTEX.ord() as usize)
            .to::<PackedVector3Array>();

        let mut importer = ImporterMesh::new_gd();
        importer
            .add_surface_ex(PrimitiveType::TRIANGLES, &arrays)
            .name("grid")
            .done();
        let mut state = state_with_importer(importer);

        apply_pre_generate_mesh_simplification(&mut state, 0.5);

        let importer = first_importer(&mut state);
        let out = importer.get_surface_arrays(0);
        let out_indices = out
            .at(ArrayType::INDEX.ord() as usize)
            .to::<PackedInt32Array>();
        let out_verts = out
            .at(ArrayType::VERTEX.ord() as usize)
            .to::<PackedVector3Array>();

        assert_eq!(
            out_indices.as_slice(),
            src_indices.as_slice(),
            "LOD0 indices must be byte-identical to source"
        );
        assert_eq!(
            out_verts.len(),
            src_verts.len(),
            "LOD0 vertex count must match source"
        );
        for (a, b) in out_verts.as_slice().iter().zip(src_verts.as_slice()) {
            assert_eq!(a, b, "LOD0 vertex must be byte-identical to source");
        }
    }

    #[godot::test::itest]
    fn lod_chain_skips_surfaces_below_min_indices(_ctx: &TestContext) {
        // 5x5 grid → 16 quads → 32 triangles → 96 indices. Below the
        // MIN_INDICES_FOR_LOD floor (100) used by the implementation.
        let arrays = make_grid_arrays(5, 5);
        let mut importer = ImporterMesh::new_gd();
        importer
            .add_surface_ex(PrimitiveType::TRIANGLES, &arrays)
            .name("tiny")
            .done();
        let mut state = state_with_importer(importer);

        apply_pre_generate_mesh_simplification(&mut state, 0.5);

        let importer = first_importer(&mut state);
        assert_eq!(
            importer.get_surface_lod_count(0),
            0,
            "tiny surface must not get LODs"
        );
    }

    #[godot::test::itest]
    fn lod_chain_skips_blend_shape_meshes(_ctx: &TestContext) {
        let arrays = make_grid_arrays(11, 11);
        let mut importer = ImporterMesh::new_gd();
        importer.add_blend_shape("morph_a");
        importer
            .add_surface_ex(PrimitiveType::TRIANGLES, &arrays)
            .name("blendy")
            .done();
        let mut state = state_with_importer(importer);

        apply_pre_generate_mesh_simplification(&mut state, 0.5);

        let importer = first_importer(&mut state);
        assert_eq!(
            importer.get_surface_lod_count(0),
            0,
            "blend-shape meshes must be left untouched"
        );
        assert_eq!(importer.get_blend_shape_count(), 1);
    }

    #[godot::test::itest]
    fn lod_chain_skips_skinned_surfaces(_ctx: &TestContext) {
        let mut verts = PackedVector3Array::new();
        let mut normals = PackedVector3Array::new();
        let rows = 11usize;
        let cols = 11usize;
        for r in 0..rows {
            for c in 0..cols {
                verts.push(Vector3::new(c as f32, 0.0, r as f32));
                normals.push(Vector3::new(0.0, 1.0, 0.0));
            }
        }
        let mut indices = PackedInt32Array::new();
        for r in 0..(rows - 1) {
            for c in 0..(cols - 1) {
                let i0 = (r * cols + c) as i32;
                let i1 = (r * cols + c + 1) as i32;
                let i2 = ((r + 1) * cols + c) as i32;
                let i3 = ((r + 1) * cols + c + 1) as i32;
                indices.push(i0);
                indices.push(i2);
                indices.push(i1);
                indices.push(i1);
                indices.push(i2);
                indices.push(i3);
            }
        }
        let mut bones = PackedInt32Array::new();
        for _ in 0..(verts.len() * 4) {
            bones.push(0);
        }
        let arrays = build_surface_arrays(verts, Some(normals), indices, Some(bones));

        let mut importer = ImporterMesh::new_gd();
        importer
            .add_surface_ex(PrimitiveType::TRIANGLES, &arrays)
            .name("skinned")
            .done();
        let mut state = state_with_importer(importer);

        apply_pre_generate_mesh_simplification(&mut state, 0.5);

        let importer = first_importer(&mut state);
        assert_eq!(
            importer.get_surface_lod_count(0),
            0,
            "skinned surfaces must be left untouched"
        );
    }

    #[godot::test::itest]
    fn lod_chain_preserves_per_surface_flags(_ctx: &TestContext) {
        let arrays = make_grid_arrays(11, 11);
        let flag = ArrayFormat::FLAG_USE_DYNAMIC_UPDATE.ord();
        let mut importer = ImporterMesh::new_gd();
        importer
            .add_surface_ex(PrimitiveType::TRIANGLES, &arrays)
            .name("flagged")
            .flags(flag)
            .done();
        let mut state = state_with_importer(importer);

        apply_pre_generate_mesh_simplification(&mut state, 0.5);

        let importer = first_importer(&mut state);
        let post = importer.get_surface_format(0);
        assert!(
            (post & flag) == flag,
            "FLAG_USE_DYNAMIC_UPDATE must survive the clear-and-readd dance (got {:b})",
            post
        );
    }

    #[godot::test::itest]
    fn lod_chain_handles_multi_surface_mesh(_ctx: &TestContext) {
        let big = make_grid_arrays(11, 11); // 600 indices → eligible
        let small = make_grid_arrays(5, 5); // 96 indices → skipped
        let medium = make_grid_arrays(9, 9); // 384 indices → eligible

        let mut importer = ImporterMesh::new_gd();
        importer
            .add_surface_ex(PrimitiveType::TRIANGLES, &big)
            .name("big")
            .done();
        importer
            .add_surface_ex(PrimitiveType::TRIANGLES, &small)
            .name("small")
            .done();
        importer
            .add_surface_ex(PrimitiveType::TRIANGLES, &medium)
            .name("medium")
            .done();
        let mut state = state_with_importer(importer);

        apply_pre_generate_mesh_simplification(&mut state, 0.5);

        let importer = first_importer(&mut state);
        assert_eq!(importer.get_surface_count(), 3);
        assert_eq!(
            importer.get_surface_lod_count(0),
            3,
            "big surface should get full LOD chain"
        );
        assert_eq!(
            importer.get_surface_lod_count(1),
            0,
            "small surface should be skipped"
        );
        assert_eq!(
            importer.get_surface_lod_count(2),
            3,
            "medium surface should get full LOD chain"
        );
        assert_eq!(
            importer.get_surface_name(0).to_string(),
            "big",
            "surface name must survive re-add"
        );
        assert_eq!(importer.get_surface_name(1).to_string(), "small");
        assert_eq!(importer.get_surface_name(2).to_string(), "medium");
    }
}
