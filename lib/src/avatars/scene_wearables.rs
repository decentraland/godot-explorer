use std::{collections::HashMap, sync::Arc};

use crate::{
    avatars::item::ItemEntityDefinition,
    content::content_mapping::ContentMappingAndUrlRef,
    dcl::common::wearable::{
        BaseItemEntityMetadata, WearableCategory, WearableData, WearableRepresentation,
    },
};

const SCENE_WEARABLE_URN_PREFIX: &str = "urn:decentraland:off-chain:scene-wearable:";

/// Categories a scene GLB can fill: every model slot (texture slots and the body shape are not GLBs).
const GLB_CATEGORIES: [WearableCategory; 14] = [
    WearableCategory::FACIAL_HAIR,
    WearableCategory::HAIR,
    WearableCategory::UPPER_BODY,
    WearableCategory::LOWER_BODY,
    WearableCategory::FEET,
    WearableCategory::EARRING,
    WearableCategory::EYEWEAR,
    WearableCategory::HAT,
    WearableCategory::HELMET,
    WearableCategory::MASK,
    WearableCategory::TIARA,
    WearableCategory::TOP_HEAD,
    WearableCategory::SKIN,
    WearableCategory::HANDS_WEAR,
];

/// `<name>_<category>.glb`; the longest suffix wins, so `beard_facial_hair.glb` is not `hair`.
fn category_from_path(path: &str) -> Option<WearableCategory> {
    let stem = path.rsplit('/').next()?.strip_suffix(".glb")?;
    GLB_CATEGORIES
        .iter()
        .filter(|category| {
            stem.strip_suffix(category.slot)
                .is_some_and(|name| name.len() > 1 && name.ends_with('_'))
        })
        .max_by_key(|category| category.slot.len())
        .copied()
}

/// Scene GLB wearables registered by the scenes, looked up before the catalyst.
/// An entry lives while its scene is loaded.
#[derive(Default)]
pub struct SceneWearableRegistry {
    by_urn: HashMap<String, Arc<ItemEntityDefinition>>,
}

impl SceneWearableRegistry {
    pub fn get(&self, urn_lowercase: &str) -> Option<Arc<ItemEntityDefinition>> {
        self.by_urn.get(urn_lowercase).cloned()
    }

    pub fn forget_scene(&mut self, scene_entity_id: &str) {
        let prefix = scene_urn_prefix(scene_entity_id);
        self.by_urn.retain(|urn, _| !urn.starts_with(&prefix));
    }

    fn register(
        &mut self,
        urn: String,
        category: WearableCategory,
        path: &str,
        content_mapping: &ContentMappingAndUrlRef,
    ) {
        if self.by_urn.contains_key(&urn) {
            return;
        }
        let representation = |body_shape: &str| WearableRepresentation {
            body_shapes: vec![body_shape.to_string()],
            main_file: path.to_string(),
            override_replaces: vec![],
            override_hides: vec![],
            contents: vec![path.to_string()],
        };
        let item = BaseItemEntityMetadata {
            id: urn.clone(),
            name: Some(path.to_string()),
            wearable_data: Some(WearableData {
                tags: vec![],
                category,
                representations: vec![
                    representation("urn:decentraland:off-chain:base-avatars:BaseMale"),
                    representation("urn:decentraland:off-chain:base-avatars:BaseFemale"),
                ],
                replaces: vec![],
                hides: vec![],
                // A scene GLB hides nothing, not even the hands under an upper body.
                removes_default_hiding: Some(vec!["hands".to_string()]),
            }),
            ..Default::default()
        };
        let definition = ItemEntityDefinition {
            id: urn.clone(),
            item,
            content_mapping: content_mapping.clone(),
            ..Default::default()
        };
        self.by_urn.insert(urn, Arc::new(definition));
    }
}

fn scene_urn_prefix(scene_entity_id: &str) -> String {
    format!(
        "{SCENE_WEARABLE_URN_PREFIX}{}-",
        scene_entity_id.to_lowercase()
    )
}

pub struct ResolvedWearable {
    pub urn: String,
    /// Set for a scene GLB: its category is force-rendered, since a scene GLB is never hidden.
    pub scene_category: Option<&'static str>,
}

/// Resolves `AvatarShape.wearables` for a scene: URNs pass through and `.glb` paths of the
/// scene content become scene-wearable URNs. Entries that cannot be resolved are skipped.
pub fn resolve_scene_wearables(
    registry: &mut SceneWearableRegistry,
    scene_entity_id: &str,
    content_mapping: &ContentMappingAndUrlRef,
    items: &[String],
) -> Vec<ResolvedWearable> {
    let own_prefix = scene_urn_prefix(scene_entity_id);
    let mut resolved = Vec::with_capacity(items.len());
    for item in items {
        let lowercase = item.to_lowercase();
        if lowercase.ends_with(".glb") {
            let Some(category) = category_from_path(&lowercase) else {
                tracing::warn!("Scene wearable {item} skipped: not named <name>_<category>.glb");
                continue;
            };
            let Some(hash) = content_mapping.get_hash(item) else {
                tracing::warn!("Scene wearable {item} skipped: not in the scene content");
                continue;
            };
            let urn = format!("{own_prefix}{}", hash.to_lowercase());
            registry.register(urn.clone(), category, &lowercase, content_mapping);
            resolved.push(ResolvedWearable {
                urn,
                scene_category: Some(category.slot),
            });
        } else if lowercase.starts_with(SCENE_WEARABLE_URN_PREFIX) {
            let definition = lowercase
                .starts_with(&own_prefix)
                .then(|| registry.get(&lowercase))
                .flatten();
            let Some(definition) = definition else {
                tracing::warn!("Scene wearable {item} skipped: not registered by this scene");
                continue;
            };
            resolved.push(ResolvedWearable {
                scene_category: definition
                    .item
                    .wearable_data
                    .as_ref()
                    .map(|data| data.category.slot),
                urn: lowercase,
            });
        } else if lowercase.starts_with("urn:") {
            resolved.push(ResolvedWearable {
                urn: item.clone(),
                scene_category: None,
            });
        } else {
            tracing::warn!("Scene wearable {item} skipped: neither a URN nor a .glb path");
        }
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_uses_the_longest_suffix() {
        let slot = |path: &str| category_from_path(path).map(|c| c.slot);
        assert_eq!(slot("wearables/cap_hat.glb"), Some("hat"));
        assert_eq!(slot("beard_facial_hair.glb"), Some("facial_hair"));
        assert_eq!(slot("green_hoodie_f_upper_body.glb"), Some("upper_body"));
        assert_eq!(slot("gloves_hands_wear.glb"), Some("hands_wear"));
        assert_eq!(slot("crown_top_head.glb"), Some("top_head"));
        assert_eq!(slot("hat.glb"), None);
        assert_eq!(slot("blue_eyes.glb"), None);
        assert_eq!(slot("cap_hat.gltf"), None);
    }
}
