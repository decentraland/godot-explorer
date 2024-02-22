use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use godot::{prelude::*, test::itest};

use crate::http_request::{
    http_requester::HttpRequester,
    request_response::{RequestOption, RequestResponse, ResponseEnum, ResponseType},
};

use super::{
    dcl_scene_entity_definition::DclSceneEntityDefinition,
    parcel::*,
    scene_definition::{EntityBase, SceneEntityDefinition},
};

#[derive(Debug, Default, GodotClass)]
#[class(base=Node)]
struct SceneEntityCoordinator {
    parcel_radius_calculator: ParcelRadiusCalculator,

    current_position: Coord,
    should_load_city_scenes: bool,
    requested_city_pointers: HashMap<u32, HashSet<Coord>>,
    cache_city_pointers: HashMap<Coord, String>, // coord to entity_id

    global_desired_entities: Vec<EntityBase>,
    requested_entity: HashMap<u32, EntityBase>,
    cache_scene_data: HashMap<String, Arc<SceneEntityDefinition>>, // entity_id to SceneData

    http_requester: HttpRequester,
    entities_active_url: String,
    content_url: String,

    version: u32,
    dirty_loadable_scenes: bool,
    loadable_scenes: HashSet<String>,
    keep_alive_scenes: HashSet<String>,
    empty_parcels: HashSet<String>,
}

impl SceneEntityCoordinator {
    const REQUEST_TYPE_SCENE_DATA: u32 = 1;
    const REQUEST_TYPE_SCENE_POINTERS: u32 = 2;

    pub fn new(
        entities_active_url: String,
        content_url: String,
        should_load_city_scenes: bool,
    ) -> Self {
        let mut _self = SceneEntityCoordinator {
            parcel_radius_calculator: ParcelRadiusCalculator::new(3),
            ..Default::default()
        };

        _self._config(entities_active_url, content_url, should_load_city_scenes);
        _self
    }

    pub fn _config(
        &mut self,
        entities_active_url: String,
        content_url: String,
        should_load_city_scenes: bool,
    ) {
        self.entities_active_url = entities_active_url;
        self.content_url = content_url;
        self.current_position = Coord(-1000, -1000);
        self.should_load_city_scenes = should_load_city_scenes;
        self.global_desired_entities.clear();
        self.cache_city_pointers.clear();
        self.cache_scene_data.clear();
        self.requested_city_pointers.clear();
        self.requested_entity.clear();
        self.dirty_loadable_scenes = true;
    }

    fn request_pointers(&mut self, set_request_pointers: HashSet<Coord>) {
        // Request the new pointers
        if !set_request_pointers.is_empty() {
            let request_pointers_body = set_request_pointers
                .iter()
                .map(|coord| format!("\"{coord}\""))
                .collect::<Vec<_>>()
                .join(",");

            let request_body: String = format!("{{\"pointers\":[{request_pointers_body}]}}");

            let request = RequestOption::new(
                Self::REQUEST_TYPE_SCENE_POINTERS,
                self.entities_active_url.to_string(),
                http::Method::POST,
                ResponseType::AsJson,
                Some(request_body.as_bytes().to_vec()),
                Some(vec!["Content-Type: application/json".to_string()]),
                None,
            );
            self.requested_city_pointers
                .insert(request.id, set_request_pointers);
            self.http_requester.send_request(request);
        }
    }

