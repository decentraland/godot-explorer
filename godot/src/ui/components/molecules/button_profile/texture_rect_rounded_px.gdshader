shader_type canvas_item;

// Corner radius in actual screen pixels, independent of texture size or aspect ratio
uniform float corner_radius_px : hint_range(0.0, 512.0, 1.0) = 24.0;

uniform bool round_top_left = true;
uniform bool round_top_right = true;
uniform bool round_bottom_left = true;
uniform bool round_bottom_right = true;

varying vec2 local_pos;

float rounded_box_sdf(vec2 p, vec2 half_size, float r_tl, float r_tr, float r_bl, float r_br) {
	// Select radius based on quadrant (y increases downward in screen space)
	float r = p.x > 0.0
		? (p.y > 0.0 ? r_br : r_tr)
		: (p.y > 0.0 ? r_bl : r_tl);
	vec2 q = abs(p) - half_size + r;
	return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

void vertex() {
	local_pos = VERTEX;
}

void fragment() {
	vec2 rect_size = vec2(
		abs(dFdx(local_pos.x) / dFdx(UV.x)),
		abs(dFdy(local_pos.y) / dFdy(UV.y))
	);
	vec2 center = rect_size * 0.5;

	float r = corner_radius_px;
	float dist = rounded_box_sdf(
		local_pos - center,
		center,
		round_top_left     ? r : 0.0,
		round_top_right    ? r : 0.0,
		round_bottom_left  ? r : 0.0,
		round_bottom_right ? r : 0.0
	);

	vec4 tex_color = texture(TEXTURE, UV);
	float aa = fwidth(dist);
	tex_color.a *= 1.0 - smoothstep(-aa, aa, dist);
	COLOR = tex_color;
}
