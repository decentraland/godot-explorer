shader_type spatial;
render_mode ambient_light_disabled, cull_back, depth_prepass_alpha;

#include "dcl_toon.gdshaderinc"

uniform vec4 material_color;
uniform vec4 mask_color;
uniform sampler2D base_texture : hint_default_white;
uniform sampler2D mask_texture : hint_default_white;

void fragment() {
	vec4 base = texture(base_texture, UV);
	vec4 mask = texture(mask_texture, UV);

	float color_amt = mask.r * mask.a * mask_color.r;
	vec4 mixed_color = mix(material_color, vec4(1.0), color_amt);
	vec4 final_color = mixed_color * base;

	ALBEDO = final_color.rgb;
	EMISSION = final_color.rgb * SECOND_SHADE_MULT;
	ALPHA = final_color.a;
	ALPHA_SCISSOR_THRESHOLD = 0.5;
	ROUGHNESS = 1.0;
	METALLIC = 0.0;
	SPECULAR = 0.0;
}
