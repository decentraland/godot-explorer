use num_traits::ToPrimitive;

use crate::dcl::crdt::message::CrdtMessageType;

use super::writer::{DclWriter, ToDclWriter};

impl ToDclWriter for CrdtMessageType {
    fn to_writer(&self, buf: &mut DclWriter) {
        buf.write_u32(ToPrimitive::to_u32(self).unwrap())
    }
}
