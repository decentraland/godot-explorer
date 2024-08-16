pub mod entity;
pub mod grow_only_set;
pub mod last_write_wins;
pub mod message;

use std::{
    any::Any,
    collections::{HashMap, HashSet},
};

use self::{
    entity::SceneEntityContainer,
    grow_only_set::{GenericGrowOnlySetComponent, GrowOnlySet},
    last_write_wins::{GenericLastWriteWinsComponent, LastWriteWins},
};

use super::components::{
    internal_player_data::InternalPlayerData, proto_components,
    transform_and_parent::DclTransformAndParent, SceneComponentId, SceneEntityId,
};

#[derive(Debug)]
pub struct SceneCrdtState {
    pub components: HashMap<SceneComponentId, Box<dyn Any + Send>>,
    pub entities: SceneEntityContainer,
}

pub trait InsertIfNotExists<T> {
    fn insert_if_not_exists(&mut self, value: T) -> bool;
}

impl<T: PartialEq> InsertIfNotExists<T> for Vec<T> {
    fn insert_if_not_exists(&mut self, value: T) -> bool {
        if !self.contains(&value) {
            self.push(value);
            true
        } else {
            false
        }
    }
}

pub type DirtyLwwComponents = HashMap<SceneComponentId, Vec<SceneEntityId>>;
pub type DirtyGosComponents = HashMap<SceneComponentId, HashMap<SceneEntityId, usize>>;

// message from scene-thread describing new and deleted entities
#[derive(Debug, Default)]
pub struct DirtyEntities {
    pub born: HashSet<SceneEntityId>,
    pub died: HashSet<SceneEntityId>,
}

#[derive(Debug, Default)]
pub struct DirtyCrdtState {
    pub entities: DirtyEntities,
    pub lww: DirtyLwwComponents,
    pub gos: DirtyGosComponents,
}

impl Default for SceneCrdtState {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneCrdtState {
    pub fn new() -> Self {
        let mut crdt_state = SceneCrdtState {
            components: HashMap::new(),
            entities: SceneEntityContainer::new(),
        };
        crdt_state.insert_lww_component::<DclTransformAndParent>(SceneComponentId::TRANSFORM);
        crdt_state
            .insert_lww_component::<InternalPlayerData>(SceneComponentId::INTERNAL_PLAYER_DATA);
        crdt_state
    }

    fn insert_lww_component<T: 'static + Send>(
        &mut self,
        component_id: SceneComponentId,
    ) -> &mut Self {
        self.components
            .entry(component_id)
            .or_insert(Box::new(LastWriteWins::<T>::new()));

        self
    }

