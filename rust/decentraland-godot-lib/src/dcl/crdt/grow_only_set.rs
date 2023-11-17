use std::collections::{HashMap, VecDeque};

use crate::dcl::{
    components::SceneEntityId,
    serialization::{
        reader::{DclReader, FromDclReader},
        writer::{DclWriter, ToDclWriter},
    },
};

#[derive(Debug, PartialEq, Eq)]
pub struct GrowOnlySet<T: 'static> {
    pub values: HashMap<SceneEntityId, VecDeque<T>>,
    pub dirty: HashMap<SceneEntityId, usize>,
}

pub trait GenericGrowOnlySetComponent {
    fn append_from_binary(&mut self, entity: SceneEntityId, reader: &mut DclReader);
    fn take_dirty(&mut self) -> HashMap<SceneEntityId, usize>;
    fn clean(&mut self, entity: SceneEntityId);
    fn clean_without_dirty(&mut self, entity: SceneEntityId);
    fn to_binary(
        &self,
        entity: SceneEntityId,
        element_index: usize,
        writer: &mut DclWriter,
    ) -> Result<(), String>;
}

pub trait GenericGrowOnlySetComponentOperation<T: 'static + FromDclReader + ToDclWriter> {
    fn append(&mut self, entity: SceneEntityId, value: T);
    fn get(&self, entity: &SceneEntityId) -> Option<&VecDeque<T>>;
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
    fn take_dirty(&mut self) -> HashMap<SceneEntityId, usize> {
        if self.dirty.is_empty() {
            HashMap::with_capacity(0)
        } else {
            std::mem::take(&mut self.dirty)
        }
    }

    fn clean_without_dirty(&mut self, entity: SceneEntityId) {
        self.values.remove(&entity);
    }

    fn clean(&mut self, entity: SceneEntityId) {
        self.values.remove(&entity);
        self.dirty.remove(&entity);
    }

    fn to_binary(
        &self,
        entity: SceneEntityId,
        reverse_element_index: usize,
        writer: &mut DclWriter,
    ) -> Result<(), String> {
        if let Some(entry) = self.values.get(&entity) {
            let element_index = entry.len() - reverse_element_index - 1;

            if let Some(value) = entry.get(element_index) {
                value.to_writer(writer);
                Ok(())
            } else {
                Err("Value is None".into())
            }
        } else {
            Err("Entity not found".into())
        }
    }
}

const APPEND_SIZE: usize = 100;

impl<T: 'static + FromDclReader + ToDclWriter> GenericGrowOnlySetComponentOperation<T>
    for GrowOnlySet<T>
{
    fn append(&mut self, entity: SceneEntityId, value: T) {
        let queue = self.values.entry(entity).or_default();
        if queue.len() == APPEND_SIZE {
            queue.pop_front().unwrap();
        }
        queue.push_back(value);
        let dirty_count = self.dirty.entry(entity).or_default();
        *dirty_count += 1;
    }

    fn get(&self, entity: &SceneEntityId) -> Option<&VecDeque<T>> {
        self.values.get(entity)
    }
}

mod test {
    #[test]
    fn test() {}
}
