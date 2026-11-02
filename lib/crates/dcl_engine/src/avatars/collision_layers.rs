//! Physics layers the avatar code masks and probes against, named as in project.godot
//! (`[layer_names]`); the mask value is 1 << (layer - 1).

pub const CL_PHYSICS: u32 = 2;
/// Every avatar sits on CL_PLAYER. The local player adds CL_MAIN_PLAYER on top, so scenes and
/// probes can tell it apart from remote avatars.
pub const CL_PLAYER: u32 = 4;
pub const CL_MAIN_PLAYER: u32 = 8;
