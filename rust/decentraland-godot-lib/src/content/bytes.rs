use godot::builtin::PackedByteArray;

// TODO: gdext should implement a packedByteArray constructor from &[u8] and not iteration
pub fn fast_create_packed_byte_array_from_slice(bytes_slice: &[u8]) -> PackedByteArray {
    let byte_length = bytes_slice.len();
    let mut bytes = PackedByteArray::new();
    bytes.resize(byte_length);

    let data_arr_ptr = bytes.as_mut_slice();
    unsafe {
        let dst_ptr = &mut data_arr_ptr[0] as *mut u8;
        let src_ptr = &bytes_slice[0] as *const u8;
        std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, byte_length);
    }

    bytes
}

pub fn fast_create_packed_byte_array_from_vec(bytes_vec: &Vec<u8>) -> PackedByteArray {
    fast_create_packed_byte_array_from_slice(bytes_vec.as_slice())
}
