use crate::dcl::{components::SceneEntityId, DirtyEntities};
use std::collections::HashSet;

#[derive(Debug)]
pub struct SceneEntityContainer {
    // fixed array of 65536=2^16 elements, each index is the entity_number
    // the u16 item is the generation and the bool if is alive
    entity_version: Vec<(u16, bool)>,
    new_entities_created: HashSet<SceneEntityId>,
    entities_deleted: HashSet<SceneEntityId>,
}

impl Default for SceneEntityContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneEntityContainer {
    pub fn new() -> Self {
        let mut entity_version: Vec<(u16, bool)> = Vec::new();
        entity_version.resize(65536, (0, false));

        Self {
            entity_version,
            new_entities_created: Default::default(),
            entities_deleted: Default::default(),
        }
    }

    pub fn try_init(&mut self, new_entity: SceneEntityId) -> bool {
        let (version, live) = &mut self.entity_version[new_entity.number as usize];

        // The most common case, entity exists and it's the used one. (live = true, version = entity.version)
        if *live && new_entity.version == *version {
            return true;
        }

        // The version try to init is old (live = ?, version > new_entity.version)
        if *version > new_entity.version {
            return false;
        }

        // The version never was used, so I add it to new_entities (live = false, version = new_entity.version)
        if !*live && new_entity.version == *version {
            self.new_entities_created.insert(new_entity);
            *live = true;
        }

        // The version to use is newer, so current one is deleted (live = true/false, new_entity.version > version)
        if new_entity.version > *version {
            // current entity was used at least once, so I have to add to deleted entities
            if *live {
                let entity_to_die = SceneEntityId::new(new_entity.number, *version);
                self.entities_deleted.insert(entity_to_die);
                self.new_entities_created.remove(&entity_to_die);
            }

            *live = true;
            *version = new_entity.version;
            self.new_entities_created.insert(new_entity);
        }

        true
    }

    pub fn kill(&mut self, deleted_entity: SceneEntityId) {
        let (version, live) = &mut self.entity_version[deleted_entity.number as usize];

        // Typical case, the entity to delete is the current used one
        if deleted_entity.version >= *version {
            if *live {
                let entity_to_die = SceneEntityId::new(deleted_entity.number, *version);
                self.entities_deleted.insert(entity_to_die);
                self.new_entities_created.remove(&entity_to_die);
            }

            *version = deleted_entity.version + 1;
            *live = false;
        }

        // The entity is old
        if deleted_entity.version < *version {}
    }

    pub fn is_dead(&self, entity: &SceneEntityId) -> bool {
        self.entity_version[entity.number as usize].0 > entity.version
    }

    // If the entity is alive, return Some(entity) else None
    pub fn get_entity_stat(&self, entity_number: u16) -> &(u16, bool) {
        return &self.entity_version[entity_number as usize];
    }

    pub fn take_dirty(&mut self) -> DirtyEntities {
        DirtyEntities {
            born: std::mem::take(&mut self.new_entities_created),
            died: std::mem::take(&mut self.entities_deleted),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    impl DirtyEntities {
        fn to_tuple(&self) -> (HashSet<SceneEntityId>, HashSet<SceneEntityId>) {
            (self.born.clone(), self.died.clone())
        }
    }

    #[test]
    fn test_instance_new_entity_context() {
        let scene_entity_ctx = SceneEntityContainer::new();
        assert_eq!(
            scene_entity_ctx.entity_version.len(),
            usize::from(u16::MAX) + 1
        );

        assert_eq!(scene_entity_ctx.new_entities_created.len(), 0);
        assert_eq!(scene_entity_ctx.entities_deleted.len(), 0);
    }

    #[test]
    fn test_take_dirty() {
        let mut scene_entity_ctx = SceneEntityContainer::new();

        // 1. from empty context
        assert_eq!(
            scene_entity_ctx.take_dirty().to_tuple(),
            (HashSet::new(), HashSet::new())
        );

        // 2. one born, zero died
        scene_entity_ctx.try_init(SceneEntityId::new(0, 0));
        assert_eq!(
            scene_entity_ctx.take_dirty().to_tuple(),
            (HashSet::from([SceneEntityId::new(0, 0)]), HashSet::new())
        );

        // 3. one died, zero born
        scene_entity_ctx.kill(SceneEntityId::new(0, 0));
        assert_eq!(
            scene_entity_ctx.take_dirty().to_tuple(),
            (HashSet::new(), HashSet::from([SceneEntityId::new(0, 0)]))
        );

        // 4. now nothing happens
        assert_eq!(
            scene_entity_ctx.take_dirty().to_tuple(),
            (HashSet::new(), HashSet::new())
        );

        // 5. the same entity number try init multiple version
        assert!(scene_entity_ctx.try_init(SceneEntityId::new(32, 4)));
        assert!(scene_entity_ctx.try_init(SceneEntityId::new(32, 6)));
        assert!(scene_entity_ctx.try_init(SceneEntityId::new(32, 10)));
        assert!(scene_entity_ctx.try_init(SceneEntityId::new(32, 10)));
        assert!(!scene_entity_ctx.try_init(SceneEntityId::new(32, 7))); // this should be ignore
        assert_eq!(
            scene_entity_ctx.take_dirty().to_tuple(),
            (
                HashSet::from([SceneEntityId::new(32, 10)]),
                HashSet::from([SceneEntityId::new(32, 4), SceneEntityId::new(32, 6)])
            )
        );

        // 5. existing entity without any effect
        let entity = SceneEntityId::new(32, 10);
        assert!(scene_entity_ctx.try_init(entity));
        assert!(scene_entity_ctx.try_init(entity));
        assert!(scene_entity_ctx.try_init(entity));
        assert!(scene_entity_ctx.try_init(entity));
        assert_eq!(
            scene_entity_ctx.take_dirty().to_tuple(),
            (HashSet::new(), HashSet::new())
        );

        // 6. delete a newer entity that it doesn't exist
        let entity = SceneEntityId::new(32, 54);
        scene_entity_ctx.kill(entity);
        assert_eq!(
            scene_entity_ctx.take_dirty().to_tuple(),
            (HashSet::new(), HashSet::from([SceneEntityId::new(32, 10)]))
        );
    }
}
