#ifndef INC_FLOW
#define INC_FLOW

vec3 apply_flow(sampler2D base_tex, vec3 flow_sample, float flow_speed, float flow_intensity, float time, vec2 uv, float uv_scale) {
	vec2 flow = (flow_sample.xy -.5) * 2.0;

	float phase1 = fract(time*flow_speed);
	float phase2 = fract(phase1+.5);
	float mix_t = abs((phase1-.5)*2.0);

	vec3 sample1 = texture(base_tex, uv_scale * (uv+flow * phase1 * flow_intensity)).rgb;
	vec3 sample2 = texture(base_tex, uv_scale * (uv+flow * phase2 * flow_intensity)).rgb;

	return mix(sample1, sample2, mix_t);
}

#endif
