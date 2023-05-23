use std::collections::{HashMap, VecDeque};

use crate::dcl::{
    components::SceneEntityId,
    serialization::{
        reader::{DclReader, FromDclReader},
        writer::ToDclWriter,
    },
};

#[derive(Debug, PartialEq, Eq)]
pub struct GrowOnlySet<T: 'static> {
    pub values: HashMap<SceneEntityId, VecDeque<T>>,
    pub dirty: HashMap<SceneEntityId, u32>,
}

pub trait GenericGrowOnlySetComponent {
    fn append_from_binary(&mut self, entity: SceneEntityId, reader: &mut DclReader);
    fn take_dirty(&mut self) -> HashMap<SceneEntityId, u32>;
    fn clean(&mut self, entity: SceneEntityId);
}

pub trait GenericGrowOnlySetComponentOperation<T: 'static + FromDclReader + ToDclWriter> {
    fn append(&mut self, entity: SceneEntityId, value: Option<T>);
    fn get(&self, entity: SceneEntityId) -> Option<&VecDeque<T>>;
}

impl<T> GrowOnlySet<T> {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            dirty: HashMap::new(),
        }
    }
}

impl<T> Default for GrowOnlySet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static + FromDclReader + ToDclWriter> GenericGrowOnlySetComponent for GrowOnlySet<T> {
    fn append_from_binary(&mut self, _entity: SceneEntityId, _reader: &mut DclReader) {
        todo!();
    }
    fn take_dirty(&mut self) -> HashMap<SceneEntityId, u32> {
        if self.dirty.is_empty() {
            HashMap::with_capacity(0)
        } else {
            std::mem::take(&mut self.dirty)
        }
    }

    fn clean(&mut self, entity: SceneEntityId) {
        self.values.remove(&entity);
        self.dirty.remove(&entity);
    }
}

impl<T: 'static + FromDclReader + ToDclWriter> GenericGrowOnlySetComponentOperation<T>
    for GrowOnlySet<T>
{
    fn append(&mut self, _entity: SceneEntityId, _value: Option<T>) {
        todo!();
    }

    fn get(&self, entity: SceneEntityId) -> Option<&VecDeque<T>> {
        self.values.get(&entity)
    }
}

mod test {
    #[test]
    fn test() {}
}
