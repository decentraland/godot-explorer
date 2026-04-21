shader_type sky;

#include "./modules/voronoiEdges2d.gdshaderinc"
#include "./modules/rotate2D.gdshaderinc"
#include "./modules/cartesian2polar.gdshaderinc"
#include "./modules/polar2cartesian.gdshaderinc"

uniform sampler2D milky_way_noise : source_color;
uniform float stars_energy = 1.0;

// Cubemap clouds (imported from a 6×1 strip atlas). Sampled with EYEDIR rotated around
// the Y axis by clouds_rotation_y_deg + scroll, so the cloud silhouettes can be aligned
// to a reference (and slowly rotate with clouds_scroll_speed).
// Cloud data cubemap. R = highlight (top of cloud bright, bottom dark). G = silhouette mask.
// NO source_color — these are DATA channels, not color; sRGB decode would crush the values.
uniform samplerCube clouds_cubemap : hint_default_black;
uniform float clouds_rotation_y_deg : hint_range(-180.0, 180.0) = 0.0;
// Squashes the cubemap sampling vertically: <1 pulls clouds toward the horizon (clouds
// appear lower and concentrated near horizon), >1 pushes them up. Match Unity's apparent
// cloud altitude.
uniform float clouds_height_compress : hint_range(0.1, 2.0) = 1.5;

uniform sampler2D sky_gradient : source_color, repeat_disable, hint_default_white;

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

