shader_type sky;

#include "./modules/voronoiEdges2d.gdshaderinc"

uniform sampler2D milky_way_noise : source_color;
uniform float stars_energy = 1.0;

// Cubemap clouds (imported from a 6×1 strip atlas). Sampled with EYEDIR rotated around
// the Y axis by clouds_rotation_y_deg + scroll, so the cloud silhouettes can be aligned
// to a reference (and slowly rotate with clouds_scroll_speed).
// Cloud data cubemap. R = cloud body. G = silhouette mask. B = sun-tinted highlight.
// NO source_color — these are DATA channels, not color; sRGB decode would crush the values.
uniform samplerCube clouds_cubemap : hint_default_black;
uniform float clouds_rotation_y_deg : hint_range(-180.0, 180.0) = 0.0;
// Squashes the cubemap sampling vertically: <1 pulls clouds toward the horizon (clouds
// appear lower and concentrated near horizon), >1 pushes them up. Match Unity's apparent
// cloud altitude.
uniform float clouds_height_compress : hint_range(0.1, 2.0) = 1.5;

// Cloud color per time-of-day, HDR (values up to ~2 around noon). NO `source_color` —
// the gradient is already linear (use_hdr=true on the GradientTexture1D), and the hint
// would re-apply sRGB→linear, crushing the >1 values.
uniform sampler2D clouds_color_cycle : repeat_disable, hint_default_white;

uniform float energy : hint_range(0.0, 10.0, 0.1) = 1.0;
uniform float gamma : hint_range(0.0,4.0) = 1.0;

// Vertical tint zones. Legacy names kept for back-compat with VR lobby material:
//   tint_color_cycle_top    → applied at horizon (mid-band)
//   tint_color_cycle_bottom → applied at zenith (looking up)
//   tint_color_cycle_nadir  → applied below horizon (looking down)
uniform sampler2D tint_color_cycle_top : source_color, repeat_disable, hint_default_white;
uniform sampler2D tint_color_cycle_bottom : source_color, repeat_disable, hint_default_white;
uniform sampler2D tint_color_cycle_nadir : source_color, repeat_disable, hint_default_white;

// HDR gradients (use_hdr=true). NO source_color — values are already linear floats.
uniform sampler2D sun_color_cycle : repeat_disable, hint_default_white;
uniform sampler2D moon_color_cycle : source_color, repeat_disable, hint_default_white;
uniform sampler2D rim_color_cycle : repeat_disable, hint_default_white;
uniform float sun_radiance_intensity : hint_range(0.0, 5.0) = 0.05;
uniform float sun_radiance_falloff : hint_range(0.5, 32.0) = 12.0;

// Cloud highlights curve (Unity cloudsHighlightsIntensity, ranges ~0.5 → 4 across day).
// Lerp factor between cloud body (R × cloud_tint) and chromatic highlight (B × sun_chroma).
// Stored as 1D HDR gradient indexed by day_night_cycle, value read from R channel.
uniform sampler2D cloud_highlights_cycle : repeat_disable, hint_default_white;
uniform float cloud_opacity : hint_range(0.0, 2.0) = 1.0;

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

// Second sun orbital parameters
uniform float second_sun_speed : hint_range(0.0, 2.0) = 0.1;
uniform float second_sun_orbit : hint_range(0.0, 5.0) = 0.1;
uniform float second_sun_size_factor : hint_range(0.01, 4.0) = 0.1;

// Moon mask 2D offset
uniform vec2 moon_mask_offset = vec2(0.01, -0.01);

// Floor: soft horizon falloff for the below-horizon "cotton field" cover.
uniform float floor_horizon_blend : hint_range(0.01, 0.5) = 0.15;

// Asymmetric horizon bands — Unity SkyboxRenderController.cs writes the exact same
// per-TOD HDR gradients we already have (Gradient_horizon ↔ _HorizonColor, Gradient_nadir
// ↔ _NadirColor). So sample horizon_tint above (cloud-side) and nadir_tint below
// (water-side). Hard limit at eye_y = 0.
uniform float horizon_above_opacity : hint_range(0.0, 3.0) = 1.0;
uniform float horizon_above_spread : hint_range(5.0, 200.0) = 65.0;
uniform float horizon_below_opacity : hint_range(0.0, 3.0) = 1.0;
uniform float horizon_below_spread : hint_range(5.0, 200.0) = 55.0;

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

