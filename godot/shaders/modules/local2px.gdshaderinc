#ifndef INC_LOCAL_TO_PIXEL
#define INC_LOCAL_TO_PIXEL

#include "clip2px.gdshaderinc"

vec2 localToPixel(vec3 locPos, mat4 proj_mat, mat4 inv_cam_mat, mat4 world_mat, vec2 view_size) {
	vec4 clip = proj_mat * inv_cam_mat * world_mat * vec4(locPos, 1.0);
	return clipToPixel(clip, view_size);
}

#endif
