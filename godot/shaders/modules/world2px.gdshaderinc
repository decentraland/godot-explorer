#ifndef INC_WORLD_TO_PIXEL
#define INC_WORLD_TO_PIXEL

#include "./clip2px.gdshaderinc"

vec2 worldToPixel(vec3 worldPos, mat4 proj_mat, mat4 inv_cam_mat, vec2 view_size) {
	vec4 clip = proj_mat * inv_cam_mat * vec4(worldPos, 1.0);
	return clipToPixel(clip, view_size);
}

#endif
