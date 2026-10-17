class_name AvatarCollisionLayers
extends RefCounted

# Physics layers the avatar code masks and probes against, named as in
# project.godot ([layer_names]); the mask value is 1 << (layer - 1).
const CL_PHYSICS := 2
# Every avatar sits on CL_PLAYER. The local player adds CL_MAIN_PLAYER on top, so
# scenes and probes can tell it apart from remote avatars.
const CL_PLAYER := 4
const CL_MAIN_PLAYER := 8
