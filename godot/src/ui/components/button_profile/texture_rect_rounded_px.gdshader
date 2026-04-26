shader_type canvas_item;

// Radio de las esquinas en pixeles
uniform float corner_radius_px : hint_range(0.0, 512.0, 1.0) = 24.0;

float rounded_box_sdf(vec2 p, vec2 size, float radius) {
	vec2 q = abs(p) - size + vec2(radius);
	return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
}

void fragment() {
	vec2 pixel_pos = UV * (1.0 / TEXTURE_PIXEL_SIZE);
	vec2 rect_size = 1.0 / TEXTURE_PIXEL_SIZE;
	vec2 center = rect_size * 0.5;
	vec2 half_size = rect_size * 0.5;
	float dist = rounded_box_sdf(pixel_pos - center, half_size, corner_radius_px);

	vec4 tex_color = texture(TEXTURE, UV);
	float aa = fwidth(dist);
	tex_color.a *= 1.0 - smoothstep(-aa, aa, dist);
	COLOR = tex_color;
}
