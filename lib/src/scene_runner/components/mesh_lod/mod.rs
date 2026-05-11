//! LOD baking helpers reused across the GLTF import pipeline.
//!
//! Only `lod_baker::bake_shadow_mesh` is wired up in this PR — it's the
//! fallback path for visible MIs that don't have a sibling collider to
//! pair with in `apply_shadow_mesh`. Sibling perf PRs in this stack add
//! the runtime LOD pass and the `bake_lods` chain.

pub mod lod_baker;