float interleaved_gradient_noise(vec2 n) {
    float f = 0.06711056 * n.x + 0.00583715 * n.y;
    return fract(52.9829189 * fract(f));
}

vec3 dither_interleaved(vec3 rgb, float levels, vec2 frag_coord) {
    float noise = interleaved_gradient_noise(frag_coord.xy);
    noise = noise - 0.5;
    return rgb + noise / (levels - 1.0);
}

// Hash + value-noise FBM for the procedural cloud field below the horizon.
float hash21(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float value_noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash21(i + vec2(0.0, 0.0)), hash21(i + vec2(1.0, 0.0)), u.x),
        mix(hash21(i + vec2(0.0, 1.0)), hash21(i + vec2(1.0, 1.0)), u.x),
        u.y
    );
}

float cloud_field_fbm(vec2 p) {
    float v = 0.0;
    float a = 0.5;
    for (int i = 0; i < 4; i++) {
        v += a * value_noise(p);
        p *= 2.13;
        a *= 0.55;
    }
    return v;
}

vec3 srgb_to_linear(vec3 c) {
    vec3 safe = max(c, vec3(0.0));
    vec3 lo = safe / 12.92;
    vec3 hi = pow(max((safe + 0.055) / 1.055, vec3(1.192092896e-07)), vec3(2.4));
    return mix(hi, lo, vec3(lessThanEqual(safe, vec3(0.04045))));
}

