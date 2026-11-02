use crate::dcl::components::{SceneComponentId, SceneCrdtTimestamp, SceneEntityId};

use super::{
    reader::{DclReader, DclReaderError, FromDclReader},
    writer::{DclWriter, ToDclWriter},
};

impl FromDclReader for SceneEntityId {
    fn from_reader(buf: &mut DclReader) -> Result<Self, DclReaderError> {
        Ok(Self {
            number: buf.read_u16()?,
            version: buf.read_u16()?,
        })
    }
}

impl ToDclWriter for SceneEntityId {
    fn to_writer(&self, buf: &mut DclWriter) {
        buf.write_u16(self.number);
        buf.write_u16(self.version);
    }
}

impl FromDclReader for SceneComponentId {
    fn from_reader(buf: &mut DclReader) -> Result<Self, DclReaderError> {
        Ok(Self(buf.read_u32()?))
    }
}

impl ToDclWriter for SceneComponentId {
    fn to_writer(&self, buf: &mut DclWriter) {
        buf.write_u32(self.0)
    }
}

impl FromDclReader for SceneCrdtTimestamp {
    fn from_reader(buf: &mut DclReader) -> Result<Self, DclReaderError> {
        Ok(Self(buf.read_u32()?))
    }
}

impl ToDclWriter for SceneCrdtTimestamp {
    fn to_writer(&self, buf: &mut DclWriter) {
        buf.write_u32(self.0)
    }
}
