shader_type canvas_item;

// Full-rect dim with one rounded-rect hole, in the rect's own pixels.
uniform vec2 rect_size = vec2(1600.0, 720.0);
uniform vec4 hole = vec4(0.0);
uniform float radius = 16.0;

void fragment() {
	vec2 half_size = hole.zw * 0.5;
	vec2 q = abs(UV * rect_size - (hole.xy + half_size)) - half_size + radius;
	float distance_to_hole = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
	COLOR.a *= smoothstep(0.0, 1.5, distance_to_hole);
}
