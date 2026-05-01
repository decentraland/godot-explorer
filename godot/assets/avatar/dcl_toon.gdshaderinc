// DCL Toon Shader - Shared logic
// Port of Unity's DCL/DCL_Toon (Universal Toon Shader variant)
// 3-zone toon diffuse with shadow reception
//
// In Unity, avatars look the same day/night — only shadow direction changes.
// So we ignore LIGHT_COLOR and output a fixed shade multiplier.
// Godot multiplies DIFFUSE_LIGHT * ALBEDO after light() runs.

// --- Texture uniforms ---
uniform sampler2D albedo_texture : source_color, hint_default_white;
uniform vec4 albedo_color = vec4(1.0);
uniform sampler2D emission_texture : source_color, hint_default_white;
uniform vec4 emission_color = vec4(0.0, 0.0, 0.0, 1.0);
uniform float emission_energy : hint_range(0.0, 16.0) = 5.0;

// --- Toon shading parameters (hardcoded from Unity deployed values) ---
const float BASE_COLOR_STEP = 0.2;
const float BASE_SHADE_FEATHER = 0.02;
const float SHADE_COLOR_STEP = 0.2;
const float SHADE_FEATHER_2ND = 0.05;
const float LIT_MULT = 0.5;
const float FIRST_SHADE_MULT = 0.949 * 0.5;
const float SECOND_SHADE_MULT = 0.8 * 0.5;

// --- Specular ---
const float HIGHCOLOR_POWER = 0.15;

// --- Rim light ---
const float RIMLIGHT_POWER = 0.2;

void light() {
	float n_dot_l = dot(NORMAL, LIGHT);
	float half_lambert = 0.5 * n_dot_l + 0.5;

	// Shadow mask (uses ATTENUATION for cast shadow reception)
	float shadow_mask = smoothstep(
		BASE_COLOR_STEP - BASE_SHADE_FEATHER,
		BASE_COLOR_STEP,
		half_lambert * ATTENUATION
	);

	float shade_zone_2 = smoothstep(
		SHADE_COLOR_STEP - SHADE_FEATHER_2ND,
		SHADE_COLOR_STEP,
		half_lambert * ATTENUATION
	);

	// 3-zone shade multiplier — no LIGHT_COLOR (avatar is light-independent)
	float shade_factor = mix(SECOND_SHADE_MULT, FIRST_SHADE_MULT, shade_zone_2);
	float final_shade = mix(shade_factor, LIT_MULT, shadow_mask);

	// Subtract the floor (SECOND_SHADE_MULT is baked into EMISSION)
	float excess = max(final_shade - SECOND_SHADE_MULT, 0.0);
	DIFFUSE_LIGHT = max(DIFFUSE_LIGHT, vec3(excess));

	// Toon specular (Blinn-Phong, hard-edge)
	vec3 half_dir = normalize(LIGHT + VIEW);
	float n_dot_h = max(dot(NORMAL, half_dir), 0.0);
	float spec = pow(n_dot_h, 128.0) * HIGHCOLOR_POWER;
	float spec_mask = smoothstep(0.45, 0.55, spec) * shadow_mask;

	// Rim light (soft Fresnel gradient, masked by light direction)
	float rim_dot = 1.0 - max(dot(NORMAL, VIEW), 0.0);
	float rim = pow(rim_dot, 5.0) * RIMLIGHT_POWER;
	float rim_dir_mask = smoothstep(0.0, 0.3, n_dot_l * ATTENUATION);

	SPECULAR_LIGHT = max(SPECULAR_LIGHT, vec3(spec_mask + rim * rim_dir_mask));
}
