use godot::prelude::*;
use multihash_codetable::MultihashDigest;

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
