shader_type sky;

#include "./modules/voronoiEdges2d.gdshaderinc"
#include "./modules/rotate2D.gdshaderinc"
#include "./modules/cartesian2polar.gdshaderinc"
#include "./modules/polar2cartesian.gdshaderinc"

const float TIME_OFFSET = .43;

uniform sampler2D milky_way_noise : source_color;
uniform float stars_energy = 1.0;

uniform sampler2D clouds_panorama : source_color;

// Cubemap clouds (imported from a 6×1 strip atlas). Sampled with EYEDIR rotated around
// the Y axis by clouds_rotation_y_deg + scroll, so the cloud silhouettes can be aligned
// to a reference (and slowly rotate with clouds_scroll_speed).
uniform samplerCube clouds_cubemap : source_color, hint_default_black;
uniform float clouds_rotation_y_deg : hint_range(-180.0, 180.0) = 0.0;
// Squashes the cubemap sampling vertically: <1 pulls clouds toward the horizon (clouds
// appear lower and concentrated near horizon), >1 pushes them up. Match Unity's apparent
// cloud altitude.
uniform float clouds_height_compress : hint_range(0.1, 2.0) = 1.5;

uniform sampler2D sky_gradient : source_color, repeat_disable, hint_default_white;

// Cloud color per time-of-day. Single gradient indexed by day_night_cycle (0=midnight,
// 0.5=noon, 1=midnight).
uniform sampler2D clouds_color_cycle : source_color, repeat_disable, hint_default_white;

uniform float energy : hint_range(0.0, 10.0, 0.1) = 1.0;
uniform float gamma : hint_range(0.0,4.0) = 1.0;

// Vertical tint zones. Legacy names kept for back-compat with VR lobby material:
//   tint_color_cycle_top    → applied at horizon (mid-band)
//   tint_color_cycle_bottom → applied at zenith (looking up)
//   tint_color_cycle_nadir  → applied below horizon (looking down)
uniform sampler2D tint_color_cycle_top : source_color, repeat_disable, hint_default_white;
uniform sampler2D tint_color_cycle_bottom : source_color, repeat_disable, hint_default_white;
uniform sampler2D tint_color_cycle_nadir : source_color, repeat_disable, hint_default_white;

uniform sampler2D sun_color_cycle : source_color, repeat_disable, hint_default_white;
uniform sampler2D moon_color_cycle : source_color, repeat_disable, hint_default_white;

// Sun radiance halo (Genesis Sky parity). Warm bloom around sun direction, transparent at
// night via the gradient — adds the "morning glow" that's missing from the procedural sun.
uniform sampler2D rim_color_cycle : source_color, repeat_disable, hint_default_white;
uniform float sun_radiance_intensity : hint_range(0.0, 5.0) = 0.05;
uniform float sun_radiance_falloff : hint_range(0.5, 32.0) = 12.0;

// Cheap cloud rotation. Single uniform, ~zero added GPU cost vs static panorama.
uniform float clouds_scroll_speed : hint_range(-0.05, 0.05) = 0.0;

uniform float dither_range = .1;
uniform float dither_amount = 128.0;

global uniform float day_night_cycle : hint_range(0.0,1.0);
global uniform vec3 sun_direction;
global uniform vec3 moon_direction;
global uniform float sun_opacity;
global uniform float sun_size;
global uniform float moon_mask_size;

// Synthetic sun direction for Rayleigh/Mie scattering. Driven by skybox_time so it cycles
// from -Y at midnight to +Y at noon. Independent from the visual sun_direction (which is
// driven by an animation that keeps the disc above horizon).
global uniform vec3 atm_sun_direction;

// Bake mode: when true, output pure Rayleigh/Mie atmosphere only (no clouds, sun/moon
// discs, stars, floor). Used by the cubemap baker (Phase E2). Runtime keeps it false
// and composites sun/moon/clouds/stars on top of the baked cubemap (Phase E3).
global uniform bool atm_bake_mode;

