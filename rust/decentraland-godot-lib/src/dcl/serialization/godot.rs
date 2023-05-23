use super::{
    reader::{DclReader, DclReaderError, FromDclReader},
    writer::{DclWriter, ToDclWriter},
};

impl FromDclReader for godot::prelude::Quaternion {
    fn from_reader(buf: &mut DclReader) -> Result<Self, DclReaderError> {
        let x = buf.read_float()?;
        let y = buf.read_float()?;
        let z = buf.read_float()?;
        let w = buf.read_float()?;
        Ok(Self::new(x, y, z, w))
    }
}

impl ToDclWriter for godot::prelude::Quaternion {
    fn to_writer(&self, buf: &mut DclWriter) {
        buf.write_float(self.x);
        buf.write_float(self.y);
        buf.write_float(self.z);
        buf.write_float(self.w);
    }
}

impl FromDclReader for godot::prelude::Vector3 {
    fn from_reader(buf: &mut DclReader) -> Result<Self, DclReaderError> {
        let x = buf.read_float()?;
        let y = buf.read_float()?;
        let z = buf.read_float()?;
        Ok(Self::new(x, y, z))
    }
}

impl ToDclWriter for godot::prelude::Vector3 {
    fn to_writer(&self, buf: &mut DclWriter) {
        buf.write_float(self.x);
        buf.write_float(self.y);
        buf.write_float(self.z);
    }
}
