shader_type spatial;

uniform bool mix_mask = true;
uniform vec4 material_color ;
uniform vec4 mask_color ;
uniform sampler2D base_texture : hint_default_white;
uniform sampler2D mask_texture : hint_default_white;

void fragment() {
	vec4 base = texture(base_texture, UV);
	vec4 mask = texture(mask_texture, UV);

	float color_amt = mask.r * mask.a * mask_color.r;
	vec4 mixed_color = mix(material_color, vec4(1.0), color_amt);
	vec4 final_color = mixed_color * base;
	
	ALBEDO = final_color.rgb;
	ALPHA = final_color.a;
	ALPHA_SCISSOR_THRESHOLD = 0.5;
}
