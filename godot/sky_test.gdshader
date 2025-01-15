shader_type sky;
render_mode use_half_res_pass;

uniform sampler2D clouds_tex : source_color;
uniform sampler2D clouds_gradient_day : source_color, repeat_disable, hint_default_white;
uniform sampler2D clouds_gradient_night : source_color, repeat_disable, hint_default_white;
uniform sampler2D stars_tex : source_color;
uniform float stars_scale = 3.0;

uniform float energy : hint_range(0.0, 10.0, 0.1) = 1.0;
uniform float gamma : hint_range(0.0,4.0) = 1.0;
uniform sampler2D tint_color_cycle_top : source_color, repeat_disable, hint_default_white;
uniform sampler2D tint_color_cycle_bottom : source_color, repeat_disable, hint_default_white;
uniform vec3 scale = vec3(1);
uniform float triplanar_blend_power : hint_range(0.0,40.0) = 4.0;
uniform float seamless_blend : hint_range(0.0,0.5) = 0.1;
uniform float dither_range = .1;

uniform float dither_amount = 128.0;

uniform float day_night_cycle : hint_range(0.0,1.0);

vec2 fix_uv(vec2 uv, float val) {
	return mix(vec2(-uv.x,uv.y), vec2(uv.x, uv.y), max(sign(val),0.0));
}
const float PHI = 1.61803398874989484820459;  // Φ = Golden Ratio   

float gold_noise(in vec2 xy, in float seed){
       return fract(tan(distance(xy*PHI, xy)*seed)*xy.x);
}

vec2 world_aligned_uv(vec3 normal, vec3 world_pos, vec3 distortion, vec4 frag_coord) {
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
			step(abs_x+dither_range*gold_noise(frag_coord.xy,1.0),abs_y)
		), 
		fix_uv(world_pos.xy,z_value), 
		step(abs_x+dither_range*gold_noise(frag_coord.xy,2.0),abs_z) * step(abs_y+dither_range*gold_noise(frag_coord.xy,3.0),abs_z)
	);
}

vec4 textureSeamless(sampler2D tex, vec2 uv, vec2 padding){
	vec2 inv_scale = 1.0 / (1.0 + 2.0 * padding);

	// Get partial derivatives here for textureGrad() to avoid mipmapping seams
	vec2 dx = dFdx(uv) * inv_scale;
	vec2 dy = dFdy(uv) * inv_scale;

	// Textures need to repeat somewhere. In addition, repeat must be enabled on the sampler
	uv = fract(uv+0.5)-0.5;

	// Get the bilinear blend factors. This is actually undefined for padding = 0
	vec2 u = smoothstep(-padding, padding, uv);

	// Apply scale
	uv *= inv_scale;

	// Bilinear interpolation
	inv_scale = 0.5 * (1.0 - inv_scale);
	vec4 lower_right = textureGrad(tex, uv - vec2(-1,-1) * inv_scale,  dx, dy);
	vec4 lower_left = textureGrad(tex, uv - vec2(1,-1) * inv_scale,  dx, dy);
	vec4 upper_right = textureGrad(tex, uv - vec2(-1,1) * inv_scale, dx, dy);
	vec4 upper_left = textureGrad(tex, uv - vec2(1,1) * inv_scale, dx, dy);

	lower_right = mix(lower_left, lower_right, u.x);
	upper_right = mix(upper_left, upper_right, u.x);

	return mix(upper_right, lower_right, u.y);
}

// "p" point being textured
// "n" surface normal at "p"
// "k" controls the sharpness of the blending in the transitions areas
// "s" texture sampler
vec4 boxmap( in sampler2D s, in vec3 p, in vec3 n, in float k, in vec4 frag_coord )
{
	vec2 uv = world_aligned_uv( n, p*.5-.5, vec3(0.0), frag_coord);
	return textureLod(s, uv, 0.0);
}


float interleaved_gradient_noise(vec2 n) {
    float f = 0.06711056 * n.x + 0.00583715 * n.y;
    return fract(52.9829189 * fract(f));
}

vec3 dither_interleaved(vec3 rgb, float levels, vec4 frag_coord) {
    float noise = interleaved_gradient_noise(frag_coord.xy);
    noise = noise - 0.5;
    return rgb + noise / (levels - 1.0);
}

void sky() {
    /* if (AT_HALF_RES_PASS) { */
		// Ray direction
		vec3 rd = EYEDIR;
		
		// Apply scaling
		vec3 p = rd;
		p = normalize(rd / scale) * scale;
		vec3 n = normalize(p / scale);
		
		// Triplanar mapping of sky texture
		float clouds = boxmap(clouds_tex, p, n, triplanar_blend_power, FRAGCOORD).r;
		/* float clouds = 0.0; */
		vec3 stars = boxmap(stars_tex, p * stars_scale, n, triplanar_blend_power, FRAGCOORD).rgb * 2.0;
		/* vec3 stars = vec3(0.0); */

		float day_night_blend = sin((day_night_cycle * 3.14159));
		COLOR = mix(
			textureLod(clouds_gradient_day, vec2(clouds, 0.0), 0.0).rgb,
			textureLod(clouds_gradient_night, vec2(clouds, 0.0), 0.0).rgb,
			day_night_blend
		);
		COLOR = energy * pow(COLOR, vec3(gamma));
		COLOR *= mix(
			textureLod(tint_color_cycle_bottom, vec2(day_night_cycle),0.0).rgb,
			textureLod(tint_color_cycle_top, vec2(day_night_cycle),0.0).rgb,
			smoothstep(.75,.25, SKY_COORDS.y)
		);
		COLOR += stars * day_night_blend * .5;
		COLOR = dither_interleaved(COLOR,dither_amount,FRAGCOORD);
    /* } else { */
    /*     COLOR = HALF_RES_COLOR.rgb; */
    /* } */
}
