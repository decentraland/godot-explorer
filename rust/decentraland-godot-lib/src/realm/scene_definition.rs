use std::{str::FromStr, sync::Arc};

use godot::builtin::{Vector2i, Vector3};

use crate::{
    content::content_mapping::{ContentMappingAndUrl, ContentMappingAndUrlRef},
    dcl::common::{content_entity::EntityDefinitionJson, scene::SceneEntityMetadata},
};
#[derive(Debug, Clone, PartialEq)]
pub struct EntityBase {
    pub hash: String,
    pub base_url: String,
}

// This struct wraps the `EntityDefinitionJson`:
// - It ensures the scene metadata
// - It holds the ContentMappingAndUrlRef

#[derive(Default, Debug)]
pub struct SceneEntityDefinition {
    pub id: String,
    pub is_global: bool,
    pub entity_definition_json: EntityDefinitionJson,
    pub scene_meta_scene: SceneEntityMetadata,
    pub content_mapping: ContentMappingAndUrlRef,
}

impl SceneEntityDefinition {
    pub fn from_json_ex(
        id: Option<String>,
        base_url: String,
        is_global: bool,
        json: serde_json::Value,
    ) -> Result<SceneEntityDefinition, anyhow::Error> {
        let mut entity_definition_json = serde_json::from_value::<EntityDefinitionJson>(json)?;
        let id = id.unwrap_or_else(|| entity_definition_json.id.take().unwrap_or_default());
        let metadata = entity_definition_json
            .metadata
            .take()
            .ok_or(anyhow::Error::msg("missing entity metadata"))?;
        let scene_meta_scene = serde_json::from_value::<SceneEntityMetadata>(metadata)?;

        let content_mapping_vec = std::mem::take(&mut entity_definition_json.content);
        let content_mapping = Arc::new(ContentMappingAndUrl::from_base_url_and_content(
            base_url,
            content_mapping_vec,
        ));

        Ok(SceneEntityDefinition {
            id,
            is_global,
            entity_definition_json,
            scene_meta_scene,
            content_mapping,
        })
    }

    pub fn get_title(&self) -> String {
        if let Some(scene_display) = self
            .scene_meta_scene
            .display
            .as_ref()
            .and_then(|d| d.title.as_ref())
        {
            scene_display.to_string()
        } else {
            self.id.clone()
        }
    }

    pub fn get_base_parcel(&self) -> Vector2i {
        self.scene_meta_scene.scene.base
    }

    pub fn get_parcels(&self) -> &Vec<Vector2i> {
        &self.scene_meta_scene.scene.parcels
    }

    pub fn get_godot_3d_position(&self) -> Vector3 {
        Vector3::new(
            16.0 * self.scene_meta_scene.scene.base.x as f32,
            0.0,
            -16.0 * self.scene_meta_scene.scene.base.y as f32,
        )
    }

    pub fn get_global_spawn_position(&self) -> Vector3 {
        let bounding_box = if let Some(spawn_points) = self.scene_meta_scene.spawn_points.as_ref() {
            // find the spawnpoint with default=true
            if let Some(spawn_point) = spawn_points.iter().find(|sp| sp.default) {
                spawn_point.position.bounding_box()
            } else if let Some(spawn_point) = spawn_points.first() {
                spawn_point.position.bounding_box()
            } else {
                (Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 0.0))
            }
        } else {
            (Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 0.0))
        };

        self.get_godot_3d_position()
            + Vector3::new(
                godot::engine::utilities::randf_range(
                    bounding_box.0.x as f64,
                    bounding_box.1.x as f64,
                ) as f32,
                godot::engine::utilities::randf_range(
                    bounding_box.0.y as f64,
                    bounding_box.1.y as f64,
                ) as f32,
                -godot::engine::utilities::randf_range(
                    bounding_box.0.z as f64,
                    bounding_box.1.z as f64,
                ) as f32,
            )
    }
}

impl EntityBase {
    pub fn from_urn(urn_str: &str, default_base_url: &String) -> Option<Self> {
        let Ok(urn) = urn::Urn::from_str(urn_str) else {
            return None;
        };
        let Some((lhs, rhs)) = urn.nss().split_once(':') else {
            return None;
        };
        let hash = match lhs {
            "entity" => rhs.to_owned(),
            _ => return None,
        };

        let key_values = urn
            .q_component()
            .unwrap_or("")
            .split('&')
            .flat_map(|piece| piece.split_once('='))
            .flat_map(|(key, value)| match key {
                "baseUrl" => Some(value.to_string()),
                _ => None,
            })
            .collect::<Vec<String>>();

        Some(EntityBase {
            hash,
            base_url: if let Some(base_url) = key_values.first() {
                base_url.clone()
            } else {
                format!("{default_base_url}contents/")
            },
        })
    }
}
