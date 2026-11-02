use godot::builtin::PackedByteArray;

// TODO: gdext should implement a packedByteArray constructor from &[u8] and not iteration

pub trait PackedByteArrayFromVec {
    fn from_vec(bytes_slice: &[u8]) -> PackedByteArray;
}

impl PackedByteArrayFromVec for PackedByteArray {
    fn from_vec(bytes_slice: &[u8]) -> PackedByteArray {
        let byte_length = bytes_slice.len();
        let mut bytes = PackedByteArray::new();
        bytes.resize(byte_length);

        bytes.as_mut_slice().copy_from_slice(bytes_slice);

        bytes
    }
}
