shader_type spatial;
render_mode ambient_light_disabled, blend_mix, cull_back;

#include "dcl_toon.gdshaderinc"

void fragment() {
	vec4 albedo_tex = texture(albedo_texture, UV);
	ALBEDO = albedo_tex.rgb * albedo_color.rgb;
	ALPHA = albedo_tex.a * albedo_color.a;

	vec3 emission_tex = texture(emission_texture, UV).rgb;
	EMISSION = ALBEDO * SECOND_SHADE_MULT + emission_tex * emission_color.rgb * emission_energy;

	ROUGHNESS = 1.0;
	METALLIC = 0.0;
	SPECULAR = 0.0;
}
