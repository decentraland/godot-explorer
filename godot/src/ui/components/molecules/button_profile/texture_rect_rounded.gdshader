shader_type canvas_item;

// Radio de las esquinas en UV (0.5 = círculo completo, menor = esquinas más rectas)
uniform float corner_radius : hint_range(0.0, 0.5, 0.01) = 0.5;

float rounded_box_sdf(vec2 p, vec2 size, float radius) {
	vec2 q = abs(p) - size + vec2(radius);
	return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
}

void fragment() {
	vec2 uv = UV;
	vec2 center = vec2(0.5, 0.5);
	vec2 half_size = vec2(0.5, 0.5);
	float dist = rounded_box_sdf(uv - center, half_size, corner_radius);

	vec4 tex_color = texture(TEXTURE, UV);
	if (dist > 0.0) {
		tex_color.a = 0.0;
	}
	COLOR = tex_color;
}
