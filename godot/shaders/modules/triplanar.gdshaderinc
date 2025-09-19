#ifndef INC_TRIPLANAR_TEXTURE
#define INC_TRIPLANAR_TEXTURE

vec4 triplanar_texture(sampler2D p_sampler, vec3 p_weights, vec3 p_triplanar_pos) {
	vec4 samp = vec4(0.0);
	samp += texture(p_sampler, p_triplanar_pos.xy) * p_weights.z;
	samp += texture(p_sampler, p_triplanar_pos.xz) * p_weights.y;
	samp += texture(p_sampler, p_triplanar_pos.zy * vec2(-1.0, 1.0)) * p_weights.x;
	return samp;
}

vec4 triplanar_texture_atlas(sampler2D p_sampler, vec3 p_weights, vec3 p_triplanar_pos, vec4 rect) {
	vec4 samp = vec4(0.0);
	samp += texture(p_sampler, rect.xy + p_triplanar_pos.xy * rect.zw) * p_weights.z;
	samp += texture(p_sampler, rect.xy + p_triplanar_pos.xz * rect.zw) * p_weights.y;
	samp += texture(p_sampler, rect.xy + p_triplanar_pos.zy * rect.zw  * vec2(-1.0, 1.0)) * p_weights.x;
	return samp;
}

#endif
