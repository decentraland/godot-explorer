use crate::dcl::components::internal_player_data::InternalPlayerData;

use super::{
    reader::{DclReader, DclReaderError, FromDclReader},
    writer::{DclWriter, ToDclWriter},
};

impl FromDclReader for InternalPlayerData {
    fn from_reader(buf: &mut DclReader) -> Result<Self, DclReaderError> {
        Ok(InternalPlayerData {
            inside: buf.read()?,
        })
    }
}

impl ToDclWriter for InternalPlayerData {
    fn to_writer(&self, buf: &mut DclWriter) {
        buf.write(&self.inside);
    }
}