void sky() {
  // Sun and moon directions come from animated DirectionalLight (sky_base.gd pushes uniforms)
  vec3 sun_dir = normalize(sun_direction);
  vec3 moon_dir = normalize(moon_direction);

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
  // Cloud cubemap channel encoding (matches Unity GenesisSkyClouds subgraph):
  //   R = cloud body factor — multiplied by per-TOD HDR cloud color
  //   G = silhouette mask — smoothstep tightens edges so the post-pow cloud_final dark
  //       values don't bleed into a thin fringe at semi-transparent edges
  //   B = highlight factor — multiplied by normalized sun chroma to add a sun-tinted edge
  float clouds_mask = smoothstep(0.5, 0.85, clouds_sample.g);

  // === SUN DISC ===
  // Was 0.3 (too small per user) → 0.5 (too big per user) → 0.35 (target between).
  float sun_angle = acos(clamp(sun_dot, -1.0, 1.0));
  float remapped_sun_size = sun_size * 0.35;
  float sun_disc = step(sun_angle, remapped_sun_size);

  // === SECOND SUN (orbital secondary, pulsates around the sun/moon position) ===
  // Orbit in the plane perpendicular to sun_dir → constant angular distance regardless of
  // sun position. Distance scales with sun_size (same as the disc size) so the orbit
  // radius tracks the animated sun.
  float ss_rotation = second_sun_speed * TIME;
  vec3 ss_up_ref = abs(sun_dir.y) > 0.9 ? vec3(0.0, 0.0, 1.0) : vec3(0.0, 1.0, 0.0);
  vec3 ss_perp1 = normalize(cross(sun_dir, ss_up_ref));
  vec3 ss_perp2 = cross(sun_dir, ss_perp1);
  vec3 rotated_orbit = cos(ss_rotation) * ss_perp1 + sin(ss_rotation) * ss_perp2;
  vec3 second_sun_dir = normalize(sun_dir + rotated_orbit * second_sun_orbit * sun_size);
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

  // Gate celestial rendering by time-of-day, not by the visual sun_dir.y (the imported
  // animation keeps the sun mostly above horizon even at night). day_factor is 1 around
  // noon and 0 around midnight, so sun + glow disappear at night and the moon takes over.
  float day_factor = max(0.0, sin(day_night_cycle * PI));
  // Only the main sun disc gets the moon bite carved; second sun stays whole (otherwise
  // it can disappear behind the crescent shadow as it orbits).
  float sun_with_bite = sun_disc * moon_mask_step;
  float combined_disc = clamp(sun_with_bite + second_sun_disc, 0.0, 1.0);
  float celestial = combined_disc * sun_opacity * sun_opacity * day_factor;

  // 1.0 at midnight (t=0 or t=1), 0.0 at noon (t=0.5). Drives star visibility.
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
  // Use canonical edge order (edge0 < edge1) so it behaves the same on all drivers.
  float is_floor = 1.0 - smoothstep(-floor_horizon_blend, 0.0, eye_y);

  // === BASE SKY — tri-zone gradient (zenith / horizon / nadir) blended by view-direction
  // Y. This IS the sky color in Unity's GenesisSky — there is no Rayleigh layer underneath,
  // just the per-time-of-day color picked from these three gradients.
  float zenith_w = smoothstep(0.25, 0.75, eye_y);
  float nadir_w = smoothstep(-0.25, -0.75, eye_y);
  float horizon_w = max(0.0, 1.0 - zenith_w - nadir_w);
  vec3 zenith_tint = textureLod(tint_color_cycle_bottom, vec2(day_night_cycle, 0.0), 0.0).rgb;
  vec3 horizon_tint = textureLod(tint_color_cycle_top, vec2(day_night_cycle, 0.0), 0.0).rgb;
  vec3 nadir_tint = textureLod(tint_color_cycle_nadir, vec2(day_night_cycle, 0.0), 0.0).rgb;
  vec3 sky_base = zenith_tint * zenith_w + horizon_tint * horizon_w + nadir_tint * nadir_w;

  // Direct port of Unity GenesisSkyClouds + parent compositor.
  // Inner BlendNode mode 17 = Screen (verified from Unity-Technologies/Graphics source):
  //   Out = lerp(Base, 1.0 - (1.0 - Blend) * (1.0 - Base), Opacity)
  // - Base    = R × _CloudsColor (HDR per-TOD cloud tint)
  // - Blend   = B × normalize(_SunColor)
  // - Opacity = _Cloud_Highlights (per-TOD curve, 0.5 → 4)
  // Final cloud color = inner × _Cloud_Opacity. Composited over the sky with another
  // Screen using G as the lerp Opacity.
  vec3 cloud_tint = textureLod(clouds_color_cycle, vec2(day_night_cycle, 0.0), 0.0).rgb;
  vec3 sun_tint = textureLod(sun_color_cycle, vec2(day_night_cycle, 0.0), 0.0).rgb;
  vec3 sun_chroma = normalize(max(sun_tint, vec3(1e-4)));
  vec3 cloud_body = clouds_sample.r * cloud_tint;
  vec3 cloud_highlight = clouds_sample.b * sun_chroma;
  float cloud_highlights = textureLod(cloud_highlights_cycle, vec2(day_night_cycle, 0.0), 0.0).r;
  vec3 inner_screen = vec3(1.0) - (vec3(1.0) - cloud_highlight) * (vec3(1.0) - cloud_body);
  vec3 cloud_color = mix(cloud_body, inner_screen, cloud_highlights) * cloud_opacity;
  // Outer Screen blends cloud onto sky. Apply Unity's exact sRGB→Linear conversion
  // (piecewise per ColorspaceConversionNode source) only to the cloud_color part so the
  // sky stays unchanged at the cloud edges (avoids dark halo).
  vec3 cloud_safe = max(cloud_color, vec3(0.0));
  vec3 lin_lo = cloud_safe / 12.92;
  vec3 lin_hi = pow(max((cloud_safe + 0.055) / 1.055, vec3(1.192092896e-07)), vec3(2.4));
  vec3 cloud_linear = mix(lin_hi, lin_lo, vec3(lessThanEqual(cloud_safe, vec3(0.04045))));
  vec3 outer_screen = vec3(1.0) - (vec3(1.0) - cloud_linear) * (vec3(1.0) - sky_base);
  COLOR = mix(sky_base, outer_screen, clouds_mask);

  COLOR += mix(stars * day_night_blend * day_night_blend * day_night_blend * day_night_blend * .5, 0.0, max(clouds_mask, celestial));

  COLOR = dither_interleaved(COLOR,dither_amount,FRAGCOORD.xy);

  // Sun disc: render as bright white. The sun_color_cycle gradient is used for the
  // rim/glow halo (where color is appropriate), not the disc itself — Unity's sun is
  // a near-white disc, not a tinted one.
  vec3 sun_col = vec3(2.0);
  vec3 moon_tint = textureLod(moon_color_cycle, vec2(day_night_cycle, 0.0), 0.0).rgb;
  vec3 moon_col = moon_tint * 7.0;
  float moon_influence = smoothstep(0.0, 0.01, moon_mask_size);
  vec3 celestial_col = mix(sun_col, moon_col, moon_influence);

  COLOR += celestial_col * celestial * (1.0 - clouds_mask * 0.9);

  // Atmospheric glow — sun + orbiting secondary, gated by day_factor so it disappears
  // at night (the imported sun animation never dips below horizon).
  float second_sun_dot = dot(second_sun_dir, EYEDIR);
  COLOR += sun_col * smoothstep(0.7, 1.7, sun_dot) * sun_opacity * day_factor * 0.5;
  COLOR += sun_col * smoothstep(0.85, 1.5, second_sun_dot) * sun_opacity * day_factor * 0.2;

  // Sun radiance halo (rim color) — warm halo around sun direction, gated by day_factor
  // so it disappears at night.
  float radiance = pow(max(sun_dot, 0.0), sun_radiance_falloff) * (1.0 - eye_dot * 0.5);
  vec3 rim_tint = textureLod(rim_color_cycle, vec2(day_night_cycle, 0.0), 0.0).rgb;
  COLOR += rim_tint * radiance * sun_radiance_intensity * sun_opacity * day_factor;

  // Floor: TOD-modulated color × depth-based dim. No more flat purple cap.
  vec3 floor_tod = textureLod(floor_color_cycle, vec2(day_night_cycle, 0.0), 0.0).rgb;
  float floor_depth = clamp(-eye_y, 0.0, 1.0);
  vec3 floor_final = floor_tod * mix(1.0, floor_nadir_dim, smoothstep(0.0, 0.7, floor_depth));
  COLOR = mix(COLOR, floor_final, is_floor);

  // Final brightness adjust. Lowered overall energy because the env's filmic + glow
  // brightens our linear output more than Unity's URP pipeline brightens its own.
  COLOR = energy * pow(COLOR, vec3(gamma)) * 0.6;
}
