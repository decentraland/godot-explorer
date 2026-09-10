//! Offline scan of a scene's `main.crdt` for the assets its static composition
//! references, so the bake can ship them in one `{entity}-static.zip`.
//!
//! The scan decodes the file exactly like the runtime does
//! (`crate::dcl::js` feeds the same bytes to `process_many_messages`), so a
//! GLB the static composition would request on the phone is a GLB found here.

use std::collections::{HashMap, HashSet};
use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::dcl::{
    components::proto_components::{
        common::texture_union::Tex,
        common::TextureUnion,
        sdk::components::{pb_material, PbGltfContainer, PbMaterial},
    },
    crdt::{message::process_many_messages, SceneCrdtState, SceneCrdtStateProtoComponents},
    serialization::reader::DclReader,
};

use crate::content::content_provider::{optimized_cache_name, OptimizedKind};

use super::types::SceneOptimizationMetadata;

/// Content hashes referenced by a scene's `main.crdt`, in entity order.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StaticAssets {
    /// GLB hashes of every live `GltfContainer`
    pub gltfs: Vec<String>,
    /// Texture hashes of every live `Material` (`Tex::Texture` sources only)
    pub textures: Vec<String>,
}

impl StaticAssets {
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.gltfs.is_empty() && self.textures.is_empty()
    }
}

/// Decode `main.crdt` and collect the content hashes of the GLBs and material
/// textures the final composed state references.
///
/// `content_mapping` is the entity's lowercased `file path -> hash` map; a
/// `src` is resolved with the same normalization as
/// `ContentMappingAndUrl::get_hash` (lowercase, exact), so paths the runtime
/// would fail to resolve are dropped here too. Never panics: the CRDT reader
/// aborts on truncated input, and a broken file must not take the batch down —
/// it just yields no static bundle.
pub fn scan_main_crdt(bytes: &[u8], content_mapping: &HashMap<String, String>) -> StaticAssets {
    match catch_unwind(AssertUnwindSafe(|| scan_inner(bytes, content_mapping))) {
        Ok(assets) => assets,
        Err(_) => {
            tracing::warn!("main.crdt scan panicked (truncated or corrupt file); no static bundle");
            StaticAssets::default()
        }
    }
}

fn scan_inner(bytes: &[u8], content_mapping: &HashMap<String, String>) -> StaticAssets {
    let mut state = SceneCrdtState::from_proto();
    process_many_messages(&mut DclReader::new(bytes), &mut state);
    // Drops the components of entities deleted later in the file.
    state.take_dirty();

    let mut gltfs = Vec::new();
    let mut seen_gltf: HashSet<String> = HashSet::new();
    let mut textures = Vec::new();
    let mut seen_tex: HashSet<String> = HashSet::new();

    let resolve = |src: &str| -> Option<String> {
        if src.starts_with("http://") || src.starts_with("https://") {
            return None;
        }
        content_mapping.get(&src.to_lowercase()).cloned()
    };

    // `values` is a HashMap: sort by entity so the bundle is reproducible.
    let containers = SceneCrdtStateProtoComponents::get_gltf_container(&state);
    let mut entries: Vec<(&_, &PbGltfContainer)> = containers
        .values
        .iter()
        .filter(|(entity, _)| !state.entities.is_dead(entity))
        .filter_map(|(entity, entry)| entry.value.as_ref().map(|v| (entity, v)))
        .collect();
    entries.sort_by_key(|(entity, _)| (entity.number, entity.version));
    for (_, container) in entries {
        if let Some(hash) = resolve(&container.src) {
            if seen_gltf.insert(hash.clone()) {
                gltfs.push(hash);
            }
        }
    }

    let materials = SceneCrdtStateProtoComponents::get_material(&state);
    let mut entries: Vec<(&_, &PbMaterial)> = materials
        .values
        .iter()
        .filter(|(entity, _)| !state.entities.is_dead(entity))
        .filter_map(|(entity, entry)| entry.value.as_ref().map(|v| (entity, v)))
        .collect();
    entries.sort_by_key(|(entity, _)| (entity.number, entity.version));
    for (_, material) in entries {
        for union in material_textures(material) {
            let Some(Tex::Texture(texture)) = &union.tex else {
                continue;
            };
            if let Some(hash) = resolve(&texture.src) {
                if seen_tex.insert(hash.clone()) {
                    textures.push(hash);
                }
            }
        }
    }

    StaticAssets { gltfs, textures }
}

