shader_type canvas_item;

uniform float blur_radius : hint_range(0.0, 20.0) = 8.0;
uniform vec4 shadow_color : source_color = vec4(0.0, 0.0, 0.0, 0.4);
uniform float corner_radius_px : hint_range(0.0, 50.0) = 12.0;
uniform float softness : hint_range(0.0, 1.0) = 1.0;
uniform vec2 rect_size = vec2(200.0, 100.0);

// Rounded rectangle SDF in centered coordinates (center at origin)
// half_extents = half size from center to edge
float rounded_box_sdf_centered(vec2 uv_centered, vec2 half_extents, float radius) {
	vec2 corner = half_extents - radius;
	vec2 d = abs(uv_centered) - corner;
	return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0) - radius;
}

void fragment() {
	// Convert UV from 0-1 to pixel coordinates
	vec2 uv_px = UV * rect_size;
	
	// Work in coordinates centered at the rect center so both boxes share the same center
	vec2 center = rect_size * 0.5;
	vec2 uv_centered = uv_px - center;
	
	// Inner solid area: rect_size minus 2*radius on each side, so we have blur on all 4 sides
	// e.g. 100x100 with radius 12 -> inner 76x76, centered -> half_extents (38, 38)
	vec2 inner_half = (rect_size - vec2(corner_radius_px * 2.0)) * 0.5;
	inner_half = max(inner_half, vec2(0.0));
	
	// Outer edge: half_extents = rect_size * 0.5
	vec2 outer_half = rect_size * 0.5;
	
	float dist_inner_px = rounded_box_sdf_centered(uv_centered, inner_half, corner_radius_px);
	float dist_outer_px = rounded_box_sdf_centered(uv_centered, outer_half, corner_radius_px);
	
	float alpha = 0.0;
	
	if (dist_inner_px <= 0.0) {
		// Inside the solid area - full opacity
		alpha = shadow_color.a;
	} else if (dist_outer_px <= 0.0) {
		// Between solid area and outer edge - apply blur transition
		// This creates the blur effect from inner edge to outer edge
		// The blur range is the difference between inner and outer edges
		float blur_range = corner_radius_px;
		// Normalize distance from inner edge (0 at inner edge, 1.0 at outer edge)
		float normalized_dist = dist_inner_px / blur_range;
		normalized_dist = clamp(normalized_dist, 0.0, 1.0);
		alpha = (1.0 - smoothstep(0.0, 1.0, normalized_dist)) * shadow_color.a;
	} else {
		// Outside the outer edge - no shadow
		alpha = 0.0;
	}
	
	// Apply softness to the entire shadow
	alpha *= softness;
	
	COLOR = vec4(shadow_color.rgb, alpha);
}
