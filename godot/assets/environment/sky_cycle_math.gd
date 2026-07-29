class_name SkyCycleMath
extends RefCounted

# Pure day/night cycle math for SkyBase. Kept Global-free and RefCounted so it
# can be unit-tested standalone headless:
#   godot4_bin --headless --path godot \
#     --script res://src/test/environment/test_sky_cycle_math.gd
#
# The baked `light_cycle` animation drives a single DirectionalLight3D as sun
# by day AND moon by night: the night arc (~21:00 → ~05:00, normalized time
# ~0.83 → ~0.22 wrapped) keeps 15-32° of elevation while the visual sun is
# below the horizon. MOON_* defines the crossfade window between the two.
# Note: light ENERGY stays elevation-driven (see SkyBase), which is what
# masks the animation's dusk/dawn jumps — the light is off mid-jump.

const MOON_FADE_IN_START := 0.878  # ~21:04 — moon rises right after the whip
const MOON_FADE_IN_END := 0.905  # ~21:43 — moon fully in charge
# Fade-out is time-driven (not elevation): the baked moonset drops from
# +28° to 0° in a single keyframe interval, so an elevation gate cuts the
# light abruptly. The window ends exactly when the arc crosses the horizon
# (~04:21) and starts early enough to match the ~65min fade-in, so the moon
# dims gently while setting and never uplights.
const MOON_FADE_OUT_START := 0.12  # ~02:53 — moon starts dimming (still ~30° up)
const MOON_FADE_OUT_END := 0.181  # ~04:21 — moon fully out at the horizon

# The baked animation "teleports" the light from its sunset parking spot to
# the night arc between ~20:30 and ~21:00: azimuth whips 135° → -15° → 0°
# while elevation swings -10° → +48° → +74° → +15°. Elevation gating can't
# mask it (the whip goes way above the energy threshold), so ALL directional
# light is forced off during the segment — a short ambient-only blue hour —
# and the moon fades in afterwards on the stable arc (#2516 shadow jerk).
const LIGHTS_OUT_START := 0.85  # ~20:24 — just before the whip's first jump key
const LIGHTS_OUT_END := 0.878  # ~21:04 — whip finished, arc stable

# Day-arc gate for the sun light: it may only turn on while the baked
# animation is on the day arc. Rises at sunrise (~05:38 → ~06:21) and falls
# right after sunset (~18:50 → ~19:12); it stays 0 all night so the sun
# can't pop back on when the night arc rises (#2516 dusk flash).
const SUN_GATE_START := 0.235  # ~05:38
const SUN_GATE_END := 0.265  # ~06:21
const SUN_GATE_DOWN_START := 0.785  # ~18:50 — sunset done, energy already ~0
const SUN_GATE_DOWN_END := 0.8  # ~19:12 — sun gated for the night

# Sun energy falloff by elevation (kept from the original SkyBase._process):
# sun at/below -0.05 elevation -> 0, at/above 0.3 -> full energy.
const SUN_ENERGY_FADE_LOW := -0.05
const SUN_ENERGY_FADE_HIGH := 0.3


## 0.0 during the day, 1.0 when the baked animation is on its night (moon) arc.
static func compute_moon_factor(skybox_time: float) -> float:
	var evening := smoothstep(MOON_FADE_IN_START, MOON_FADE_IN_END, skybox_time)
	var morning := 1.0 - smoothstep(MOON_FADE_OUT_START, MOON_FADE_OUT_END, skybox_time)
	return clampf(evening + morning, 0.0, 1.0)


## True during the baked dusk whip: no directional light may be on.
static func is_lights_out(skybox_time: float) -> bool:
	return skybox_time >= LIGHTS_OUT_START and skybox_time < LIGHTS_OUT_END


## 0.0 while the animation is on the night (moon) arc, 1.0 on the day arc.
## Without this gate the sun light would crossfade IN during moonset
## (~04:00, elevation still ~28°) and flash warm light from the west at 4am,
## and pop back on at ~21:00 when the night arc rises after the dusk whip
## (#2516). The sun only lives on the day arc.
static func compute_sun_gate(skybox_time: float) -> float:
	var morning := smoothstep(SUN_GATE_START, SUN_GATE_END, skybox_time)
	var evening := 1.0 - smoothstep(SUN_GATE_DOWN_START, SUN_GATE_DOWN_END, skybox_time)
	return morning * evening


## Energy multiplier for the sun light from its elevation (y of the direction
## pointing TO the sun). 0 at/under the horizon, 1 at/above 0.3 (~17.5°).
static func compute_sun_energy_factor(elevation: float) -> float:
	return smoothstep(SUN_ENERGY_FADE_LOW, SUN_ENERGY_FADE_HIGH, elevation)


## Lifts `dir_to_light` (unit direction TO the sun/moon) so its elevation never
## drops below `min_elevation` (radians), preserving azimuth. Directions at or
## under the horizon are returned untouched (they produce no light anyway).
## Used so a low-but-visible sun keeps casting a readable ground shadow
## (issue #2516: morning/afternoon sun grazes the horizon -> no shadows).
static func clamp_direction_elevation(dir_to_light: Vector3, min_elevation: float) -> Vector3:
	var elevation := asin(clampf(dir_to_light.y, -1.0, 1.0))
	if elevation <= 0.0 or elevation >= min_elevation:
		return dir_to_light
	var horizontal := Vector2(dir_to_light.x, dir_to_light.z)
	var horizontal_len := horizontal.length()
	if horizontal_len < 0.0001:
		return dir_to_light
	var scale := cos(min_elevation) / horizontal_len
	return Vector3(dir_to_light.x * scale, sin(min_elevation), dir_to_light.z * scale).normalized()
