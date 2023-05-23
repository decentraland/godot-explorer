pub mod entity;
pub mod grow_only_set;
pub mod last_write_wins;

use std::{
    any::Any,
    collections::{HashMap, HashSet},
};

use self::{
    entity::SceneEntityContainer,
    grow_only_set::{GenericGrowOnlySetComponent, GrowOnlySet},
    last_write_wins::{GenericLastWriteWinsComponent, LastWriteWins},
};

use super::{
    components::{
        proto_components, transform_and_parent::DclTransformAndParent, SceneComponentId,
        SceneEntityId,
    },
    DirtyComponents, DirtyEntities,
};

#[derive(Debug)]
pub struct SceneCrdtState {
    pub components: HashMap<SceneComponentId, Box<dyn Any + Send>>,
    pub entities: SceneEntityContainer,
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
        &mut self,
        component_id: SceneComponentId,
    ) -> Option<&mut dyn GenericLastWriteWinsComponent> {
        if SceneCrdtStateProtoComponents::is_proto_component_id(component_id) {
            return self.get_proto_lww_component_definition(component_id);
        }

        match component_id {
            SceneComponentId::TRANSFORM => self
                .get_unknown_lww_component_mut::<LastWriteWins<DclTransformAndParent>>(
                    SceneComponentId::TRANSFORM,
                ),
            _ => None,
        }
    }

    pub fn get_gos_component_definition(
        &mut self,
        component_id: SceneComponentId,
    ) -> Option<&mut dyn GenericGrowOnlySetComponent> {
        if SceneCrdtStateProtoComponents::is_proto_component_id(component_id) {
            return self.get_proto_gos_component_definition(component_id);
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

    pub fn take_dirty(&mut self) -> (DirtyEntities, DirtyComponents) {
        let mut dirty_components: HashMap<SceneComponentId, HashSet<SceneEntityId>> =
            HashMap::new();
        let keys: Vec<SceneComponentId> = self.components.keys().cloned().collect(); // another way to do this?
        let dirty_entities = self.entities.take_dirty();

        for component_id in keys {
            if let Some(component_definition) = self.get_lww_component_definition(component_id) {
                let mut dirty = component_definition.take_dirty();

                for entity in dirty_entities.died.iter() {
                    component_definition.remove_without_dirty(*entity);
                    dirty.remove(entity);
                }

                if !dirty.is_empty() {
                    dirty_components.insert(component_id, dirty);
                }
            }
        }

        (dirty_entities, dirty_components)
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
}

include!(concat!(env!("OUT_DIR"), "/crdt_impl.gen.rs"));

mod test {
    // #[test]
    // fn test_invalid_component_id() {
    //     let mut crdt_state = SceneCrdtState::default();
    //     assert!(crdt_state
    //         .get_lww_component_mut::<bool>(SceneComponentId(0))
    //         .is_none());

    //     crdt_state.insert_lww_component::<bool>(SceneComponentId(0));

    //     assert!(crdt_state
    //         .get_lww_component_mut::<bool>(SceneComponentId(0))
    //         .is_some());
    // }

    // #[test]
    // fn test_invalid_component_type() {
    //     let mut crdt_state = SceneCrdtState::default();
    //     assert!(crdt_state
    //         .get_lww_component_mut::<bool>(SceneComponentId(0))
    //         .is_none());

    //     crdt_state.insert_lww_component::<bool>(SceneComponentId(0));

    //     assert!(crdt_state
    //         .get_lww_component_mut::<bool>(SceneComponentId(0))
    //         .is_some());

    //     assert!(crdt_state
    //         .get_lww_component_mut::<u64>(SceneComponentId(0))
    //         .is_none());
    // }

    // #[test]
    // fn test() {
    //     let mut crdt_state = SceneCrdtState::default();
    //     let entity = SceneEntityId::new(0, 0);
    //     let mut MeshRenderer = crdt_state
    //         .get_lww_component_mut::<PbMeshRenderer>(SceneComponentId::MESH_RENDERER)
    //         .unwrap();

    //     let some_mesh_renderer = PbMeshRenderer::default();
    //     MeshRenderer.set(
    //         SceneEntityId::new(0, 0),
    //         SceneCrdtTimestamp(0),
    //         Some(some_mesh_renderer),
    //     );

    //     let mesh_renderer = MeshRenderer.get(SceneEntityId::new(0, 0));
    //     assert_eq!(
    //         *mesh_renderer.unwrap(),
    //         LWWEntry {
    //             timestamp: SceneCrdtTimestamp(0),
    //             value: Some(PbMeshRenderer { mesh: None })
    //         }
    //     );

    //     let new_mesh_renderer = PbMeshRenderer {
    //         mesh: Some(pb_mesh_renderer::Mesh::Box(BoxMesh { uvs: vec![] })),
    //     };
    //     MeshRenderer.set(
    //         SceneEntityId::new(0, 0),
    //         SceneCrdtTimestamp(0),
    //         Some(new_mesh_renderer),
    //     );

    //     let mesh_renderer = MeshRenderer.get(SceneEntityId::new(0, 0));
    //     assert_eq!(
    //         *mesh_renderer.unwrap(),
    //         LWWEntry {
    //             timestamp: SceneCrdtTimestamp(0),
    //             value: Some(PbMeshRenderer {
    //                 mesh: Some(pb_mesh_renderer::Mesh::Box(BoxMesh { uvs: vec![] }))
    //             })
    //         }
    //     );

    //     MeshRenderer.set(SceneEntityId::new(0, 0), SceneCrdtTimestamp(0), None);

    //     let mesh_renderer = MeshRenderer.get(SceneEntityId::new(0, 0));
    //     assert_eq!(
    //         *mesh_renderer.unwrap(),
    //         LWWEntry {
    //             timestamp: SceneCrdtTimestamp(0),
    //             value: None
    //         }
    //     );

    //     MeshRenderer.remove(SceneEntityId::new(0, 0));

    //     let mesh_renderer = MeshRenderer.get(SceneEntityId::new(0, 0));
    //     assert!(mesh_renderer.is_none());

    //     let mut MeshRenderer = crdt_state
    //         .get_unknown_lww_component_mut::<LastWriteWins<PbMeshRenderer>>(
    //             SceneComponentId::MESH_RENDERER,
    //         );
    //     assert!(MeshRenderer.is_some());

    //     let MeshRenderer: &mut dyn GenericLastWriteWinsComponent = MeshRenderer.unwrap();
    //     let bin_mesh = PbMeshRenderer {
    //         mesh: Some(pb_mesh_renderer::Mesh::Box(BoxMesh {
    //             uvs: vec![1.2, 1.3],
    //         })),
    //     };

    //     let mut buf = Vec::new();
    //     DclWriter::new(&mut buf).write(&bin_mesh);

    //     let mut reader = DclReader::new(&buf);
    //     MeshRenderer.set_from_binary(
    //         SceneEntityId::new(0, 0),
    //         SceneCrdtTimestamp(10),
    //         &mut reader,
    //     );

    //     let MeshRenderer = crdt_state
    //         .get_lww_component_mut::<PbMeshRenderer>(SceneComponentId::MESH_RENDERER)
    //         .unwrap();

    //     let mesh_renderer = MeshRenderer.get(SceneEntityId::new(0, 0));
    //     println!("{:?}", mesh_renderer);
    // }
}