    fn insert_gos_component<T: 'static + Send>(
        &mut self,
        component_id: SceneComponentId,
    ) -> &mut Self {
        self.components
            .entry(component_id)
            .or_insert(Box::new(GrowOnlySet::<T>::new()));

        self
    }

    pub fn get_lww_component_definition(
        &self,
        component_id: SceneComponentId,
    ) -> Option<&dyn GenericLastWriteWinsComponent> {
        if SceneCrdtStateProtoComponents::is_proto_component_id(component_id) {
            return self.get_proto_lww_component_definition(component_id);
        }

        match component_id {
            SceneComponentId::TRANSFORM => self
                .get_unknown_lww_component::<LastWriteWins<DclTransformAndParent>>(
                    SceneComponentId::TRANSFORM,
                ),
            SceneComponentId::INTERNAL_PLAYER_DATA => self
                .get_unknown_lww_component::<LastWriteWins<InternalPlayerData>>(
                    SceneComponentId::INTERNAL_PLAYER_DATA,
                ),
            _ => None,
        }
    }

    pub fn get_gos_component_definition(
        &self,
        component_id: SceneComponentId,
    ) -> Option<&dyn GenericGrowOnlySetComponent> {
        if SceneCrdtStateProtoComponents::is_proto_component_id(component_id) {
            return self.get_proto_gos_component_definition(component_id);
        }
        None
    }

    pub fn get_unknown_gos_component<T: 'static + GenericGrowOnlySetComponent>(
        &self,
        component_id: SceneComponentId,
    ) -> Option<&dyn GenericGrowOnlySetComponent> {
        let component = self.components.get(&component_id)?.downcast_ref::<T>()?;
        Some(component)
    }

    pub fn get_unknown_lww_component<T: 'static + GenericLastWriteWinsComponent>(
        &self,
        component_id: SceneComponentId,
    ) -> Option<&dyn GenericLastWriteWinsComponent> {
        let component = self.components.get(&component_id)?.downcast_ref::<T>()?;
        Some(component)
    }

    pub fn get_lww_component_definition_mut(
        &mut self,
        component_id: SceneComponentId,
    ) -> Option<&mut dyn GenericLastWriteWinsComponent> {
        if SceneCrdtStateProtoComponents::is_proto_component_id(component_id) {
            return self.get_proto_lww_component_definition_mut(component_id);
        }

        match component_id {
            SceneComponentId::TRANSFORM => self
                .get_unknown_lww_component_mut::<LastWriteWins<DclTransformAndParent>>(
                    SceneComponentId::TRANSFORM,
                ),
            SceneComponentId::INTERNAL_PLAYER_DATA => self
                .get_unknown_lww_component_mut::<LastWriteWins<InternalPlayerData>>(
                    SceneComponentId::INTERNAL_PLAYER_DATA,
                ),
            _ => None,
        }
    }

    pub fn get_gos_component_definition_mut(
        &mut self,
        component_id: SceneComponentId,
    ) -> Option<&mut dyn GenericGrowOnlySetComponent> {
        if SceneCrdtStateProtoComponents::is_proto_component_id(component_id) {
            return self.get_proto_gos_component_definition_mut(component_id);
        }
        None
    }

    pub fn get_unknown_lww_component_mut<T: 'static + GenericLastWriteWinsComponent>(
        &mut self,
        component_id: SceneComponentId,
    ) -> Option<&mut dyn GenericLastWriteWinsComponent> {
        let component = self
            .components
            .get_mut(&component_id)?
            .downcast_mut::<T>()?;
        Some(component)
    }

    pub fn get_unknown_gos_component_mut<T: 'static + GenericGrowOnlySetComponent>(
        &mut self,
        component_id: SceneComponentId,
    ) -> Option<&mut dyn GenericGrowOnlySetComponent> {
        let component = self
            .components
            .get_mut(&component_id)?
            .downcast_mut::<T>()?;
        Some(component)
    }

    pub fn get_lww_component_mut<T: 'static>(
        &mut self,
        component_id: SceneComponentId,
    ) -> Option<&mut LastWriteWins<T>> {
        let component = self
            .components
            .get_mut(&component_id)?
            .downcast_mut::<LastWriteWins<T>>()?;
        Some(component)
    }

    pub fn get_gos_component_mut<T: 'static>(
        &mut self,
        component_id: SceneComponentId,
    ) -> Option<&mut GrowOnlySet<T>> {
        let component = self
            .components
            .get_mut(&component_id)?
            .downcast_mut::<GrowOnlySet<T>>()?;
        Some(component)
    }

    pub fn take_dirty(&mut self) -> DirtyCrdtState {
        let mut dirty_lww_components: DirtyLwwComponents = HashMap::new();
        let mut dirty_gos_components: DirtyGosComponents = HashMap::new();
        let keys: Vec<SceneComponentId> = self.components.keys().cloned().collect(); // another way to do this?
        let dirty_entities = self.entities.take_dirty();

        for component_id in keys.iter() {
            if let Some(component_definition) = self.get_lww_component_definition_mut(*component_id)
            {
                let mut dirty = component_definition.take_dirty();

                for entity in dirty_entities.died.iter() {
                    component_definition.remove_without_dirty(*entity);
                    dirty.remove(entity);
                }

                if !dirty.is_empty() {
                    dirty_lww_components.insert(*component_id, dirty.into_iter().collect());
                }
            }
        }

        for component_id in keys.iter() {
            if let Some(component_definition) = self.get_gos_component_definition_mut(*component_id)
            {
                let mut dirty = component_definition.take_dirty();

                for entity in dirty_entities.died.iter() {
                    component_definition.clean_without_dirty(*entity);
                    dirty.remove(entity);
                }

                if !dirty.is_empty() {
                    dirty_gos_components.insert(*component_id, dirty);
                }
            }
        }

        DirtyCrdtState {
            entities: dirty_entities,
            lww: dirty_lww_components,
            gos: dirty_gos_components,
        }
    }

    pub fn get_transform_mut(&mut self) -> &mut LastWriteWins<DclTransformAndParent> {
        self.components
            .get_mut(&SceneComponentId::TRANSFORM)
            .unwrap()
            .downcast_mut::<LastWriteWins<DclTransformAndParent>>()
            .unwrap()
    }

    pub fn get_transform(&self) -> &LastWriteWins<DclTransformAndParent> {
        self.components
            .get(&SceneComponentId::TRANSFORM)
            .unwrap()
            .downcast_ref::<LastWriteWins<DclTransformAndParent>>()
            .unwrap()
    }

    pub fn get_internal_player_data_mut(&mut self) -> &mut LastWriteWins<InternalPlayerData> {
        self.components
            .get_mut(&SceneComponentId::INTERNAL_PLAYER_DATA)
            .unwrap()
            .downcast_mut::<LastWriteWins<InternalPlayerData>>()
            .unwrap()
    }

    pub fn get_internal_player_data(&self) -> &LastWriteWins<InternalPlayerData> {
        self.components
            .get(&SceneComponentId::INTERNAL_PLAYER_DATA)
            .unwrap()
            .downcast_ref::<LastWriteWins<InternalPlayerData>>()
            .unwrap()
    }

    pub fn kill_entity(&mut self, entity_id: &SceneEntityId) {
        self.entities.kill(*entity_id);
        // TODO: iterato over every component and remove
    }
}