// === Atmospheric scattering ===
uniform vec3 atm_default_wavelength = vec3(0.65, 0.57, 0.475);
uniform vec3 atm_variable_wavelength_range = vec3(0.15, 0.15, 0.15);
uniform float atm_outer_radius = 1.025;
uniform float atm_inner_radius = 1.0;
uniform float atm_camera_height = 0.000001;
uniform float atm_rayleigh_max = 0.0025;
uniform float atm_rayleigh_pow = 2.5;
uniform float atm_mie = 0.0010;
uniform float atm_sun_brightness = 20.0;
uniform float atm_max_scatter = 50.0;
uniform float atm_scale_depth = 0.25;
uniform float atm_km_4pi_multi = 4.0;
uniform float atm_scale_over_scale_depth_multi = 0.25;
uniform float atm_mie_g = -0.990;
uniform float atm_mie_g2 = 0.9801;
uniform float atm_sky_ground_threshold = 0.02;
uniform float atm_thickness : hint_range(0.0, 5.0) = 1.0;
uniform vec4 atm_sky_tint : source_color = vec4(0.5, 0.5, 0.5, 1.0);
uniform vec4 atm_ground_color : source_color = vec4(0.369, 0.349, 0.341, 1.0);
uniform float atm_exposure : hint_range(0.0, 5.0) = 1.3;

// Baked atmosphere lookup — CubemapArray with one layer per keyframe (uniform 2h
// spacing: 00, 02, 04, ..., 22). Uniform spacing lets the shader pick adjacent
// layers + blend factor with pure arithmetic — no loop, no branches.
uniform samplerCubeArray baked_array : hint_default_black;
const int BAKE_LAYERS = 12;

// Second sun orbital parameters
uniform float second_sun_speed : hint_range(0.0, 2.0) = 0.1;
uniform float second_sun_orbit : hint_range(0.0, 0.5) = 0.1;
uniform float second_sun_size_factor : hint_range(0.01, 4.0) = 0.1;

// Moon mask 2D offset
uniform vec2 moon_mask_offset = vec2(0.01, -0.01);

// Floor: replaces flat purple cap with TOD-modulated gradient + soft horizon falloff.
uniform sampler2D floor_color_cycle : source_color, repeat_disable, hint_default_white;
uniform float floor_horizon_blend : hint_range(0.01, 0.5) = 0.15;
uniform float floor_nadir_dim : hint_range(0.0, 1.0) = 1.0;

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
    float r = length(cartesian);
    float theta = acos(z / r);
    float phi = atan(y, x);
    return vec3(r, theta, phi);
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

// Rodrigues' rotation formula (axis + angle in radians)
vec3 rotateAboutAxis(vec3 point, vec3 axis, float angle) {
    float s = sin(angle);
    float c = cos(angle);
    float omc = 1.0 - c;
    return vec3(
        point.x * (omc * axis.x * axis.x + c)         + point.y * (omc * axis.x * axis.y - axis.z * s) + point.z * (omc * axis.x * axis.z + axis.y * s),
        point.x * (omc * axis.y * axis.x + axis.z * s) + point.y * (omc * axis.y * axis.y + c)         + point.z * (omc * axis.y * axis.z - axis.x * s),
        point.x * (omc * axis.z * axis.x - axis.y * s) + point.y * (omc * axis.z * axis.y + axis.x * s) + point.z * (omc * axis.z * axis.z + c)
    );
}

// === Atmospheric scattering helpers (DCL_SkyBox_Fragment.hlsl port) ===
float atm_scale(float in_cos) {
    float x = 1.0 - in_cos;
    return 0.25 * exp(-0.00287 + x * (0.459 + x * (3.83 + x * (-6.80 + x * 5.25))));
}

float atm_rayleigh_phase(float eye_cos2) {
    return 0.75 + 0.75 * eye_cos2;
}

float atm_rayleigh_phase_dir(vec3 light, vec3 ray) {
    float eye_cos = dot(light, ray);
    return atm_rayleigh_phase(eye_cos * eye_cos);
}