    fn handle_scene_data(&mut self, id: u32, json: serde_json::Value) {
        let entity_base = if let Some(entity_base) = self.requested_entity.remove(&id) {
            entity_base
        } else {
            return;
        };

        let is_global_scene = self
            .global_desired_entities
            .iter()
            .any(|value| *value == entity_base);

        let entity_definition_json = match SceneEntityDefinition::from_json_ex(
            Some(entity_base.hash.clone()),
            entity_base.base_url.clone(),
            is_global_scene,
            json,
        ) {
            Ok(entity_definition) => entity_definition,
            Err(err) => {
                tracing::info!(
                    "Error handling scene data from entity {:?}: {:?}",
                    entity_base,
                    err
                );
                return;
            }
        };

        // If it's a global scene, it doesn't add the pointers to the cache
        if !is_global_scene {
            let entity_id = entity_definition_json.id.as_str();
            for pointer in entity_definition_json.scene_meta_scene.scene.parcels.iter() {
                let coord = Coord::from(pointer);
                self.cache_city_pointers
                    .insert(coord, entity_id.to_string());
            }
        }

        self.cache_scene_data
            .insert(entity_base.hash, Arc::new(entity_definition_json));
    }

    fn handle_entity_pointers(&mut self, request_id: u32, mut json: serde_json::Value) {
        // If the request was dismissed, early return (this typically happens when the realm is changed)
        let mut remaining_pointers =
            if let Some(remaining_pointers) = self.requested_city_pointers.remove(&request_id) {
                remaining_pointers
            } else {
                return;
            };

        let Some(entity_pointers) = json.as_array_mut() else {
            return;
        };

        // Add the scene data to the cache
        for entity_pointer in entity_pointers.iter_mut() {
            let base_url = format!("{}contents/", self.content_url);

            let entity_definition_json = match SceneEntityDefinition::from_json_ex(
                None,
                base_url,
                false,
                entity_pointer.take(),
            ) {
                Ok(entity_definition) => entity_definition,
                Err(err) => {
                    tracing::info!("Error handling pointer from entity {:?}", err);
                    continue;
                }
            };

            for pointer in entity_definition_json
                .entity_definition_json
                .pointers
                .iter()
            {
                let coord = Coord::from(pointer);

                remaining_pointers.remove(&coord);
                self.cache_city_pointers
                    .insert(coord, entity_definition_json.id.clone());
            }

            self.cache_scene_data.insert(
                entity_definition_json.id.clone(),
                Arc::new(entity_definition_json),
            );
        }

        for pointer in remaining_pointers.into_iter() {
            self.cache_city_pointers
                .insert(pointer, "empty".to_string());
        }
    }

    fn handle_response(&mut self, response: RequestResponse) {
        match response.response_data {
            Ok(response_data) => match response_data {
                ResponseEnum::Json(json) => {
                    if json.is_err() {
                        self.cleanup_request_id(response.request_option.id);
                        tracing::info!("Error parsing the JSON {json:?}");
                        return;
                    }

                    match response.request_option.reference_id {
                        Self::REQUEST_TYPE_SCENE_DATA => {
                            self.handle_scene_data(response.request_option.id, json.unwrap());
                        }
                        Self::REQUEST_TYPE_SCENE_POINTERS => {
                            self.handle_entity_pointers(response.request_option.id, json.unwrap());
                        }
                        _ => {
                            tracing::info!("Invalid type of request ID while handling a request");
                        }
                    }
                }
                _ => {
                    self.cleanup_request_id(response.request_option.id);
                    tracing::info!("Invalid type of request while handling a request");
                }
            },
            Err(err) => {
                self.cleanup_request_id(response.request_option.id);
                tracing::info!("Error while handling a request: {err:?}");
            }
        }
    }

    fn cleanup_request_id(&mut self, request_id: u32) {
        self.requested_city_pointers.remove(&request_id);
        self.requested_entity.remove(&request_id);
    }

