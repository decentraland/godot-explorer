#ifndef INC_DEPTH
#define INC_DEPTH

vec4 get_depth_data(float depth, vec2 uv, mat4 inv_proj_mat, mat4 cam) {
	vec3 ndc = vec3(uv, depth) * 2.0 - 1.0;
	vec4 view = inv_proj_mat * vec4(ndc, 1.0);
	view.xyz /= view.w;

	vec4 world = cam * inv_proj_mat * vec4(ndc, 1.0);
	vec3 world_position = world.xyz / world.w;

	return vec4(world_position,-view.z);
}

#endif
