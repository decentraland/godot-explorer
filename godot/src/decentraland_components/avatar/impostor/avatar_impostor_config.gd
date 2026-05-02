class_name AvatarImpostorConfig
extends Object

const DISTANCE_FAR: float = 30.0
const DISTANCE_NEAR: float = 25.0
const MID_RANGE_NEAR: float = 15.0

const HYSTERESIS_FAR: float = 1.0
const HYSTERESIS_NEAR: float = 1.0
const HYSTERESIS_MID: float = 1.0

const TINT_FULL_DISTANCE: float = 60.0

const TEXTURE_SIZE: Vector2i = Vector2i(256, 512)
const SPRITE_SIZE_M: Vector2 = Vector2(2.0, 2.5)
const MAX_LAYERS: int = 256

const DISTANCE_CHECK_PERIOD_FRAMES: int = 6
const CAPTURE_BUDGET_PER_FRAME: int = 1

# Hard caps applied by AvatarLODCoordinator. The N closest avatars get FULL,
# the next M closest get MID/CROSSFADE, the rest are forced to FAR. Static
# distance thresholds still cap the upper tier (an avatar at 50m is FAR even
# if it's the only one in the scene), so caps only kick in under high
# concurrency.
const MAX_FULL_AVATARS: int = 8
const MAX_THROTTLED_AVATARS: int = 32

const MID_ANIMATION_PLAYBACK_SPEED: float = 0.5

# When in MID/CROSSFADE we drive the AnimationTree manually and only call
# advance() every N frames. The skeleton then updates its bones at ~20fps
# (60/3) — imperceptible at 15-25m distance and frees significant CPU time.
const MID_ANIM_ADVANCE_EVERY_N_FRAMES: int = 3
