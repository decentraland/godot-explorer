shader_type spatial;
render_mode ambient_light_disabled, cull_back, depth_prepass_alpha;

#include "dcl_toon.gdshaderinc"

uniform float alpha_scissor_threshold : hint_range(0.0, 1.0) = 0.5;

void fragment() {
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