    /// Returns the scenes that are desired to be loaded
    fn update_loadable_and_keep_alive_scenes(&mut self) {
        self.version += 1;
        self.loadable_scenes.clear();
        self.keep_alive_scenes.clear();
        self.empty_parcels.clear();

        let unexisting_taken_as_empty: bool = !self.should_load_city_scenes
            && self.requested_city_pointers.is_empty()
            && self.requested_entity.is_empty();

        // Check what are the new scenes to load that are not in the cache
        for coord in self.parcel_radius_calculator.get_inner_parcels() {
            let coord = coord.plus(&self.current_position);

            if let Some(entity_id) = self.cache_city_pointers.get(&coord) {
                if entity_id == "empty" {
                    self.empty_parcels.insert(coord.to_string());
                } else {
                    self.loadable_scenes.insert(entity_id.clone());
                }
            } else if unexisting_taken_as_empty {
                self.empty_parcels.insert(coord.to_string());
            }
        }

        for coord in self.parcel_radius_calculator.get_outer_parcels() {
            let coord = coord.plus(&self.current_position);

            if let Some(entity_id) = self.cache_city_pointers.get(&coord) {
                if entity_id == "empty" {
                    continue;
                }
                if self.loadable_scenes.contains(entity_id) {
                    continue;
                }
                self.keep_alive_scenes.insert(entity_id.clone());
            }
        }

        for entity_base in self.global_desired_entities.iter() {
            if self.cache_scene_data.contains_key(&entity_base.hash) {
                self.loadable_scenes.insert(entity_base.hash.clone());
            }
        }
    }

    pub fn _set_fixed_desired_entities_urns(&mut self, entities: Vec<String>) {
        if self.content_url.is_empty() {
            return;
        }

        self.dirty_loadable_scenes = true;

        for urn_str in entities.iter() {
            if self.cache_scene_data.contains_key(urn_str) {
                continue;
            }
            let Some(entity_base) = EntityBase::from_urn(urn_str, &self.content_url) else {
                continue;
            };

            let url = format!("{}{}", entity_base.base_url, entity_base.hash);
            let request = RequestOption::new(
                Self::REQUEST_TYPE_SCENE_DATA,
                url,
                http::Method::GET,
                ResponseType::AsJson,
                None,
                None,
                None,
            );

            self.requested_entity.insert(request.id, entity_base);
            self.http_requester.send_request(request);
        }
    }

    pub fn _set_fixed_desired_entities_global_urns(&mut self, entities: Vec<String>) {
        if self.content_url.is_empty() {
            return;
        }

        self.dirty_loadable_scenes = true;
        self.global_desired_entities.clear();

        for urn_str in entities.iter() {
            let Some(entity_base) = EntityBase::from_urn(urn_str, &self.content_url) else {
                continue;
            };
            if self.cache_scene_data.contains_key(urn_str) {
                self.global_desired_entities.push(entity_base);
                continue;
            }

            let url = format!("{}{}", entity_base.base_url, entity_base.hash);
            let request = RequestOption::new(
                Self::REQUEST_TYPE_SCENE_DATA,
                url,
                http::Method::GET,
                ResponseType::AsJson,
                None,
                None,
                None,
            );

            self.global_desired_entities.push(entity_base.clone());
            self.requested_entity.insert(request.id, entity_base);
            self.http_requester.send_request(request);
        }
    }

    pub fn update_position(&mut self, x: i16, z: i16) {
        if self.entities_active_url.is_empty() {
            return;
        }

        self.dirty_loadable_scenes = true;
        self.current_position = Coord(x, z);

        if self.should_load_city_scenes {
            let inner_parcels = self.parcel_radius_calculator.get_inner_parcels();
            let mut request_pointers = HashSet::with_capacity(inner_parcels.capacity());
            // Check what are the new scenes to load that are not in the cache
            for coord in inner_parcels {
                let coord = coord.plus(&self.current_position);

                // If I already have the scene data, continue
                if self.cache_city_pointers.contains_key(&coord) {
                    continue;
                }

                request_pointers.insert(coord);
            }

            // Request the new pointers
            self.request_pointers(request_pointers);
        }
    }