fn material_textures(material: &PbMaterial) -> Vec<&TextureUnion> {
    match &material.material {
        Some(pb_material::Material::Pbr(pbr)) => [
            pbr.texture.as_ref(),
            pbr.alpha_texture.as_ref(),
            pbr.emissive_texture.as_ref(),
            pbr.bump_texture.as_ref(),
        ]
        .into_iter()
        .flatten()
        .collect(),
        Some(pb_material::Material::Unlit(unlit)) => {
            [unlit.texture.as_ref(), unlit.alpha_texture.as_ref()]
                .into_iter()
                .flatten()
                .collect()
        }
        None => Vec::new(),
    }
}

/// Zip entries for the static bundle: `(entry name, content hash)`, entry
/// names being the client's cache names (`{hash}.opt.scn` / `{hash}.opt.res`).
/// Each GLB brings the textures its `.scn` references externally; anything
/// whose bake did not complete (not in `optimized_content`) is left out so the
/// bundle never carries a `.scn` with a dangling ExtResource.
pub fn static_bundle_entries(
    assets: &StaticAssets,
    metadata: &SceneOptimizationMetadata,
) -> Vec<(String, String)> {
    let optimized: HashSet<&String> = metadata.optimized_content.iter().collect();
    let mut entries = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut push = |hash: &String, kind: OptimizedKind, entries: &mut Vec<(String, String)>| {
        let name = optimized_cache_name(hash, kind);
        if seen.insert(name.clone()) {
            entries.push((name, hash.clone()));
        }
    };

    for glb in &assets.gltfs {
        if !optimized.contains(glb) {
            continue;
        }
        push(glb, OptimizedKind::Scene, &mut entries);
        if let Some(deps) = metadata.external_scene_dependencies.get(glb) {
            for tex in deps {
                if optimized.contains(tex) {
                    push(tex, OptimizedKind::Texture, &mut entries);
                }
            }
        }
    }
    for tex in &assets.textures {
        if optimized.contains(tex) {
            push(tex, OptimizedKind::Texture, &mut entries);
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dcl::{
        components::{
            proto_components::{
                common::{texture_union, Texture, TextureUnion, VideoTexture},
                sdk::components::{pb_material, PbGltfContainer, PbMaterial},
            },
            SceneComponentId, SceneCrdtTimestamp, SceneEntityId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation,
            message::{delete_entity, put_or_delete_lww_component},
        },
        serialization::writer::DclWriter,
    };

    fn gltf(src: &str) -> PbGltfContainer {
        PbGltfContainer {
            src: src.to_string(),
            ..Default::default()
        }
    }

    fn tex(src: &str) -> TextureUnion {
        TextureUnion {
            tex: Some(texture_union::Tex::Texture(Texture {
                src: src.to_string(),
                ..Default::default()
            })),
        }
    }

    /// Build a `main.crdt` byte stream from a composed state, then delete `killed`.
    fn encode(
        state: &SceneCrdtState,
        entities: &[SceneEntityId],
        killed: &[SceneEntityId],
    ) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut writer = DclWriter::new(&mut buf);
        for entity in entities {
            for component in [SceneComponentId::GLTF_CONTAINER, SceneComponentId::MATERIAL] {
                let _ = put_or_delete_lww_component(state, entity, &component, &mut writer);
            }
        }
        for entity in killed {
            delete_entity(entity, &mut writer);
        }
        buf
    }

    fn mapping(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_lowercase(), v.to_string()))
            .collect()
    }

    #[test]
    fn scan_collects_live_gltfs_and_textures_in_entity_order() {
        let mut state = SceneCrdtState::from_proto();
        let e1 = SceneEntityId::new(512, 0);
        let e2 = SceneEntityId::new(513, 0);
        let e3 = SceneEntityId::new(514, 0);
        let e4 = SceneEntityId::new(515, 0);
        let ts = SceneCrdtTimestamp(1);
        let containers = SceneCrdtStateProtoComponents::get_gltf_container_mut(&mut state);
        containers.set(e3, ts, Some(gltf("Models/Tree.glb"))); // mixed case, later entity
        containers.set(e1, ts, Some(gltf("models/house.glb")));
        containers.set(e2, ts, Some(gltf("models/tree.glb"))); // duplicate of e3
        containers.set(e4, ts, Some(gltf("models/missing.glb"))); // not in mapping
        let materials = SceneCrdtStateProtoComponents::get_material_mut(&mut state);
        materials.set(
            e1,
            ts,
            Some(PbMaterial {
                material: Some(pb_material::Material::Pbr(pb_material::PbrMaterial {
                    texture: Some(tex("images/wall.png")),
                    emissive_texture: Some(tex("https://example.com/remote.png")),
                    bump_texture: Some(TextureUnion {
                        tex: Some(texture_union::Tex::VideoTexture(VideoTexture::default())),
                    }),
                    ..Default::default()
                })),
            }),
        );
        let bytes = encode(&state, &[e1, e2, e3, e4], &[]);
        let map = mapping(&[
            ("models/house.glb", "hash-house"),
            ("models/tree.glb", "hash-tree"),
            ("images/wall.png", "hash-wall"),
        ]);

        let assets = scan_main_crdt(&bytes, &map);
        assert_eq!(assets.gltfs, vec!["hash-house", "hash-tree"]);
        assert_eq!(assets.textures, vec!["hash-wall"]);
    }

    #[test]
    fn scan_drops_deleted_entities() {
        let mut state = SceneCrdtState::from_proto();
        let e1 = SceneEntityId::new(512, 0);
        let e2 = SceneEntityId::new(513, 0);
        let ts = SceneCrdtTimestamp(1);
        let containers = SceneCrdtStateProtoComponents::get_gltf_container_mut(&mut state);
        containers.set(e1, ts, Some(gltf("a.glb")));
        containers.set(e2, ts, Some(gltf("b.glb")));
        let bytes = encode(&state, &[e1, e2], &[e2]);
        let map = mapping(&[("a.glb", "hash-a"), ("b.glb", "hash-b")]);

        let assets = scan_main_crdt(&bytes, &map);
        assert_eq!(assets.gltfs, vec!["hash-a"]);
    }

    #[test]
    fn scan_survives_truncated_input() {
        let mut state = SceneCrdtState::from_proto();
        let e1 = SceneEntityId::new(512, 0);
        SceneCrdtStateProtoComponents::get_gltf_container_mut(&mut state).set(
            e1,
            SceneCrdtTimestamp(1),
            Some(gltf("a.glb")),
        );
        let bytes = encode(&state, &[e1], &[]);
        let truncated = &bytes[..bytes.len() / 2];
        let assets = scan_main_crdt(truncated, &mapping(&[("a.glb", "hash-a")]));
        assert!(assets.is_empty());
    }

    #[test]
    fn bundle_entries_follow_dependencies_and_skip_failed_bakes() {
        let assets = StaticAssets {
            gltfs: vec!["glb-ok".into(), "glb-failed".into()],
            textures: vec!["tex-mat".into(), "tex-failed".into()],
        };
        let mut metadata = SceneOptimizationMetadata::default();
        metadata.optimized_content = vec![
            "glb-ok".into(),
            "tex-a".into(),
            "tex-b".into(),
            "tex-mat".into(),
        ];
        metadata.external_scene_dependencies.insert(
            "glb-ok".into(),
            vec!["tex-a".into(), "tex-b".into(), "tex-gone".into()],
        );

        let entries = static_bundle_entries(&assets, &metadata);
        let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "glb-ok.opt.scn",
                "tex-a.opt.res",
                "tex-b.opt.res",
                "tex-mat.opt.res"
            ]
        );
    }
}
