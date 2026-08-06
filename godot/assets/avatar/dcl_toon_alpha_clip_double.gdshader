shader_type spatial;
render_mode ambient_light_disabled, cull_disabled, depth_prepass_alpha;

#include "dcl_toon.gdshaderinc"

// Own-avatar proximity fade (issue #1814): uniform (object-level) dissolve of
// the local player's avatar when the camera is forced into the head. Driven
// per-instance via set_instance_shader_parameter by own_avatar_proximity_fade.gd.
instance uniform float own_fade : hint_range(0.0, 1.0) = 0.0;

uniform float alpha_scissor_threshold : hint_range(0.0, 1.0) = 0.5;

void fragment() {
	if (own_fade > 0.0) {
		float own_dither = fract(52.9829189 * fract(dot(FRAGCOORD.xy, vec2(0.06711056, 0.00583715))));
		if (own_dither < own_fade) {
			discard;
		}
	}
	vec4 albedo_tex = texture(albedo_texture, UV);
	ALBEDO = albedo_tex.rgb * albedo_color.rgb;
	ALPHA = albedo_tex.a * albedo_color.a;
	ALPHA_SCISSOR_THRESHOLD = alpha_scissor_threshold;

	vec3 emission_tex = texture(emission_texture, UV).rgb;
	EMISSION = ALBEDO * SECOND_SHADE_MULT + emission_tex * emission_color.rgb * emission_energy;

	ROUGHNESS = 1.0;
	METALLIC = 0.0;
	SPECULAR = 0.0;
}
