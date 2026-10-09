shader_type spatial;
render_mode unshaded, blend_mix, depth_draw_never, cull_disabled;

// Port of unity-explorer's GhostHologram.shader (AvatarGhostSystem) — the
// spawn/teleport "hologram materialize" effect. Reveal sweeps bottom-up in
// WORLD space (our rigs are cm-scaled; Unity's object-space trick doesn't
// translate).
uniform vec4 fresnel_color : source_color = vec4(0.0, 4.82, 7.13, 0.97);
uniform float fresnel_power = 1.0;
uniform float fresnel_band_density = 4.0;
// Unity runs this HDR (5.0) because bloom carries the glow; without bloom a
// 5x multiply burns every saturated name color to white.
uniform float emission_intensity = 1.5;
uniform float scanline_density = 5.0;
uniform float scanline_speed = -1.0;
uniform float scanlines_alpha = 0.5;
uniform float scanline_noise_min = 0.4;
uniform float flicker_speed = 0.8;
uniform float glitch_intensity = 0.03;
uniform float glitch_density = 12.0;
uniform float glitch_speed = 6.0;
uniform float glitch_threshold = 0.85;
// Driven per-frame by avatar.gd during the spawn reveal.
uniform float reveal_y = -1000.0;

varying vec3 world_pos;

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	// Horizontal glitch jitter (banded by world Y, like Unity's).
	float band = floor(world_pos.y * glitch_density);
	float glitch_seed = fract(sin(band * 127.1 + floor(TIME * glitch_speed)) * 43758.5453);
	float glitch = step(glitch_threshold, glitch_seed) * (glitch_seed - 0.5) * glitch_intensity;
	world_pos.x += glitch;
	VERTEX += (inverse(MODEL_MATRIX) * vec4(world_pos - (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz, 0.0)).xyz;
}

void fragment() {
	// Reveal plane: keep only fragments at/below the sweep.
	if (world_pos.y > reveal_y) {
		discard;
	}
	vec3 view_dir = normalize(CAMERA_POSITION_WORLD - world_pos);
	vec3 normal_ws = normalize(NORMAL);
	float base_fresnel = pow(1.0 - clamp(dot(view_dir, normal_ws), 0.0, 1.0), fresnel_power);

	float bright_band = floor(world_pos.y * fresnel_band_density);
	float band_brightness = fract(sin(bright_band * 91.7) * 43758.5453);
	float banded_fresnel = base_fresnel * band_brightness * band_brightness;

	float scan_y = world_pos.y * scanline_density + TIME * scanline_speed;
	float scanline = smoothstep(0.3, 0.7, fract(scan_y));
	float scan_row = floor(world_pos.y * scanline_density);
	float scan_noise = fract(sin(scan_row * 153.7) * 43758.5453);
	scanline *= scanlines_alpha * (scanline_noise_min + scan_noise * (1.0 - scanline_noise_min));

	float flicker = fract(sin(floor(TIME * flicker_speed) * 127.1) * 43758.5453);
	float intensity = emission_intensity * (0.9 + flicker * 0.1);

	vec3 col = fresnel_color.rgb * (base_fresnel + banded_fresnel + scanline) * intensity;
	// Thin base presence: Unity's look is low-alpha center + HDR bloom; without
	// bloom, keep a faint floor and let fresnel/scanlines carry it.
	float alpha = clamp(0.1 + base_fresnel * 0.8 + banded_fresnel * 0.5 + scanline * 0.6, 0.0, 1.0) * fresnel_color.a;
	ALBEDO = col;
	EMISSION = col;
	ALPHA = alpha;
}
