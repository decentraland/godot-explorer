use std::collections::{HashMap, HashSet};

use crate::dcl::{
    components::{SceneCrdtTimestamp, SceneEntityId},
    serialization::{
        reader::{DclReader, FromDclReader},
        writer::{DclWriter, ToDclWriter},
    },
};

#[derive(Debug, PartialEq, Eq)]
pub struct LWWEntry<T: 'static> {
    pub timestamp: SceneCrdtTimestamp,
    pub value: Option<T>,
}

#[derive(Default)]
pub struct LastWriteWins<T: 'static> {
    pub values: HashMap<SceneEntityId, LWWEntry<T>>,
    pub dirty: HashSet<SceneEntityId>,
}

pub trait LastWriteWinsComponentOperation<T> {
    fn set(
        &mut self,
        entity: SceneEntityId,
        timestamp: SceneCrdtTimestamp,
        value: Option<T>,
    ) -> bool;
    fn put(&mut self, entity: SceneEntityId, value: Option<T>) -> bool;
    fn get(&self, entity: &SceneEntityId) -> Option<&LWWEntry<T>>;
}

// The generic trait is only applied to the component with the types that sastifies the implementation
//  which is T: 'static + FromDclReader + ToDclWriter (see the impl block below)
pub trait GenericLastWriteWinsComponent {
    fn set_from_binary(
        &mut self,
        entity: SceneEntityId,
        timestamp: SceneCrdtTimestamp,
        reader: &mut DclReader,
    ) -> bool;
    fn set_none(&mut self, entity: SceneEntityId, timestamp: SceneCrdtTimestamp) -> bool;
    fn get_opaque(&self, entity: SceneEntityId) -> Option<LWWEntry<()>>;

    fn to_binary(&self, entity: SceneEntityId, writer: &mut DclWriter) -> Result<(), String>;
    fn take_dirty(&mut self) -> HashSet<SceneEntityId>;

    fn remove(&mut self, entity: SceneEntityId);
    fn remove_without_dirty(&mut self, entity: SceneEntityId);
}

impl<T> LastWriteWins<T> {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            dirty: HashSet::new(),
        }
    }

    fn is_timestamp_greater(&self, entity: SceneEntityId, timestamp: SceneCrdtTimestamp) -> bool {
        if let Some(entry) = self.values.get(&entity) {
            timestamp > entry.timestamp
        } else {
            true
        }
    }
}

impl<T> LastWriteWinsComponentOperation<T> for LastWriteWins<T> {
    fn set(
        &mut self,
        entity: SceneEntityId,
        timestamp: SceneCrdtTimestamp,
        value: Option<T>,
    ) -> bool {
        if !self.is_timestamp_greater(entity, timestamp) {
            return false;
        }

        self.values.insert(entity, LWWEntry { value, timestamp });
        self.dirty.insert(entity);
        true
    }

    fn get(&self, entity: &SceneEntityId) -> Option<&LWWEntry<T>> {
        self.values.get(entity)
    }

    fn put(&mut self, entity: SceneEntityId, value: Option<T>) -> bool {
        let new_timestamp = if let Some(entry) = self.values.get(&entity) {
            SceneCrdtTimestamp(entry.timestamp.0 + 1)
        } else {
            SceneCrdtTimestamp(0)
        };

        self.set(entity, new_timestamp, value)
    }
}

