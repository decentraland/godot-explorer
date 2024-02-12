use crate::{
    content::content_mapping::{
        ContentMappingAndUrl, ContentMappingAndUrlRef, DclContentMappingAndUrl,
    },
    dcl::common::{
        content_entity::EntityDefinitionJson,
        wearable::{WearableEntityMetadata, WearableRepresentation},
    },
};
use godot::{
    bind::{godot_api, GodotClass},
    builtin::{GString, PackedStringArray},
    obj::Gd,
};
use std::{collections::HashMap, sync::Arc};

pub struct WearableEntityDefinition {
    pub id: String,
    pub entity_definition_json: EntityDefinitionJson,
    pub wearable: WearableEntityMetadata,
    pub content_mapping: ContentMappingAndUrlRef,
}

impl WearableEntityDefinition {
    pub fn from_json_ex(
        base_url: String,
        json: serde_json::Value,
    ) -> Result<WearableEntityDefinition, anyhow::Error> {
        let mut entity_definition_json = serde_json::from_value::<EntityDefinitionJson>(json)?;
        let id = entity_definition_json
            .pointers
            .first()
            .ok_or(anyhow::Error::msg("missing id"))?;
        let metadata = entity_definition_json
            .metadata
            .take()
            .ok_or(anyhow::Error::msg("missing entity metadata"))?;
        let wearable = serde_json::from_value::<WearableEntityMetadata>(metadata)?;

        let content_mapping_vec = std::mem::take(&mut entity_definition_json.content);
        let content_mapping = Arc::new(ContentMappingAndUrl {
            base_url,
            content: HashMap::from_iter(
                content_mapping_vec
                    .into_iter()
                    .map(|item| (item.file.to_lowercase(), item.hash)),
            ),
        });

        Ok(WearableEntityDefinition {
            id: id.clone(),
            entity_definition_json,
            wearable,
            content_mapping,
        })
    }
}

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct DclWearableEntityDefinition {
    pub inner: Arc<WearableEntityDefinition>,
}

impl DclWearableEntityDefinition {
    pub fn from_gd(inner: Arc<WearableEntityDefinition>) -> Gd<Self> {
        Gd::from_init_fn(|_base| Self { inner })
    }

    pub fn get_representation(&self, body_shape_id: &String) -> Option<&WearableRepresentation> {
        self.inner
            .wearable
            .data
            .representations
            .iter()
            .find(|representation| {
                representation
                    .body_shapes
                    .iter()
                    .any(|shape| shape == body_shape_id)
            })
    }
}

#[godot_api]
impl DclWearableEntityDefinition {
    #[func]
    fn get_category(&self) -> GString {
        GString::from(self.inner.wearable.data.category.slot)
    }

    #[func]
    fn has_representation(&self, body_shape_id: String) -> bool {
        self.get_representation(&body_shape_id).is_some()
    }

    #[func]
    fn get_representation_main_file(&self, body_shape_id: String) -> GString {
        self.get_representation(&body_shape_id)
            .map(|representation| representation.main_file.clone())
            .unwrap_or_default()
            .into()
    }

    #[func]
    fn get_hides_list(&self, body_shape_id: String) -> PackedStringArray {
        let mut hides = vec![];
        let representation = self.get_representation(&body_shape_id);

        if let Some(override_hides) = representation.map(|v| &v.override_hides) {
            if override_hides.is_empty() {
                hides.extend(self.inner.wearable.data.hides.iter().cloned());
            } else {
                hides.extend(override_hides.iter().cloned());
            }
        } else {
            hides.extend(self.inner.wearable.data.hides.iter().cloned());
        }

        // we apply this rule to hide the hands by default if the wearable is an upper body or hides the upper body
        let is_or_hides_upper_body = hides.contains(&"upper_body".to_string())
            || self.get_category().to_string() == "upper_body";

        // the rule is ignored if the wearable contains the removal of this default rule (newer upper bodies since the release of hands)
        let removes_hand_default = self
            .inner
            .wearable
            .data
            .removes_default_hiding
            .as_ref()
            .map_or(false, |removes_default_hiding| {
                removes_default_hiding.contains(&"hands".to_string())
            });

        // why we do this? because old upper bodies contains the base hand mesh, and they might clip with the new handwear items
        if is_or_hides_upper_body && !removes_hand_default {
            hides.extend(vec!["hands".to_string()]);
        }

        if let Some(override_replaces) = representation.map(|v| &v.override_replaces) {
            if override_replaces.is_empty() {
                hides.extend(self.inner.wearable.data.replaces.iter().cloned());
            } else {
                hides.extend(override_replaces.iter().cloned());
            }
        } else {
            hides.extend(self.inner.wearable.data.replaces.iter().cloned());
        }

        // skin has implicit hides
        if self.get_category().to_string() == "skin" {
            hides.extend(vec![
                "head".to_string(),
                "hair".to_string(),
                "facial_hair".to_string(),
                "mouth".to_string(),
                "eyebrows".to_string(),
                "eyes".to_string(),
                "upper_body".to_string(),
                "lower_body".to_string(),
                "feet".to_string(),
            ]);
        }

        // Safeguard the wearable can not hide itself
        let index = hides
            .iter()
            .position(|v| v == &self.get_category().to_string());
        if let Some(index) = index {
            hides.remove(index);
        }

        // Remove duplicates
        hides.sort_unstable();
        hides.dedup();

        PackedStringArray::from_iter(hides.iter().map(GString::from))
    }

    #[func]
    fn get_content_mapping(&self) -> Gd<DclContentMappingAndUrl> {
        DclContentMappingAndUrl::from_ref(self.inner.content_mapping.clone())
    }

    #[func]
    fn get_id(&self) -> GString {
        self.inner.id.clone().into()
    }

    #[func]
    fn get_thumbnail(&self) -> GString {
        self.inner.wearable.thumbnail.clone().into()
    }

    #[func]
    fn get_rarity(&self) -> GString {
        self.inner
            .wearable
            .rarity
            .clone()
            .unwrap_or_default()
            .into()
    }

    #[func]
    fn get_display_name(&self) -> GString {
        GString::from(
            self.inner
                .wearable
                .i18n
                .first()
                .map_or(&self.inner.wearable.name, |i18n| &i18n.text),
        )
    }
}
