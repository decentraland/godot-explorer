use std::collections::{HashMap, HashSet};

use crate::dcl::{
    components::{SceneCrdtTimestamp, SceneEntityId},
    serialization::{
        reader::{DclReader, FromDclReader},
        writer::ToDclWriter,
    },
};

#[derive(Debug, PartialEq, Eq)]
pub struct LWWEntry<T: 'static> {
    pub timestamp: SceneCrdtTimestamp,
    pub value: Option<T>,
}

pub struct LastWriteWins<T: 'static> {
    pub values: HashMap<SceneEntityId, LWWEntry<T>>,
    pub dirty: HashSet<SceneEntityId>,
}

pub trait GenericLastWriteWinsComponent {
    fn set_from_binary(
        &mut self,
        entity: SceneEntityId,
        timestamp: SceneCrdtTimestamp,
        reader: &mut DclReader,
    );

    fn set_none(&mut self, entity: SceneEntityId, timestamp: SceneCrdtTimestamp);

    fn take_dirty(&mut self) -> HashSet<SceneEntityId>;
    fn remove(&mut self, entity: SceneEntityId);
    fn remove_without_dirty(&mut self, entity: SceneEntityId);
}
pub trait LastWriteWinsComponentOperation<T: 'static + FromDclReader + ToDclWriter> {
    fn set(&mut self, entity: SceneEntityId, timestamp: SceneCrdtTimestamp, value: Option<T>);
    fn get(&self, entity: SceneEntityId) -> Option<&LWWEntry<T>>;
}

impl<T> Default for LastWriteWins<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> LastWriteWins<T> {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            dirty: HashSet::new(),
        }
    }
}

impl<T: 'static + FromDclReader + ToDclWriter> GenericLastWriteWinsComponent for LastWriteWins<T> {
    fn set_from_binary(
        &mut self,
        entity: SceneEntityId,
        timestamp: SceneCrdtTimestamp,
        reader: &mut DclReader,
    ) {
        let value = T::from_reader(reader);
        if let Ok(value) = value {
            self.values.insert(
                entity,
                LWWEntry {
                    value: Some(value),
                    timestamp,
                },
            );
        }
        self.dirty.insert(entity);
    }

    fn set_none(&mut self, entity: SceneEntityId, timestamp: SceneCrdtTimestamp) {
        self.values.insert(
            entity,
            LWWEntry {
                value: None,
                timestamp,
            },
        );
        self.dirty.insert(entity);
    }

    fn take_dirty(&mut self) -> HashSet<SceneEntityId> {
        if self.dirty.is_empty() {
            HashSet::with_capacity(0)
        } else {
            std::mem::take(&mut self.dirty)
        }
    }

    fn remove_without_dirty(&mut self, entity: SceneEntityId) {
        self.values.remove(&entity);
    }

    fn remove(&mut self, entity: SceneEntityId) {
        self.values.remove(&entity);
        self.dirty.insert(entity);
    }
}

impl<T: 'static + FromDclReader + ToDclWriter> LastWriteWinsComponentOperation<T>
    for LastWriteWins<T>
{
    fn set(&mut self, entity: SceneEntityId, timestamp: SceneCrdtTimestamp, value: Option<T>) {
        self.values.insert(entity, LWWEntry { value, timestamp });
        self.dirty.insert(entity);
    }

    fn get(&self, entity: SceneEntityId) -> Option<&LWWEntry<T>> {
        self.values.get(&entity)
    }
}

mod test {
    #[test]
    fn test() {}
}