impl<T: 'static + FromDclReader + ToDclWriter> GenericLastWriteWinsComponent for LastWriteWins<T> {
    fn set_from_binary(
        &mut self,
        entity: SceneEntityId,
        timestamp: SceneCrdtTimestamp,
        reader: &mut DclReader,
    ) -> bool {
        let value = T::from_reader(reader);
        if let Ok(value) = value {
            self.set(entity, timestamp, Some(value))
        } else {
            // If the from_reader fails, we don't perform the update
            false
        }
    }

    fn to_binary(&self, entity: SceneEntityId, writer: &mut DclWriter) -> Result<(), String> {
        if let Some(entry) = self.values.get(&entity) {
            if let Some(value) = entry.value.as_ref() {
                value.to_writer(writer);
                Ok(())
            } else {
                Err("Value is None".into())
            }
        } else {
            Err("Entity not found".into())
        }
    }

    fn get_opaque(&self, entity: SceneEntityId) -> Option<LWWEntry<()>> {
        self.values.get(&entity).map(|entry| LWWEntry {
            timestamp: entry.timestamp,
            value: entry.value.as_ref().map(|_| ()),
        })
    }

    fn set_none(&mut self, entity: SceneEntityId, timestamp: SceneCrdtTimestamp) -> bool {
        self.set(entity, timestamp, None)
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

mod test {
    use super::*;

    #[allow(dead_code)]
    fn get_i32_component_and_helper() -> (LastWriteWins<i32>, SceneEntityId, i32, i32, i32, i32) {
        let i32_component = super::LastWriteWins::<i32>::new();
        let entity = SceneEntityId::new(0, 0);
        let (a_value, b_value, c_value, d_value) = (123, 747, 555, 999);
        (i32_component, entity, a_value, b_value, c_value, d_value)
    }

    #[test]
    fn test_get_and_set() {
        let (mut i32_component, entity, a_value, b_value, c_value, d_value) =
            get_i32_component_and_helper();

        // 1) should not exist the entry
        assert!(i32_component.get(&entity).is_none());

        // 2) should put the initial value
        assert!(i32_component.set(entity, SceneCrdtTimestamp(0), Some(a_value)));
        assert_eq!(i32_component.get(&entity).unwrap().value, Some(a_value));

        // 3) should not put a value with the same timestamp (should not update)
        assert!(!i32_component.set(entity, SceneCrdtTimestamp(0), Some(b_value)));
        assert_eq!(i32_component.get(&entity).unwrap().value, Some(a_value));

        // 4) should put a value with a higher timestamp (should update)
        assert!(i32_component.set(entity, SceneCrdtTimestamp(1), Some(c_value)));
        assert_eq!(i32_component.get(&entity).unwrap().value, Some(c_value));

        // 5) should not work if the timestamp is lower than the current one (should not update)
        assert!(!i32_component.set(entity, SceneCrdtTimestamp(0), Some(d_value)));
        assert_eq!(i32_component.get(&entity).unwrap().value, Some(c_value));
    }

    #[test]
    fn test_set_none() {
        let (mut i32_component, entity, a_value, _, _, _) = get_i32_component_and_helper();

        assert!(i32_component.set(entity, SceneCrdtTimestamp(123), Some(a_value)));
        assert_eq!(i32_component.get(&entity).unwrap().value, Some(a_value));
        assert!(i32_component.set_none(entity, SceneCrdtTimestamp(124)));
        assert_eq!(i32_component.get(&entity).unwrap().value, None);
    }

    #[test]
    fn test_take_dirty() {
        let (mut i32_component, entity, a_value, _, _, _) = get_i32_component_and_helper();

        // Initially, the dirty set should be empty
        assert!(i32_component.take_dirty().is_empty());

        // Add an entry to the component
        assert!(i32_component.set(entity, SceneCrdtTimestamp(0), Some(a_value)));

        // Check that the entity is marked as dirty
        let dirty_set = i32_component.take_dirty();
        assert_eq!(dirty_set.len(), 1);
        assert!(dirty_set.contains(&entity));

        // After taking the dirty set, it should be empty again
        assert!(i32_component.take_dirty().is_empty());
    }

    #[test]
    fn test_remove_without_dirty() {
        let (mut i32_component, entity, a_value, _, _, _) = get_i32_component_and_helper();

        // Add an entry to the component
        assert!(i32_component.set(entity, SceneCrdtTimestamp(0), Some(a_value)));
        assert!(!i32_component.take_dirty().is_empty()); // clean the flag

        // Check that the entry is present
        assert!(i32_component.get(&entity).is_some());

        // Remove the entry without marking as dirty
        i32_component.remove_without_dirty(entity);

        // Check that the entry is not present
        assert!(i32_component.get(&entity).is_none());

        // Check that the dirty set is empty
        assert!(i32_component.take_dirty().is_empty());
    }

    #[test]
    fn test_remove() {
        let (mut i32_component, entity, a_value, _, _, _) = get_i32_component_and_helper();

        // Add an entry to the component
        assert!(i32_component.set(entity, SceneCrdtTimestamp(0), Some(a_value)));

        // Check that the entry is present
        assert!(i32_component.get(&entity).is_some());

        // Remove the entry
        i32_component.remove(entity);

        // Check that the entry is not present
        assert!(i32_component.get(&entity).is_none());

        // Check that the entity is marked as dirty
        let dirty_set = i32_component.take_dirty();
        assert_eq!(dirty_set.len(), 1);
        assert!(dirty_set.contains(&entity));
    }

    #[test]
    fn test_set_from_binary() {
        let mut component = LastWriteWins::<i32>::new();
        let entity = SceneEntityId::new(0, 0);
        let timestamp = SceneCrdtTimestamp(0);

        let data = 123;
        let mut buf = Vec::new();
        crate::dcl::serialization::writer::DclWriter::new(&mut buf).write(&data);
        let mut reader = DclReader::new(&buf);

        // Call set_from_binary
        assert!(component.set_from_binary(entity, timestamp, &mut reader));

        // Check that the component has the correct value
        let entry = component.get(&entity).unwrap();
        assert_eq!(entry.timestamp, timestamp);
        assert_eq!(*entry.value.as_ref().unwrap(), data);
    }
}