void sky() {
  // sun_direction / moon_direction come pre-normalized from the GDScript side.
  vec3 sun_dir = sun_direction;
  vec3 moon_dir = moon_direction;

  float sun_dot = dot(sun_dir, EYEDIR);
  float eye_y = EYEDIR.y;
  float eye_dot = abs(eye_y);

  // Cubemap-sampled clouds. Rotate EYEDIR around Y by user-set offset + cheap scroll,
  // then squash Y so clouds sit closer to the horizon.
  vec3 rd = EYEDIR;
  float angle = TIME * clouds_scroll_speed * TAU + radians(clouds_rotation_y_deg);
  float cs = cos(angle);
  float sn = sin(angle);
  vec3 cloud_eye = vec3(cs * rd.x + sn * rd.z, rd.y * clouds_height_compress, -sn * rd.x + cs * rd.z);

  vec3 clouds_sample = textureLod(clouds_cubemap, cloud_eye, 0.0).rgb;
  // Cloud cubemap channel encoding (matches Unity GenesisSkyClouds subgraph):
  //   R = cloud body factor — multiplied by per-TOD HDR cloud color
  //   G = silhouette mask — smoothstep tightens edges to avoid dark fringe
  //   B = highlight factor — multiplied by normalized sun chroma
  float clouds_mask = smoothstep(0.5, 0.85, clouds_sample.g);

  // === SUN DISC === step(acos(x), s) ↔ step(cos(s), x): same result, no acos.
  float remapped_sun_size = sun_size * 0.35;
  float sun_disc = step(cos(remapped_sun_size), sun_dot);

  // === SECOND SUN (orbital secondary, pulsates around the sun/moon position) ===
  float ss_rotation = second_sun_speed * TIME;
  vec3 ss_up_ref = abs(sun_dir.y) > 0.9 ? vec3(0.0, 0.0, 1.0) : vec3(0.0, 1.0, 0.0);
  vec3 ss_perp1 = normalize(cross(sun_dir, ss_up_ref));
  vec3 ss_perp2 = cross(sun_dir, ss_perp1);
  vec3 rotated_orbit = cos(ss_rotation) * ss_perp1 + sin(ss_rotation) * ss_perp2;
  vec3 second_sun_dir = normalize(sun_dir + rotated_orbit * second_sun_orbit * sun_size);
  float second_sun_dot = dot(second_sun_dir, EYEDIR);
  float base_ss_size = remapped_sun_size * second_sun_size_factor;
  float ss_pulse = (cos(ss_rotation) + 1.0) * 0.5;
  float total_ss_size = base_ss_size + base_ss_size * ss_pulse;
  float second_sun_disc = step(cos(total_ss_size), second_sun_dot);

  // === MOON MASK (carves the crescent C bite out of the sun + second sun discs) ===
  vec3 moon_mask_dir = normalize(
      sun_dir + vec3(moon_mask_offset.x, moon_mask_offset.y, 0.0) * remapped_sun_size * 40.0
  );
  float moon_mask_dot = dot(moon_mask_dir, EYEDIR);
  float moon_mask_threshold = moon_mask_size * remapped_sun_size * 4.5;
  // step(threshold, angle) ↔ step(dot, cos(threshold)): "outside the bite" stays 1.
  float moon_active = step(0.15, moon_mask_size);
  float moon_mask_step = mix(1.0, step(moon_mask_dot, cos(moon_mask_threshold)), moon_active);

  // Gate celestial rendering by time-of-day, not visual sun_dir.y.
  float day_factor = max(0.0, sin(day_night_cycle * PI));
  float sun_with_bite = sun_disc * moon_mask_step;
  float combined_disc = clamp(sun_with_bite + second_sun_disc, 0.0, 1.0);
  float celestial = combined_disc * sun_opacity * sun_opacity * day_factor;

  // 1.0 at midnight, 0.0 at noon. Drives star visibility.
  float day_night_blend = 1.0 - sin(day_night_cycle * PI);

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

  // Soft horizon falloff (~15° instead of ~1.4°) — kills the "tea-cup cap" look.
  float is_floor = 1.0 - smoothstep(-floor_horizon_blend, 0.0, eye_y);

  // === BASE SKY — tri-zone gradient (zenith / horizon / nadir) blended by view-direction Y.
  float zenith_w = smoothstep(0.25, 0.75, eye_y);
  float nadir_w = smoothstep(-0.25, -0.75, eye_y);
  float horizon_w = max(0.0, 1.0 - zenith_w - nadir_w);
  vec3 zenith_tint = textureLod(tint_color_cycle_bottom, vec2(day_night_cycle, 0.0), 0.0).rgb;
  vec3 horizon_tint = textureLod(tint_color_cycle_top, vec2(day_night_cycle, 0.0), 0.0).rgb;
  vec3 nadir_tint = textureLod(tint_color_cycle_nadir, vec2(day_night_cycle, 0.0), 0.0).rgb;
  vec3 sky_base = zenith_tint * zenith_w + horizon_tint * horizon_w + nadir_tint * nadir_w;

  // Direct port of Unity GenesisSkyClouds + parent compositor (Screen blend mode 17).
  // Inner: lerp(R*cloud_tint, screen(B*sun_chroma, R*cloud_tint), cloud_highlights).
  // Outer: lerp(sky, screen(srgb_to_linear(cloud), sky), G mask).
  vec3 cloud_tint = textureLod(clouds_color_cycle, vec2(day_night_cycle, 0.0), 0.0).rgb;
  vec3 sun_tint = textureLod(sun_color_cycle, vec2(day_night_cycle, 0.0), 0.0).rgb;
  vec3 sun_chroma = normalize(max(sun_tint, vec3(1e-4)));
  vec3 cloud_body = clouds_sample.r * cloud_tint;
  vec3 cloud_highlight = clouds_sample.b * sun_chroma;
  float cloud_highlights = textureLod(cloud_highlights_cycle, vec2(day_night_cycle, 0.0), 0.0).r;
  vec3 inner_screen = vec3(1.0) - (vec3(1.0) - cloud_highlight) * (vec3(1.0) - cloud_body);
  vec3 cloud_color = mix(cloud_body, inner_screen, cloud_highlights) * cloud_opacity;
  vec3 cloud_linear = srgb_to_linear(cloud_color);
  vec3 outer_screen = vec3(1.0) - (vec3(1.0) - cloud_linear) * (vec3(1.0) - sky_base);
  COLOR = mix(sky_base, outer_screen, clouds_mask);

  COLOR += mix(stars * day_night_blend * day_night_blend * day_night_blend * day_night_blend * .5, 0.0, max(clouds_mask, celestial));

  // Sun disc: render as bright white. sun_color_cycle drives the rim/glow halo, not the disc.
  vec3 sun_col = vec3(2.0);
  vec3 moon_tint = textureLod(moon_color_cycle, vec2(day_night_cycle, 0.0), 0.0).rgb;
  vec3 moon_col = moon_tint * 7.0;
  float moon_influence = smoothstep(0.0, 0.01, moon_mask_size);
  vec3 celestial_col = mix(sun_col, moon_col, moon_influence);

  COLOR += celestial_col * celestial * (1.0 - clouds_mask * 0.9);

  // Atmospheric glow — sun + orbiting secondary, gated by day_factor.
  COLOR += sun_col * smoothstep(0.7, 1.7, sun_dot) * sun_opacity * day_factor * 0.5;
  COLOR += sun_col * smoothstep(0.85, 1.5, second_sun_dot) * sun_opacity * day_factor * 0.2;

  // Sun radiance halo (rim color) — warm halo around sun direction, gated by day_factor.
  float radiance = pow(max(sun_dot, 0.0), sun_radiance_falloff) * (1.0 - eye_dot * 0.5);
  vec3 rim_tint = textureLod(rim_color_cycle, vec2(day_night_cycle, 0.0), 0.0).rgb;
  COLOR += rim_tint * radiance * sun_radiance_intensity * sun_opacity * day_factor;

  // Below-horizon "cotton field" cloud cover. Heightfield FBM puffs + cheap one-tap shadow.
  // Whole block gated to the lower hemisphere — saves ~36 noise samples + sRGB conv on the
  // upper half of every frame.
  if (eye_y < 0.05) {
    vec2 plane_uv = EYEDIR.xz / max(-eye_y, 0.05) * 0.18;
    plane_uv += vec2(TIME * 0.004, TIME * 0.0025);
    float h_c = cloud_field_fbm(plane_uv);
    float h_top = cloud_field_fbm(plane_uv * 3.7);
    vec2 sun_xz = normalize(vec2(sun_dir.x, sun_dir.z) + 1e-5);
    float h_neighbor = cloud_field_fbm(plane_uv - sun_xz * 0.10);
    float occlusion = smoothstep(0.0, 0.35, h_neighbor - h_c);
    float shade = 1.0 - occlusion * 0.55;
    float puffs = smoothstep(0.35, 0.75, h_c);
    vec3 field_color = cloud_tint * (0.45 + puffs * 0.55 + h_top * 0.25) * shade;
    vec3 field_linear = srgb_to_linear(field_color);
    float field_alpha = is_floor * smoothstep(0.0, 0.25, -eye_y);
    COLOR = mix(COLOR, field_linear, field_alpha);
  }

  // Asymmetric horizon bands — hard limit at eye_y = 0:
  //   ABOVE (cloud-side): brighten with horizon_tint, narrow exponential falloff
  //   BELOW (water-side): mix toward nadir_tint, narrow exponential falloff
  // step() gates each band to its own side; without it exp(0)=1 contaminates the wrong side.
  float above_gate = step(0.0, eye_y);
  float below_gate = 1.0 - above_gate;
  float above_band = above_gate * exp(-max(eye_y, 0.0) * horizon_above_spread);
  float below_band = below_gate * exp(-max(-eye_y, 0.0) * horizon_below_spread);
  COLOR += horizon_tint * above_band * horizon_above_opacity;
  COLOR = mix(COLOR, nadir_tint, below_band * horizon_below_opacity);

  // Dither AFTER all additive contributions so the noise covers the whole pipeline output.
  COLOR = dither_interleaved(COLOR, dither_amount, FRAGCOORD.xy);

  // Final brightness adjust. Lowered overall energy because the env's filmic + glow
  // brightens our linear output more than Unity's URP pipeline brightens its own.
  COLOR = energy * pow(COLOR, vec3(gamma)) * 0.6;
}
