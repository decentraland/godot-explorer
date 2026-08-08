class_name AvatarImpostorConfig
extends Object

const DISTANCE_FAR: float = 30.0
const DISTANCE_NEAR: float = 25.0
const MID_RANGE_NEAR: float = 15.0

const TINT_FULL_DISTANCE: float = 60.0

const DISTANCE_CHECK_PERIOD_FRAMES: int = 6
const CAPTURE_BUDGET_PER_FRAME: int = 1

# Real local snapshot generation (ImpostorCapturer gen stage) is rendered at
# this reduced resolution; set_impostor_texture upscales to the 256x512 layer.
# Cuts render + GPU readback cost vs the full-size capture.
const GEN_SNAPSHOT_SIZE: Vector2i = Vector2i(128, 256)

# Minimum frames between two real generations so they never run back-to-back
# (each one still re-assembles the avatar on the main thread). Spreads the
# remaining per-generation cost out instead of bursting.
const GEN_MIN_FRAMES_BETWEEN: int = 12

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