    pub fn _update(&mut self) {
        while let Some(response) = self.http_requester.poll() {
            match response {
                Ok(response) => {
                    if response.status_code.as_u16() >= 200 && response.status_code.as_u16() < 300 {
                        self.handle_response(response);
                        self.dirty_loadable_scenes = true;
                    } else {
                        self.cleanup_request_id(response.request_option.id);
                        tracing::info!(
                            "status code while doing a request: {:?}",
                            response.status_code
                        );
                        tracing::info!("{response:?}");
                    }
                }
                Err(err) => {
                    tracing::info!("Error while doing a request: {err:?}");
                }
            }
        }

        if self.dirty_loadable_scenes {
            self.dirty_loadable_scenes = false;
            self.update_loadable_and_keep_alive_scenes();
        }
    }

    pub fn get_loadable_scenes(&self) -> &HashSet<String> {
        &self.loadable_scenes
    }

    pub fn get_keep_alive_scenes(&self) -> &HashSet<String> {
        &self.keep_alive_scenes
    }

    pub fn get_empty_parcels(&self) -> &HashSet<String> {
        &self.empty_parcels
    }

    pub fn _get_version(&self) -> u32 {
        self.version
    }

    #[allow(dead_code)]
    pub fn pending_response(&self) -> bool {
        !(self.requested_city_pointers.is_empty() && self.requested_entity.is_empty())
    }
}

#[godot_api]
impl SceneEntityCoordinator {
    #[func]
    fn config(
        &mut self,
        entities_active_url: GString,
        content_url: GString,
        should_load_city_scenes: bool,
    ) {
        self._config(
            entities_active_url.to_string(),
            content_url.to_string(),
            should_load_city_scenes,
        );
    }

    #[func]
    pub fn get_desired_scenes(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        let mut loadable_scenes = VariantArray::new();
        let mut keep_alive_scenes = VariantArray::new();
        let mut empty_parcels = VariantArray::new();

        for loadable_scene in self.get_loadable_scenes().iter() {
            loadable_scenes.push(Variant::from(GString::from(loadable_scene)));
        }

        for keep_alive_scene in self.get_keep_alive_scenes().iter() {
            keep_alive_scenes.push(Variant::from(GString::from(keep_alive_scene)));
        }

        for empty_parcel in self.get_empty_parcels().iter() {
            empty_parcels.push(Variant::from(GString::from(empty_parcel)));
        }

        dict.set(GString::from("loadable_scenes"), loadable_scenes);
        dict.set(GString::from("keep_alive_scenes"), keep_alive_scenes);
        dict.set(GString::from("empty_parcels"), empty_parcels);

        dict
    }

    #[func]
    pub fn get_version(&self) -> u32 {
        self.version
    }

    #[func]
    pub fn set_scene_radius(&mut self, new_value: i16) {
        self.parcel_radius_calculator = ParcelRadiusCalculator::new(new_value);

        // This triggers the update of the loadable scenes
        self.update_position(self.current_position.0, self.current_position.1);
    }

    #[func]
    pub fn set_fixed_desired_entities_urns(&mut self, entities: VariantArray) {
        let entities = entities
            .iter_shared()
            .map(|entity| entity.to_string())
            .collect::<Vec<_>>();
        self._set_fixed_desired_entities_urns(entities);
    }

    #[func]
    pub fn set_fixed_desired_entities_global_urns(&mut self, entities: VariantArray) {
        let entities = entities
            .iter_shared()
            .map(|entity| entity.to_string())
            .collect::<Vec<_>>();
        self._set_fixed_desired_entities_global_urns(entities);
    }

    #[func]
    pub fn set_current_position(&mut self, x: i16, z: i16) {
        self.update_position(x, z);
    }

    #[func]
    pub fn get_scene_definition(&self, entity_id: GString) -> Option<Gd<DclSceneEntityDefinition>> {
        self.cache_scene_data
            .get(&entity_id.to_string())
            .map(DclSceneEntityDefinition::from_ref)
    }

    #[func]
    pub fn update(&mut self) {
        self._update();
    }

    #[func]
    pub fn get_scene_entity_id(&self, coord: Vector2i) -> GString {
        let coord = Coord(coord.x as i16, coord.y as i16);
        if let Some(entity_id) = self.cache_city_pointers.get(&coord) {
            GString::from(entity_id)
        } else {
            GString::from("empty")
        }
    }