include!(concat!(env!("OUT_DIR"), "/crdt_impl.gen.rs"));

mod test {
    #[allow(unused_imports)]
    use crate::dcl::{
        components::{SceneCrdtTimestamp, SceneEntityId},
        crdt::last_write_wins::{LWWEntry, LastWriteWinsComponentOperation},
        serialization::{reader::DclReader, writer::DclWriter},
    };

    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_invalid_component_id() {
        let mut crdt_state = SceneCrdtState::default();
        assert!(crdt_state
            .get_lww_component_mut::<bool>(SceneComponentId(0))
            .is_none());

        crdt_state.insert_lww_component::<bool>(SceneComponentId(0));

        assert!(crdt_state
            .get_lww_component_mut::<bool>(SceneComponentId(0))
            .is_some());
    }

    #[test]
    fn test_invalid_component_type() {
        let mut crdt_state = SceneCrdtState::default();
        assert!(crdt_state
            .get_lww_component_mut::<bool>(SceneComponentId(0))
            .is_none());

        crdt_state.insert_lww_component::<bool>(SceneComponentId(0));

        assert!(crdt_state
            .get_lww_component_mut::<bool>(SceneComponentId(0))
            .is_some());

        assert!(crdt_state
            .get_lww_component_mut::<u64>(SceneComponentId(0))
            .is_none());
    }