// Pure atmospheric scattering — port of DCL_SkyBox_Fragment.hlsl::sk_frag without the
// Mie-attenuated sun/moon discs (those are composited procedurally at runtime).
// `eye_ray` and `sun_pos` must be normalized.
vec3 atm_compute_atmosphere(vec3 eye_ray, vec3 sun_pos) {
    // Apply gamma to sky_tint so 0.5 maps to mid-grey (sRGB 128) for visually linear
    // interpolation. Godot is linear-space → apply gamma exponent.
    vec3 sky_tint_gamma = pow(atm_sky_tint.rgb, vec3(1.0 / 2.2));
    vec3 wavelength = mix(
        atm_default_wavelength - atm_variable_wavelength_range,
        atm_default_wavelength + atm_variable_wavelength_range,
        vec3(1.0) - sky_tint_gamma
    );
    vec3 inv_wavelength = 1.0 / pow(wavelength, vec3(4.0));

    float k_rayleigh = mix(0.0, atm_rayleigh_max, pow(atm_thickness, atm_rayleigh_pow));
    float k_kr_e_sun = k_rayleigh * atm_sun_brightness;
    float k_kr_4pi = k_rayleigh * atm_km_4pi_multi * PI;
    float k_km_4pi = atm_mie * atm_km_4pi_multi * PI;
    float k_scale = 1.0 / (atm_outer_radius - 1.0);
    float k_scale_over_scale_depth = k_scale / atm_scale_over_scale_depth_multi;

    vec3 camera_pos = vec3(0.0, atm_inner_radius + atm_camera_height, 0.0);
    float outer_radius_sq = atm_outer_radius * atm_outer_radius;
    float inner_radius_sq = atm_inner_radius * atm_inner_radius;

    vec3 v_in = vec3(0.0);
    vec3 v_out = vec3(0.0);

    if (eye_ray.y >= -0.03) {
        // Sky branch — 2-sample integration along the ray (unrolled)
        float up = eye_ray.y;
        float far = sqrt(outer_radius_sq + inner_radius_sq * up * up - inner_radius_sq) - atm_inner_radius * up;

        float start_height = atm_inner_radius + atm_camera_height;
        float start_depth = exp(k_scale_over_scale_depth * (-atm_camera_height));
        float start_angle = dot(eye_ray, camera_pos) / start_height;
        float start_offset = start_depth * atm_scale(start_angle);

        float sample_length = far / 2.0;
        float scaled_length = sample_length * k_scale;
        vec3 sample_ray = eye_ray * sample_length;
        vec3 sample_point = camera_pos + sample_ray * 0.5;

        vec3 front_color = vec3(0.0);
        // sample 0
        {
            float h = length(sample_point);
            float depth = exp(k_scale_over_scale_depth * (atm_inner_radius - h));
            float light_angle = dot(sun_pos, sample_point) / h;
            float camera_angle = dot(eye_ray, sample_point) / h;
            float scatter = start_offset + depth * (atm_scale(light_angle) - atm_scale(camera_angle));
            vec3 attn = exp(-clamp(scatter, 0.0, atm_max_scatter) * (inv_wavelength * k_kr_4pi + k_km_4pi));
            front_color += attn * (depth * scaled_length);
            sample_point += sample_ray;
        }
        // sample 1
        {
            float h = length(sample_point);
            float depth = exp(k_scale_over_scale_depth * (atm_inner_radius - h));
            float light_angle = dot(sun_pos, sample_point) / h;
            float camera_angle = dot(eye_ray, sample_point) / h;
            float scatter = start_offset + depth * (atm_scale(light_angle) - atm_scale(camera_angle));
            vec3 attn = exp(-clamp(scatter, 0.0, atm_max_scatter) * (inv_wavelength * k_kr_4pi + k_km_4pi));
            front_color += attn * (depth * scaled_length);
        }
        v_in = front_color * (inv_wavelength * k_kr_e_sun);
        v_out = front_color * (atm_mie * atm_sun_brightness);
    } else {
        // Ground branch — single-sample
        float far = (-atm_camera_height) / min(-0.001, eye_ray.y);
        vec3 pos = camera_pos + far * eye_ray;

        float start_depth = exp((-atm_camera_height) * (1.0 / atm_scale_depth));
        float camera_angle_g = dot(-eye_ray, pos);
        float light_angle_g = dot(sun_pos, pos);
        float camera_scale = atm_scale(camera_angle_g);
        float light_scale = atm_scale(light_angle_g);
        float camera_offset = start_depth * camera_scale;
        float t_temp = light_scale + camera_scale;

        float sample_length = far / 2.0;
        float scaled_length = sample_length * k_scale;
        vec3 sample_ray = eye_ray * sample_length;
        vec3 sample_point = camera_pos + sample_ray * 0.5;

        vec3 front_color = vec3(0.0);
        vec3 attn = vec3(0.0);
        {
            float h = length(sample_point);
            float depth = exp(k_scale_over_scale_depth * (atm_inner_radius - h));
            float scatter = depth * t_temp - camera_offset;
            attn = exp(-clamp(scatter, 0.0, atm_max_scatter) * (inv_wavelength * k_kr_4pi + k_km_4pi));
            front_color += attn * (depth * scaled_length);
        }
        // Adds k_kr_e_sun (scalar) as a constant offset to inv_wavelength.
        v_in = front_color * (inv_wavelength * k_kr_e_sun + k_kr_e_sun);
        v_out = clamp(attn, vec3(0.0), vec3(1.0));
    }

    vec3 ground_atm = atm_exposure * (v_in + atm_ground_color.rgb * v_out);
    vec3 sky_atm = atm_exposure * (v_in * atm_rayleigh_phase_dir(sun_pos, -eye_ray));

    float y = eye_ray.y / atm_sky_ground_threshold;
    return mix(sky_atm, ground_atm, clamp(-y, 0.0, 1.0));
}

