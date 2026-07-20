class_name ProximityFade

## Camera proximity fade — the visual safety net of the see-through-walls fix
## (issue #1814). Scene geometry very close to the camera is dithered out, so
## when the camera still ends up inside a mesh (collider missing/coarser than
## the visual, single-sided walls, the lateral over-shoulder offset), the player
## sees a soft dissolve instead of black inside-out faces.
##
## Implemented with BaseMaterial3D.distance_fade (PIXEL_DITHER): per-fragment
## camera distance, so it works for meshes of any size, and it stays in the
## opaque pipeline (no transparency sorting side-effects).
##
## Applied at the scene-content material chokepoints:
##   - gltf_container.gd fix_material (all GLTF scene materials)
##   - lib/src/scene_runner/components/material.rs apply_dcl_material_properties
##     (SDK Material component on primitives — values duplicated there, keep in sync)
##
## Known gaps (deliberate): custom ShaderMaterials (e.g. SDK unlit with a
## separate alpha texture) and primitives using the engine fallback material
## (no Material component) are not faded.
##
## Tuning vs the SpringArm3D margin: the spring stops the camera `margin`
## meters from collider faces, so a wall the camera is legitimately pressed
## against must stay at/above FADE_START_DISTANCE (fully visible). The fade
## band below it is only ever entered when collision already failed.

const FADE_GONE_DISTANCE := 0.1  # fully transparent at/under this camera distance
const FADE_START_DISTANCE := 0.3  # starts dithering at this camera distance


static func apply_material(mat: BaseMaterial3D) -> void:
	mat.distance_fade_mode = BaseMaterial3D.DISTANCE_FADE_PIXEL_DITHER
	mat.distance_fade_min_distance = FADE_GONE_DISTANCE
	mat.distance_fade_max_distance = FADE_START_DISTANCE
