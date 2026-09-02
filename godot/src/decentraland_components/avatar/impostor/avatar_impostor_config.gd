class_name AvatarImpostorConfig
extends Object

# Distance safety valve: beyond DISTANCE_FAR an avatar becomes a billboard
# even with a free pool slot (the pool only governs near/mid-range crowding).
const DISTANCE_FAR: float = 80.0
const MID_RANGE_NEAR: float = 15.0

const TINT_FULL_DISTANCE: float = 110.0

const DISTANCE_CHECK_PERIOD_FRAMES: int = 6
const CAPTURE_BUDGET_PER_FRAME: int = 1

# The FULL-mesh pool size (how many of the closest in-frustum avatars keep
# their animated 3D mesh before the rest fall back to billboard impostors)
# is a flat 40 — see AvatarPoolPolicy. Deliberately not tied to the graphics
# profile.

# When in MID we drive the AnimationTree manually and only call
# advance() every N frames. The skeleton then updates its bones at ~20fps
# (60/3) — imperceptible at 15-80m distance and frees significant CPU time.
const MID_ANIM_ADVANCE_EVERY_N_FRAMES: int = 3