void sky() {
  // Sun and moon directions come from animated DirectionalLight (sky_base.gd pushes uniforms)
  vec3 sun_dir = normalize(sun_direction);
  vec3 moon_dir = normalize(moon_direction);

  // === BAKE MODE — pure Rayleigh/Mie atmosphere only (Phase E2 cubemap baker) ===
  // Skip all compositing (clouds, sun/moon discs, stars, floor, dither, gamma).
  // Output stays in linear space so the cubemap can be sampled directly at runtime.
  // Note: Godot's sky processor disallows `return`, so we wrap the runtime path in else.
  // Uses atm_sun_direction (synthetic, cycles below horizon at night) — see global uniform.
  if (atm_bake_mode) {
    COLOR = atm_compute_atmosphere(EYEDIR, normalize(atm_sun_direction));
  } else {
  float cycle = day_night_cycle + TIME_OFFSET;
  cycle -= floor(cycle);

  float sun_dot = dot(sun_dir, EYEDIR);
  float moon_dot = dot(moon_dir, EYEDIR);

  float eye_y = dot(EYEDIR, vec3(0.0, 1.0, 0.0));
  float eye_dot = abs(eye_y);
  float horizon = 1.0 - smoothstep(0.0, 0.1, eye_dot);
  horizon *= horizon * horizon;

  // Cubemap-sampled clouds. Rotate EYEDIR around Y by user-set offset + cheap scroll,
  // then squash Y so clouds sit closer to the horizon.
  vec3 rd = EYEDIR;
  float angle = TIME * clouds_scroll_speed * TAU + radians(clouds_rotation_y_deg);
  float cs = cos(angle);
  float sn = sin(angle);
  vec3 cloud_eye = vec3(cs * rd.x + sn * rd.z, rd.y * clouds_height_compress, -sn * rd.x + cs * rd.z);

  vec3 clouds_sample = textureLod(clouds_cubemap, cloud_eye, 0.0).rgb;
  // Cloud cubemap channel encoding:
  //   G channel = cloud SHAPE (mask, 0=sky 1=cloud body — but values can be low at edges)
  //   R channel = cloud HIGHLIGHT/shading (varies inside cloud: brighter top, darker bottom)
  // Smoothstep amplifies low G values so partial cloud coverage at edges is visible.
  float clouds_mask = smoothstep(0.05, 0.4, clouds_sample.g);
  float clouds_gradient = clouds_sample.r;

  // === SUN DISC ===
  // Was 0.3 (too small per user) → 0.5 (too big per user) → 0.35 (target between).
  float sun_angle = acos(clamp(sun_dot, -1.0, 1.0));
  float remapped_sun_size = sun_size * 0.35;
  float sun_disc = step(sun_angle, remapped_sun_size);

  // === SECOND SUN (orbital secondary, pulsates around the sun/moon position) ===
  float ss_rotation = second_sun_speed * TIME;
  vec3 rotated_orbit = rotateAboutAxis(vec3(-1.0, 0.0, -1.0), vec3(0.0, 1.0, 0.0), ss_rotation);
  vec3 second_sun_dir = normalize(sun_dir + rotated_orbit * second_sun_orbit);
  float second_sun_angle = acos(clamp(dot(second_sun_dir, EYEDIR), -1.0, 1.0));
  float base_ss_size = remapped_sun_size * second_sun_size_factor;
  float ss_pulse = (cos(ss_rotation) + 1.0) * 0.5;
  float total_ss_size = base_ss_size + base_ss_size * ss_pulse;
  float second_sun_disc = step(second_sun_angle, total_ss_size);

  // === MOON MASK (carves the crescent C bite out of the sun + second sun discs) ===
  // Bite radius and offset both scale with remapped_sun_size so the C shape stays
  // proportionally consistent regardless of the sun's animated size. At peak
  // moon_mask_size (~0.17), bite ≈ 1.3× sun radius, offset ≈ 0.4× sun radius —
  // bite extends past the sun edge on one side so the crescent tips stay open.
  vec3 moon_mask_dir = normalize(
      sun_dir + vec3(moon_mask_offset.x, moon_mask_offset.y, 0.0) * remapped_sun_size * 40.0
  );
  float moon_mask_angle = acos(clamp(dot(moon_mask_dir, EYEDIR), -1.0, 1.0));
  float moon_mask_threshold = moon_mask_size * remapped_sun_size * 4.5;
  // Gate the bite on/off so it disappears abruptly when moon_mask_size drops below the
  // threshold — otherwise the curve's interpolation through 0 leaves a shrinking bite
  // visible while the sun rises.
  float moon_active = step(0.15, moon_mask_size);
  float moon_mask_step = mix(1.0, step(moon_mask_threshold, moon_mask_angle), moon_active);

  // Gate celestial rendering by sun horizon position so the sun + bite never render
  // below horizon (avoids the bite appearing in the floor area when sun_dir dips
  // briefly below during the animation's sunset/sunrise transitions).
  float sun_above_horizon = smoothstep(-0.02, 0.05, sun_dir.y);
  float combined_disc = clamp(sun_disc + second_sun_disc, 0.0, 1.0);
  float celestial = combined_disc * moon_mask_step * sun_opacity * sun_opacity * sun_above_horizon;

  float day_night_blend = sin(cycle * 3.14159);

  // Stars: only compute at night (skip voronoi during daytime — perf win)
  float stars = 0.0;
  if (day_night_blend > 0.01) {
    vec2 uv = world_aligned_uv(rd, rd*.5-.5, vec3(0.0), vec2(.0));
    vec2 center;
    vec4 voronoi_data = voronoiEdges(uv*150.0, 0.9 + clouds_mask * .1, 0.0, 0.0, 0.0, center);
    stars = smoothstep(.0,.2,voronoi_data.a);
    float milky_way = smoothstep(.5,1.0,dot(EYEDIR, normalize(vec3(0.0,EYEDIR.yz))))* texture(milky_way_noise, (EYEDIR.xz+1.0)*.5).r ;
    stars *= milky_way * stars_energy;
  }

  // Soft horizon falloff (~15° instead of ~1.4°) — kills the "tea-cup cap" look
  float is_floor = smoothstep(0.0, -floor_horizon_blend, eye_y);

  // === BASE ATMOSPHERE — sample baked CubemapArray, lerp adjacent keyframes ===
  float layer_f = day_night_cycle * float(BAKE_LAYERS);
  float lo_idx = floor(layer_f);
  float hi_idx = mod(lo_idx + 1.0, float(BAKE_LAYERS));
  float baked_blend_factor = fract(layer_f);
  vec3 atm_a = textureLod(baked_array, vec4(EYEDIR, lo_idx), 0.0).rgb;
  vec3 atm_b = textureLod(baked_array, vec4(EYEDIR, hi_idx), 0.0).rgb;
  vec3 atm_color = mix(atm_a, atm_b, baked_blend_factor);

  // Tri-zone tint per time-of-day (zenith / horizon / nadir colors).
  // TODO: re-confirm composition formula against Unity's GenesisSky shader graph.
  float zenith_w = smoothstep(0.25, 0.75, eye_y);
  float nadir_w = smoothstep(-0.25, -0.75, eye_y);
  float horizon_w = max(0.0, 1.0 - zenith_w - nadir_w);
  vec3 zenith_tint = textureLod(tint_color_cycle_bottom, vec2(day_night_cycle, 0.0), 0.0).rgb;
  vec3 horizon_tint = textureLod(tint_color_cycle_top, vec2(day_night_cycle, 0.0), 0.0).rgb;
  vec3 nadir_tint = textureLod(tint_color_cycle_nadir, vec2(day_night_cycle, 0.0), 0.0).rgb;
  atm_color *= zenith_tint * zenith_w + horizon_tint * horizon_w + nadir_tint * nadir_w;

  // Cloud color: single gradient indexed by time-of-day. Cloud density modulates
  // brightness so denser cloud regions read brighter.
  vec3 cloud_tint = textureLod(clouds_color_cycle, vec2(day_night_cycle, 0.0), 0.0).rgb;
  vec3 cloud_color = cloud_tint * (0.5 + clouds_gradient * 1.5);
  COLOR = mix(atm_color, cloud_color, clouds_mask);

  COLOR += mix(stars * day_night_blend * day_night_blend * day_night_blend * day_night_blend * .5, 0.0, max(clouds_mask, celestial));

  COLOR = dither_interleaved(COLOR,dither_amount,FRAGCOORD.xy);

  // Celestial body color — sun/moon gradients are HDR (peaks ~24 for sun); scale down
  // to keep the disc bright but not blown-out after Godot's filmic tonemap.
  vec3 sun_col = textureLod(sun_color_cycle, vec2(day_night_cycle, 0.0), 0.0).rgb * 0.15;
  vec3 moon_col = textureLod(moon_color_cycle, vec2(day_night_cycle, 0.0), 0.0).rgb * 0.5;
  float moon_influence = smoothstep(0.0, 0.01, moon_mask_size);
  vec3 celestial_col = mix(sun_col, moon_col, moon_influence);

  COLOR += celestial_col * celestial * (1.0 - clouds_mask * 0.9);

  // Atmospheric glow — sun + orbiting secondary, also gated by horizon.
  float second_sun_dot = dot(second_sun_dir, EYEDIR);
  COLOR += sun_col * smoothstep(0.7, 1.7, sun_dot) * sun_opacity * sun_above_horizon * 0.5;
  COLOR += sun_col * smoothstep(0.85, 1.5, second_sun_dot) * sun_opacity * sun_above_horizon * 0.2;

  // Sun radiance halo (rim color) — warm halo around sun direction, gated by gradient
  // (dark at night → no halo; warm at sunrise/sunset).
  float radiance = pow(max(sun_dot, 0.0), sun_radiance_falloff) * (1.0 - eye_dot * 0.5);
  vec3 rim_tint = textureLod(rim_color_cycle, vec2(day_night_cycle, 0.0), 0.0).rgb;
  COLOR += rim_tint * radiance * sun_radiance_intensity * sun_opacity;

  // Floor: TOD-modulated color × depth-based dim. No more flat purple cap.
  vec3 floor_tod = textureLod(floor_color_cycle, vec2(day_night_cycle, 0.0), 0.0).rgb;
  float floor_depth = clamp(-eye_y, 0.0, 1.0);
  vec3 floor_final = floor_tod * mix(1.0, floor_nadir_dim, smoothstep(0.0, 0.7, floor_depth));
  COLOR = mix(COLOR, floor_final, is_floor);

  COLOR = energy * pow(COLOR, vec3(gamma));
  } // end else (runtime mode)
}
