shader_type canvas_item;

uniform float blur_amount : hint_range(0.0, 8.0) = 2.0;

uniform sampler2D screen_texture : hint_screen_texture, filter_linear_mipmap;

void fragment() {
	vec4 color = textureLod(screen_texture, SCREEN_UV, blur_amount);
	COLOR = color;
}