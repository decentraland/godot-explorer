use crate::{
    content::content_mapping::{
        ContentMappingAndUrl, ContentMappingAndUrlRef, DclContentMappingAndUrl,
    },
    dcl::common::{
        content_entity::EntityDefinitionJson,
        string::FindNthChar,
        wearable::{
            BaseItemEntityMetadata, EmoteADR74Representation, WearableCategory,
            WearableRepresentation,
        },
    },
};
use godot::{
    bind::{godot_api, GodotClass},
    builtin::{GString, PackedStringArray},
    obj::Gd,
};
use std::sync::Arc;

pub struct ItemEntityDefinition {
    pub id: String,
    pub entity_definition_json: EntityDefinitionJson,
    pub item: BaseItemEntityMetadata,
    pub content_mapping: ContentMappingAndUrlRef,
}

impl ItemEntityDefinition {
    pub fn from_json_ex(
        base_url: String,
        json: serde_json::Value,
    ) -> Result<ItemEntityDefinition, anyhow::Error> {
        let mut entity_definition_json = serde_json::from_value::<EntityDefinitionJson>(json)?;
        let id = entity_definition_json
            .pointers
            .first()
            .ok_or(anyhow::Error::msg("missing id"))?;
        let metadata = entity_definition_json
            .metadata
            .take()
            .ok_or(anyhow::Error::msg("missing entity metadata"))?;
        let item = serde_json::from_value::<BaseItemEntityMetadata>(metadata)?;

        let content_mapping_vec = std::mem::take(&mut entity_definition_json.content);
        let content_mapping = Arc::new(ContentMappingAndUrl::from_base_url_and_content(
            base_url,
            content_mapping_vec,
        ));

        Ok(ItemEntityDefinition {
            id: id.clone(),
            entity_definition_json,
            item,
            content_mapping,
        })
    }
}

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct DclItemEntityDefinition {
    pub inner: Arc<ItemEntityDefinition>,
}

impl DclItemEntityDefinition {
    pub fn from_gd(inner: Arc<ItemEntityDefinition>) -> Gd<Self> {
        Gd::from_init_fn(|_base| Self { inner })
    }

    pub fn get_wearable_representation(
        &self,
        body_shape_id: &String,
    ) -> Option<&WearableRepresentation> {
        self.inner
            .item
            .wearable_data
            .as_ref()?
            .representations
            .iter()
            .find(|representation| {
                representation
                    .body_shapes
                    .iter()
                    .any(|shape| shape == body_shape_id)
            })
    }

    pub fn get_emote_representation(
        &self,
        body_shape_id: &String,
    ) -> Option<&EmoteADR74Representation> {
        self.inner
            .item
            .emote_data
            .as_ref()?
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
impl DclItemEntityDefinition {
    #[func]
    fn get_emote_audio(&self, body_shape_id: String) -> GString {
        let Some(representation) = self.get_emote_representation(&body_shape_id) else {
            return GString::from("");
        };

        representation
            .contents
            .iter()
            .find(|file_name| file_name.ends_with(".mp3") || file_name.ends_with(".ogg"))
            .map(GString::from)
            .unwrap_or_default()
    }

    #[func]
    fn get_emote_loop(&self) -> bool {
        let Some(emote_data) = self.inner.item.emote_data.as_ref() else {
            return false;
        };

        emote_data.r#loop
    }

    /// Returns a 12 character long prefix for the emote id, removing the tokenId if it presents
    #[func]
    fn get_emote_prefix_id(&self) -> GString {
        let id = self.inner.id.to_string();
        let token_id_pos = id.find_nth_char(6, ':').unwrap_or(id.len());
        let id = id[0..token_id_pos].to_lowercase();

        let result = id.replace(':', "");
        let pos = (result.len() - 12).max(0);

        GString::from(&result[pos..])
    }

    #[func]
    fn is_wearable(&self) -> bool {
        self.inner.item.wearable_data.is_some()
    }

    #[func]
    fn is_emote(&self) -> bool {
        self.inner.item.emote_data.is_some()
    }

    #[func]
    fn get_category(&self) -> GString {
        if let Some(wearable_data) = &self.inner.item.wearable_data {
            GString::from(wearable_data.category.slot)
        } else if let Some(emote_data) = &self.inner.item.emote_data {
            GString::from(&emote_data.category)
        } else {
            GString::from("unknown")
        }
    }

    #[func]
    fn has_representation(&self, body_shape_id: String) -> bool {
        if self.inner.item.wearable_data.is_some() {
            self.get_wearable_representation(&body_shape_id).is_some()
        } else if self.inner.item.emote_data.is_some() {
            self.get_emote_representation(&body_shape_id).is_some()
        } else {
            false
        }
    }

    #[func]
    fn get_representation_main_file(&self, body_shape_id: String) -> GString {
        if self.inner.item.wearable_data.is_some() {
            self.get_wearable_representation(&body_shape_id)
                .map(|representation| representation.main_file.clone())
                .unwrap_or_default()
                .into()
        } else if self.inner.item.emote_data.is_some() {
            self.get_emote_representation(&body_shape_id)
                .map(|representation| representation.main_file.clone())
                .unwrap_or_default()
                .into()
        } else {
            GString::from("")
        }
    }

    /// Only for wearables
    #[func]
    fn get_hides_list(&self, body_shape_id: String) -> PackedStringArray {
        // Ensure it's a wearable
        let Some(wearable_data) = &self.inner.item.wearable_data else {
            return PackedStringArray::new();
        };

        let representation = self.get_wearable_representation(&body_shape_id);
        let mut hides = vec![];

        if let Some(override_hides) = representation.map(|v| &v.override_hides) {
            if override_hides.is_empty() {
                hides.extend(wearable_data.hides.iter().cloned());
            } else {
                hides.extend(override_hides.iter().cloned());
            }
        } else {
            hides.extend(wearable_data.hides.iter().cloned());
        }

        // we apply this rule to hide the hands by default if the wearable is an upper body or hides the upper body
        let is_or_hides_upper_body = hides.contains(&"upper_body".to_string())
            || self.get_category().to_string() == "upper_body";

        // the rule is ignored if the wearable contains the removal of this default rule (newer upper bodies since the release of hands)
        let removes_hand_default = wearable_data
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
                hides.extend(wearable_data.replaces.iter().cloned());
            } else {
                hides.extend(override_replaces.iter().cloned());
            }
        } else {
            hides.extend(wearable_data.replaces.iter().cloned());
        }

        // skin has implicit hides
        if self.get_category().to_string() == "skin" {
            hides.extend(vec![
                WearableCategory::HEAD.slot.to_string(),
                WearableCategory::HAIR.slot.to_string(),
                WearableCategory::FACIAL_HAIR.slot.to_string(),
                WearableCategory::MOUTH.slot.to_string(),
                WearableCategory::EYEBROWS.slot.to_string(),
                WearableCategory::EYES.slot.to_string(),
                WearableCategory::UPPER_BODY.slot.to_string(),
                WearableCategory::LOWER_BODY.slot.to_string(),
                WearableCategory::FEET.slot.to_string(),
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
        self.inner.item.thumbnail.clone().into()
    }

    #[func]
    fn get_rarity(&self) -> GString {
        self.inner.item.rarity.clone().unwrap_or_default().into()
    }

    #[func]
    fn get_display_name(&self) -> GString {
        GString::from(
            self.inner
                .item
                .i18n
                .first()
                .map_or(&self.inner.item.name, |i18n| &i18n.text),
        )
    }
}
