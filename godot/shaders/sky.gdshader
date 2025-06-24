shader_type sky;

#include "./modules/voronoiEdges2d.gdshaderinc"
#include "./modules/rotate2D.gdshaderinc"
#include "./modules/cartesian2polar.gdshaderinc"
#include "./modules/polar2cartesian.gdshaderinc"

const float TIME_OFFSET = .43;

uniform sampler2D milky_way_noise : source_color;
uniform float stars_energy = 1.0;

uniform sampler2D clouds_panorama : source_color;

uniform sampler2D sky_gradient : source_color, repeat_disable, hint_default_white;

uniform sampler2D clouds_gradient_day : source_color, repeat_disable, hint_default_white;
uniform sampler2D clouds_gradient_night : source_color, repeat_disable, hint_default_white;

uniform float energy : hint_range(0.0, 10.0, 0.1) = 1.0;
uniform float gamma : hint_range(0.0,4.0) = 1.0;

uniform sampler2D tint_color_cycle_top : source_color, repeat_disable, hint_default_white;
uniform sampler2D tint_color_cycle_bottom : source_color, repeat_disable, hint_default_white;

uniform sampler2D sun_color_cycle : source_color, repeat_disable, hint_default_white;
uniform sampler2D moon_color_cycle : source_color, repeat_disable, hint_default_white;

uniform float dither_range = .1;
uniform float dither_amount = 128.0;

uniform vec2 other_sun_rotation_offset;

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
  //float day_night_cycle = fract(TIME*.1);
  //float day_night_cycle = .05;
  float cycle = day_night_cycle + TIME_OFFSET;
  cycle -= floor(cycle);

  vec3 sun_dir = normalize(vec3(0.2, 1.0, 0.0));
  sun_dir.yz = rotate2D(sun_dir.yz, PI*2.0*cycle);
  float dot_val = dot(sun_dir, EYEDIR);

  sun_dir.yz = rotate2D(sun_dir.yz, other_sun_rotation_offset.x);
  sun_dir.yx = rotate2D(sun_dir.yx, other_sun_rotation_offset.y);
  float dot_val_offset = dot(sun_dir, EYEDIR);

  float is_sun = max(sign(dot_val),0.0);
  float eye_dot =abs(dot(EYEDIR, vec3(.0,1.0,.0)));
  float horizon = 1.0-smoothstep(0.0,0.1,eye_dot);
  horizon *= horizon * horizon;
  float horizon_distortion = -smoothstep(1.0,.1,eye_dot);

  vec3 rd = EYEDIR;
  vec3 rotated_ray = rd;
  rotated_ray.yz = rotate2D(rotated_ray.yz, PI*.5);
  vec3 sd = cartesianToSpherical(rotated_ray);
  vec2 theta_phi = vec2(sd.y/PI, (sd.z+PI)/(2.0*PI));

  vec3 clouds_sample = textureLod(clouds_panorama, theta_phi.yx * vec2(1.0,-1.0),0.0).rgb;
  float clouds_mask = 1.0 - clouds_sample.r;
  float clouds_gradient = clouds_sample.g;

  float sun_shape = max(
    smoothstep(.99+horizon_distortion*.05, 1.0, abs(dot_val+.0025*gold_noise(FRAGCOORD.xy, 1.0))+horizon*.02),
    smoothstep(.998+horizon_distortion*.02, 1.0, abs(dot_val_offset+.0015*gold_noise(FRAGCOORD.xy, 1.0))+horizon*.02)
  );
  float moon_shape = smoothstep(.995+horizon_distortion*.0125, .996+horizon_distortion*.0125, abs(dot_val));

  float sun = mix(moon_shape, sun_shape, is_sun);

  vec2 uv = world_aligned_uv( rd, rd*.5-.5, vec3(0.0), vec2(.0));

  vec2 center;
  vec4 voronoi_data = voronoiEdges(uv*150.0, 0.9 + clouds_mask * .1, 0.0, 0.0, 0.0, center);
  float stars = smoothstep(.0,.2,voronoi_data.a);
  float milky_way = smoothstep(.5,1.0,dot(EYEDIR, normalize(vec3(0.0,EYEDIR.yz))))* texture(milky_way_noise, (EYEDIR.xz+1.0)*.5).r ;
  stars *= milky_way * stars_energy;


  float is_floor = smoothstep(.00125,-.00125,dot(EYEDIR, vec3(0.0, 1.0, 0.0)));

  float day_night_blend = sin(cycle * 3.14159);

  COLOR = mix(
      textureLod(clouds_gradient_day, vec2(clouds_gradient, 0.0), 0.0).rgb,
      textureLod(clouds_gradient_night, vec2(clouds_gradient, 0.0), 0.0).rgb,
      day_night_blend
  );


  COLOR = mix(
      texture(sky_gradient, vec2(day_night_blend, 0.0), 0.0).rgb,
      COLOR,
      clouds_mask
  );
  COLOR *= mix(
      textureLod(tint_color_cycle_bottom, vec2(cycle),0.0).rgb,
      textureLod(tint_color_cycle_top, vec2(cycle),0.0).rgb,
      smoothstep(.75,.25, SKY_COORDS.y)
  );

  COLOR += mix(stars * day_night_blend * day_night_blend * day_night_blend * day_night_blend * .5, 0.0, max(clouds_mask,sun));

  COLOR = dither_interleaved(COLOR,dither_amount,FRAGCOORD.xy);
  vec3 sun_color = textureLod(sun_color_cycle, vec2(cycle),0.0).rgb * 3.0;
  vec3 moon_color = textureLod(moon_color_cycle, vec2(day_night_blend),0.0).rgb * 1.5;
  COLOR = mix(COLOR, mix(moon_color, sun_color, is_sun), sun * (1.0-clouds_mask*mix(.9,.5,is_sun)));
  COLOR += sun_color * (smoothstep(0.7,1.7,dot_val)+smoothstep(0.7,1.7,dot_val_offset)) * .5;
  COLOR = mix(COLOR, floor_color, is_floor);
  COLOR = energy * pow(COLOR, vec3(gamma));
}
