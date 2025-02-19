shader_type sky;
/* render_mode use_quarter_res_pass; */
render_mode use_half_res_pass;

#include "./modules/voronoiEdges2d.gdshaderinc"
#include "./modules/rotate2D.gdshaderinc"
#include "./modules/cartesian2polar.gdshaderinc"
#include "./modules/polar2cartesian.gdshaderinc"

uniform sampler2D clouds_tex : source_color;
uniform sampler2D clouds_gradient_day : source_color, repeat_disable, hint_default_white;
uniform sampler2D clouds_gradient_night : source_color, repeat_disable, hint_default_white;

uniform float energy : hint_range(0.0, 10.0, 0.1) = 1.0;
uniform float gamma : hint_range(0.0,4.0) = 1.0;
uniform sampler2D tint_color_cycle_top : source_color, repeat_disable, hint_default_white;
uniform sampler2D tint_color_cycle_bottom : source_color, repeat_disable, hint_default_white;
uniform sampler2D sun_color_cycle : source_color, repeat_disable, hint_default_white;
uniform sampler2D moon_color_cycle : source_color, repeat_disable, hint_default_white;

uniform vec3 scale = vec3(1);
uniform float dither_range = .1;

uniform float dither_amount = 128.0;

global uniform float day_night_cycle : hint_range(0.0,1.0);

uniform vec3 floor_color : source_color;

vec2 fix_uv(vec2 uv, float val) {
	return mix(vec2(-uv.x,uv.y), vec2(uv.x, uv.y), max(sign(val),0.0));
}
const float PHI = 1.61803398874989484820459;  // Φ = Golden Ratio

float gold_noise(in vec2 xy, in float seed){
       return fract(tan(distance(xy*PHI, xy)*seed)*xy.x);
}

vec2 world_aligned_uv(vec3 normal, vec3 world_pos, vec3 distortion, vec2 frag_coord) {
	vec2 seed = frag_coord * 1000.0;
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
			step(abs_x+dither_range*gold_noise(seed,1.0),abs_y)
		),
		fix_uv(world_pos.xy,z_value),
		step(abs_x+dither_range*gold_noise(seed,2.0),abs_z) * step(abs_y+dither_range*gold_noise(seed,3.0),abs_z)
	);
}

vec3 cartesianToSpherical(vec3 cartesian) {
    float x = cartesian.x;
    float y = cartesian.y;
    float z = cartesian.z;

    // Calculate r (radius)
    float r = length(cartesian);

    // Calculate theta (polar angle)
    float theta = acos(z / r);

    // Calculate phi (azimuthal angle)
    float phi = atan(y, x);

    return vec3(r, theta, phi);
}

vec3 sphericalToCartesian(vec3 spherical) {
    float r = spherical.x;      // Radius
    float theta = spherical.y;  // Polar angle (theta)
    float phi = spherical.z;    // Azimuthal angle (phi)

    // Convert spherical to Cartesian coordinates
    float x = r * sin(theta) * cos(phi);
    float y = r * sin(theta) * sin(phi);
    float z = r * cos(theta);

    return vec3(x, y, z);
}

float interleaved_gradient_noise(vec2 n) {
    float f = 0.06711056 * n.x + 0.00583715 * n.y;
    return fract(52.9829189 * fract(f));
}

vec3 dither_interleaved(vec3 rgb, float levels, vec2 frag_coord) {
    float noise = interleaved_gradient_noise(frag_coord.xy);
    noise = noise - 0.5;
    return rgb + noise / (levels - 1.0);
}

void sky() {
	/* float day_night_cycle = fract(TIME*.1); */
	vec3 sun_dir = normalize(vec3(0.2, 1.0, 0.0));
	sun_dir.yz = rotate2D(sun_dir.yz, PI*2.0*day_night_cycle);
	float dot_val= dot(sun_dir, EYEDIR);
	float is_sun = max(sign(dot_val),0.0);
	float eye_dot =abs(dot(EYEDIR, vec3(.0,1.0,.0)));
	float horizon = 1.0-smoothstep(0.0,0.1,eye_dot);
	horizon *= horizon * horizon;
	float horizon_distortion = -smoothstep(1.0,.1,eye_dot);

	float sun_shape = smoothstep(.99+horizon_distortion*.05, 1.0, abs(dot_val+.0025*gold_noise(FRAGCOORD.xy, 1.0))+horizon*.02);
	float moon_shape = smoothstep(.995+horizon_distortion*.001, .996+horizon_distortion*.001, abs(dot_val));

	vec3 rd = EYEDIR;
	vec3 rotated_ray = rd;
	rotated_ray.yz = rotate2D(rotated_ray.yz, PI*.5);
	vec3 sd = cartesianToSpherical(rotated_ray);
	vec2 theta_phi = vec2(sd.y/PI, (sd.z+PI)/(2.0*PI));

	/* vec3 sd2 = cartesianToSpherical(sun_dir); */
	/* vec2 sun_uv = (sd.yz - sd2.yz); */
	/* vec2 sun_uv_mask = smoothstep(0.0,.00001,sun_uv) * smoothstep(1.0,.99999, sun_uv); */
	/* sun_uv *= sun_uv_mask.x * sun_uv_mask.y; */

    if (AT_HALF_RES_PASS) {

		vec2 uv = world_aligned_uv( rd, rd*.5-.5, vec3(0.0), vec2(.0));

		vec2 center;
		float stars = smoothstep(.0,.2,voronoiEdges(uv*300.0,0.9,0.0,0.0,0.0, center).a);
		float clouds = textureLod(clouds_tex, theta_phi*2.0, 0.0).r * smoothstep(.99,0.8,eye_dot);

		float sun = mix(
			moon_shape,
			sun_shape,
			is_sun
		);

		COLOR = vec3(stars, clouds, sun);
		ALPHA = is_sun;
    } else {
		float floor = smoothstep(.0125,-.0125,dot(EYEDIR, vec3(0.0, 1.0, 0.0)));

		float clouds = HALF_RES_COLOR.g;
		float stars = HALF_RES_COLOR.r;
		float sun = HALF_RES_COLOR.b;

		float day_night_blend = sin(day_night_cycle * 3.14159);

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

		COLOR += mix(stars * day_night_blend * day_night_blend * day_night_blend * day_night_blend * .5, 0.0, max(clouds,sun));

		COLOR = dither_interleaved(COLOR,dither_amount,FRAGCOORD.xy);
		vec3 sun_color = textureLod(sun_color_cycle, vec2(day_night_cycle),0.0).rgb * 3.0;
		vec3 moon_color =textureLod(moon_color_cycle, vec2(day_night_blend),0.0).rgb;
		COLOR = mix(
			COLOR,
			mix(
				moon_color,
				sun_color,
				is_sun
			),
			sun * (1.0-clouds)
		);
		COLOR = mix(COLOR, floor_color, floor);
		COLOR += sun_color * smoothstep(0.7,1.7,dot_val);
		/* COLOR = vec3(sun_uv,0.0); */
    }
}
