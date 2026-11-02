use godot::prelude::*;
use multihash_codetable::MultihashDigest;

fn simple_hash(name: &str) -> u64 {
    let bytes = name.as_bytes();
    let mut hash: u64 = 2166136261; // FNV offset basis

    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(16777619); // FNV prime
    }

    hash
}

pub fn get_hash_number(name: String, min: i32, max: i32) -> i32 {
    if min > max {
        panic!("min cannot be greater than max");
    }

    if min == max {
        return min;
    }

    let hash = simple_hash(&name);

    let range_size = (max - min + 1) as u64;
    let mapped_value = (hash % range_size) as i32;

    min + mapped_value
}

pub fn hash_v1(content: &[u8]) -> String {
    let hash = multihash_codetable::Code::Sha2_256.digest(content);
    cid::Cid::new_v1(0x55, hash).to_string()
}

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclHashing {
    _base: Base<RefCounted>,
}
#[godot_api]
impl DclHashing {
    #[func]
    fn hash_v1(content: PackedByteArray) -> GString {
        let content = content.to_vec();
        hash_v1(content.as_ref()).to_godot()
    }
}