    #[func]
    pub fn reload_scene_data(&mut self, scene_id: GString) {
        let scene_id = scene_id.to_string();
        let mut coord_to_clean = Vec::new();
        for (key, value) in self.cache_city_pointers.iter() {
            if value.eq(&scene_id) {
                coord_to_clean.push(key.clone());
            }
        }

        for coord in coord_to_clean.iter() {
            self.cache_city_pointers.remove(coord);
        }

        self.cache_scene_data.remove(&scene_id);
        self.update_position(self.current_position.0, self.current_position.1);
    }
}

#[godot_api]
impl INode for SceneEntityCoordinator {
    fn init(_base: Base<Node>) -> Self {
        SceneEntityCoordinator::new("".into(), "".into(), false)
    }
}

#[cfg(test)]
mod tests {
    const TEST_URN: &str = "urn:decentraland:entity:bafkreias3hru4s64inlkwceqeghlolpjjfaqaxxmghvuyrcfzs6u5fmg2q?=&baseUrl=https://sdk-team-cdn.decentraland.org/ipfs/";
    const TEST_URN_HASH: &str = "bafkreias3hru4s64inlkwceqeghlolpjjfaqaxxmghvuyrcfzs6u5fmg2q";
    const TEST_POINTER_O_O_ID: &str = "b64-L3Vzci9zcmMvYXBwLzAuMC5ibGFuay1zY2VuZQ==";

    use super::*;

    fn wait_update_or_timeout(
        scene_entity_coordinator: &mut SceneEntityCoordinator,
        timeout_ms: u32,
    ) -> bool {
        let mut remaining_ms: i32 = timeout_ms as i32;
        while scene_entity_coordinator.pending_response() && remaining_ms > 0 {
            scene_entity_coordinator._update();
            std::thread::sleep(std::time::Duration::from_millis(10));
            remaining_ms -= 10;
        }
        remaining_ms > 0
    }

    #[test]
    fn test_scene_entity_coordinator() {
        // let mock_server = mock_server();
        // let entities_active_url = mock_server.url("/content/entities/active");
        // let content_url = mock_server.url("/");

        // TODO: the mock server is not working in the github actions
        // The test now is using the real server
        let entities_active_url =
            "https://sdk-test-scenes.decentraland.zone/content/entities/active".to_string();
        let content_url =
            "https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-update-asset-pack-lib/contents".to_string();

        let mut scene_entity_coordinator =
            SceneEntityCoordinator::new(entities_active_url.clone(), content_url.clone(), false);

        // Test scenes
        scene_entity_coordinator.set_scene_radius(0);
        scene_entity_coordinator.set_current_position(74, -7);
        scene_entity_coordinator._set_fixed_desired_entities_urns(vec![
            TEST_URN.to_string(),
            "unknown_entity+".to_string(),
        ]);
        assert!(wait_update_or_timeout(&mut scene_entity_coordinator, 10000));

        assert!(scene_entity_coordinator
            .get_loadable_scenes()
            .contains(&TEST_URN_HASH.to_string()));
        assert!(!scene_entity_coordinator
            .get_loadable_scenes()
            .contains(&TEST_POINTER_O_O_ID.to_string()));

        // Test parcels

        let mut scene_entity_coordinator =
            SceneEntityCoordinator::new(entities_active_url, content_url, true);
        scene_entity_coordinator.update_position(0, 0);

        assert!(wait_update_or_timeout(&mut scene_entity_coordinator, 10000));
        assert!(!scene_entity_coordinator
            .get_loadable_scenes()
            .contains(&TEST_URN_HASH.to_string()));
        assert!(scene_entity_coordinator
            .get_loadable_scenes()
            .contains(&TEST_POINTER_O_O_ID.to_string()));
    }
}

#[itest]
fn some() {
    tracing::info!("this is a itest");
}
