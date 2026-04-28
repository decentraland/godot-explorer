shader_type spatial;
render_mode ambient_light_disabled, cull_back;

#include "dcl_toon.gdshaderinc"

void fragment() {
	vec4 albedo_tex = texture(albedo_texture, UV);
	ALBEDO = albedo_tex.rgb * albedo_color.rgb;

	vec3 emission_tex = texture(emission_texture, UV).rgb;
	// Base floor: avatar is never darker than 2nd shade zone
	// light() adds the excess on top via DIFFUSE_LIGHT
	EMISSION = ALBEDO * SECOND_SHADE_MULT + emission_tex * emission_color.rgb * emission_energy;

	ROUGHNESS = 1.0;
	METALLIC = 0.0;
	SPECULAR = 0.0;
}