    #[test]
    fn test_adding_and_retrieving_proto_component() {
        let mut crdt_state = SceneCrdtState::from_proto();

        let mesh_renderer_component =
            SceneCrdtStateProtoComponents::get_mesh_renderer_mut(&mut crdt_state);
        let some_mesh_renderer = proto_components::sdk::components::PbMeshRenderer::default();
        mesh_renderer_component.set(
            SceneEntityId::new(0, 0),
            SceneCrdtTimestamp(0),
            Some(some_mesh_renderer),
        );

        let mesh_renderer = mesh_renderer_component.get(&SceneEntityId::new(0, 0));
        assert_eq!(
            *mesh_renderer.unwrap(),
            LWWEntry {
                timestamp: SceneCrdtTimestamp(0),
                value: Some(proto_components::sdk::components::PbMeshRenderer { mesh: None })
            }
        );
    }

    #[test]
    fn test_updating_proto_component() {
        let mut crdt_state = SceneCrdtState::from_proto();
        let mesh_renderer_component =
            SceneCrdtStateProtoComponents::get_mesh_renderer_mut(&mut crdt_state);

        let new_mesh_renderer = proto_components::sdk::components::PbMeshRenderer {
            mesh: Some(
                proto_components::sdk::components::pb_mesh_renderer::Mesh::Box(
                    proto_components::sdk::components::pb_mesh_renderer::BoxMesh { uvs: vec![] },
                ),
            ),
        };
        mesh_renderer_component.set(
            SceneEntityId::new(0, 0),
            SceneCrdtTimestamp(0),
            Some(new_mesh_renderer),
        );

        let mesh_renderer = mesh_renderer_component.get(&SceneEntityId::new(0, 0));
        assert_eq!(
            *mesh_renderer.unwrap(),
            LWWEntry {
                timestamp: SceneCrdtTimestamp(0),
                value: Some(proto_components::sdk::components::PbMeshRenderer {
                    mesh: Some(
                        proto_components::sdk::components::pb_mesh_renderer::Mesh::Box(
                            proto_components::sdk::components::pb_mesh_renderer::BoxMesh {
                                uvs: vec![]
                            }
                        )
                    )
                })
            }
        );
    }

    #[test]
    fn test_removing_proto_component() {
        let mut crdt_state = SceneCrdtState::from_proto();
        let mesh_renderer_component =
            SceneCrdtStateProtoComponents::get_mesh_renderer_mut(&mut crdt_state);

        mesh_renderer_component.set(SceneEntityId::new(0, 0), SceneCrdtTimestamp(0), None);
        let mesh_renderer = mesh_renderer_component.get(&SceneEntityId::new(0, 0));
        assert_eq!(
            *mesh_renderer.unwrap(),
            LWWEntry {
                timestamp: SceneCrdtTimestamp(0),
                value: None
            }
        );

        mesh_renderer_component.remove(SceneEntityId::new(0, 0));
        let mesh_renderer = mesh_renderer_component.get(&SceneEntityId::new(0, 0));
        assert!(mesh_renderer.is_none());
    }

    #[test]
    fn test_setting_proto_component_from_binary() {
        let mut crdt_state = SceneCrdtState::from_proto();
        let mesh_renderer_component_generic = crdt_state.get_unknown_lww_component_mut::<LastWriteWins<
            proto_components::sdk::components::PbMeshRenderer,
        >>(SceneComponentId::MESH_RENDERER);
        assert!(mesh_renderer_component_generic.is_some());

        let mesh_renderer_component: &mut dyn GenericLastWriteWinsComponent =
            mesh_renderer_component_generic.unwrap();
        let bin_mesh = proto_components::sdk::components::PbMeshRenderer {
            mesh: Some(
                proto_components::sdk::components::pb_mesh_renderer::Mesh::Box(
                    proto_components::sdk::components::pb_mesh_renderer::BoxMesh {
                        uvs: vec![1.2, 1.3],
                    },
                ),
            ),
        };

        let mut buf = Vec::new();
        DclWriter::new(&mut buf).write(&bin_mesh);

        let mut reader = DclReader::new(&buf);
        mesh_renderer_component.set_from_binary(
            SceneEntityId::new(0, 0),
            SceneCrdtTimestamp(10),
            &mut reader,
        );
    }
}
