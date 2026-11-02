use crate::dcl::components::proto_components::sdk;

use super::{
    reader::{DclReader, DclReaderError, FromDclReader},
    writer::{DclWriter, ToDclWriter},
};

pub trait DclProtoComponent: prost::Message + Default {}

impl<T: DclProtoComponent + Sync + Send + 'static> FromDclReader for T {
    fn from_reader(buf: &mut DclReader) -> Result<Self, DclReaderError> {
        Ok(Self::decode(buf.as_slice())?)
    }
}

impl<T: DclProtoComponent + Sync + Send + 'static> ToDclWriter for T {
    fn to_writer(&self, buf: &mut DclWriter) {
        self.encode(buf).unwrap()
    }
}

include!(concat!(env!("OUT_DIR"), "/dclcomponent.proto.impl.gen.rs"));
