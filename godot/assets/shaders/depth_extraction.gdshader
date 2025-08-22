shader_type spatial;
render_mode unshaded, depth_test_disabled, depth_draw_never, cull_front;

uniform sampler2D depth_texture : hint_depth_texture, filter_linear_mipmap;

void vertex() {
	// Make this a fullscreen quad
	POSITION = vec4(VERTEX.xy * 2.0, 0.0, 1.0);
}

void fragment() {
	ALBEDO = texture(depth_texture, SCREEN_UV).rgb;
}
