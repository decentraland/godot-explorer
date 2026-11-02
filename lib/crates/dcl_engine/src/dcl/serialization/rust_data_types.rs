use super::{
    reader::{DclReader, DclReaderError, FromDclReader},
    writer::{DclWriter, ToDclWriter},
};

impl FromDclReader for bool {
    fn from_reader(buf: &mut DclReader) -> Result<Self, DclReaderError> {
        Ok(buf.read_u8()? > 0)
    }
}

impl ToDclWriter for bool {
    fn to_writer(&self, buf: &mut DclWriter) {
        buf.write_u8(if *self { 1 } else { 0 });
    }
}

impl FromDclReader for i32 {
    fn from_reader(buf: &mut DclReader) -> Result<Self, DclReaderError> {
        Ok(buf.read_u32()? as i32)
    }
}

impl ToDclWriter for i32 {
    fn to_writer(&self, buf: &mut DclWriter) {
        buf.write_u32(*self as u32);
    }
}
