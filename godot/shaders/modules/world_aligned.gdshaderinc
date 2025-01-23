#ifndef INC_WORLD_ALIGNED_UV
#define INC_WORLD_ALIGNED_UV

vec2 fix_uv(vec2 uv, float val) {
	return mix(vec2(-uv.x,uv.y), vec2(uv.x, uv.y), max(sign(val),0.0));
}
vec2 world_aligned_uv(vec3 normal, vec3 world_pos, vec3 distortion) {
	float x_value = dot(normal,vec3(1.0,.0,.0));
	float y_value = dot(normal,vec3(.0,1.0,.0));
	float z_value = dot(normal,vec3(.0,.0,1.0));
	float abs_x = abs(x_value)-distortion.x;
	float abs_y = abs(y_value)-distortion.y;
	float abs_z = abs(z_value)-distortion.z;
	return mix(
		mix(
			fix_uv(world_pos.zy * vec2(-1.0,1.0), x_value),
			fix_uv(world_pos.zx, y_value),
			step(abs_x,abs_y)
		), 
		fix_uv(world_pos.xy,z_value), 
		step(abs_x,abs_z) * step(abs_y,abs_z)
	);
}

#endif
