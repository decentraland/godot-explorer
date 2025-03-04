#ifndef INC_GRID_SAMPLER
#define INC_GRID_SAMPLER

#include "./random.gdshaderinc"
#include "./saturate.gdshaderinc"
#include "./rotate2D.gdshaderinc"

const float TAU = 6.28318530718;

vec3 rand_color(vec2 id) {
	return fract(
		vec3(
			random(vec2(random(id.xy,10.0), random(id.yx,49.5)),30.0),
			random(vec2(random(id.xy,15.0), random(id.yx,32.6)),35.0),
			random(vec2(random(id.xy,20.0), random(id.yx,93.8)),40.0)
		)
	);
}

vec2 random_center(vec2 cell_id, float seed) {
	return vec2(random(cell_id.xy,1.0+seed),random(cell_id.yx,78.12839+seed*2.0));
}

vec4 grid_sampler(
	in vec2 in_uv, 
	in vec2 grid_size, 
	in int kernel_size, 
	in bool animated, 
	in float colored, 
	in float random_scale, 
	in float random_rotation, 
	in float local_scale,
	in sampler2D stroke,
	in bool full_uv_sample,
	in sampler2D tex
) {
	vec2 scaled_uv = in_uv * grid_size;
	vec2 gv = fract(scaled_uv);
	vec2 id = floor(scaled_uv);

	float curr_max = -1.0;
	vec4 result = vec4(0.0);
	float kernel = float(kernel_size);
	float angle = mix(TAU, TIME, float(animated));
	for(float i = -kernel; i < kernel+1.0; i+=1.0) 
	for(float j = -kernel; j < kernel+1.0; j+=1.0)
	{
		vec2 ij = vec2(i,j);
		vec2 curr_id = id + vec2(i,j);
		float rand0 = random(curr_id,21.1);
		float rand1 = random(curr_id,42.2);
		float rand2 = random(curr_id,34.3);
		float rand3 = random(curr_id,88.4);
		vec3 rand_col = rand_color(curr_id);
		vec2 center = random_center(curr_id,0.0) + vec2(sin(angle*rand0),cos(angle*rand1));
		vec2 uv = gv - vec2(i,j) - center;
		uv = rotate2D(uv-.5,rand2*angle*random_rotation)*(local_scale+random_scale*rand3*(1.0-local_scale))+.5;
		vec2 full_uv = (curr_id + center) / grid_size;
		if(rand3 > curr_max && uv.x < 1.0 && uv.y < 1.0 && uv.x > 0.0 && uv.y > 0.0) {
			vec4 sample = textureLod(stroke, uv, 0.0);
			sample.rgb = clamp(mix(sample.rgb, rand_col, colored),0.0,1.0);
			if(full_uv_sample) sample.rgb = textureLod(tex, full_uv, 0.0).rgb;
			result = mix(result, sample, sample.a);
		}
	}
	return result;
}

#endif
