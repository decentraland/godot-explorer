use crate::dcl::components::transform_and_parent::DclTransformAndParent;

use super::{
    reader::{DclReader, DclReaderError, FromDclReader},
    writer::{DclWriter, ToDclWriter},
};

impl FromDclReader for DclTransformAndParent {
    fn from_reader(buf: &mut DclReader) -> Result<Self, DclReaderError> {
        Ok(DclTransformAndParent {
            translation: buf.read()?,
            rotation: buf.read()?,
            scale: buf.read()?,
            parent: buf.read()?,
        })
    }
}

impl ToDclWriter for DclTransformAndParent {
    fn to_writer(&self, buf: &mut DclWriter) {
        buf.write(&self.translation);
        buf.write(&self.rotation);
        buf.write(&self.scale);
        buf.write(&self.parent);
    }
}
