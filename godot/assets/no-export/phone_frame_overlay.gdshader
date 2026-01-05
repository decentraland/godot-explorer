shader_type canvas_item;

uniform bool is_ios = true;
uniform bool is_portrait = false;
uniform vec4 frame_color : source_color = vec4(0.0, 0.0, 0.0, 1.0);

// iPhone 14 Pro dimensions (ratios relative to screen)
const float CORNER_RADIUS_RATIO = 0.04; // Corner radius as ratio of min dimension
const float DYNAMIC_ISLAND_WIDTH_RATIO = 0.27; // Width of dynamic island
const float DYNAMIC_ISLAND_HEIGHT_RATIO = 0.035; // Height of dynamic island
const float DYNAMIC_ISLAND_TOP_OFFSET_RATIO = 0.012; // Distance from top
const float DYNAMIC_ISLAND_CORNER_RATIO = 0.4; // Corner radius of dynamic island (relative to its height)
const float HOME_INDICATOR_WIDTH_RATIO = 0.35; // Width of home indicator
const float HOME_INDICATOR_HEIGHT_RATIO = 0.005; // Height of home indicator
const float HOME_INDICATOR_BOTTOM_OFFSET_RATIO = 0.008; // Distance from bottom

// Android dimensions
const float ANDROID_STATUS_BAR_RATIO = 0.033; // Status bar height ratio
const float ANDROID_NAV_BAR_RATIO = 0.066; // Navigation bar height ratio (portrait)
const float ANDROID_NAV_BAR_LANDSCAPE_RATIO = 0.033; // Navigation bar height ratio (landscape)

float rounded_box_sdf(vec2 p, vec2 size, float radius) {
	vec2 q = abs(p) - size + vec2(radius);
	return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
}

float pill_sdf(vec2 p, vec2 size) {
	float radius = min(size.x, size.y);
	return rounded_box_sdf(p, size, radius);
}

void fragment() {
	vec2 screen_size = 1.0 / SCREEN_PIXEL_SIZE;
	vec2 uv = SCREEN_UV;
	vec2 pixel = uv * screen_size;

	float min_dim = min(screen_size.x, screen_size.y);
	float corner_radius = min_dim * CORNER_RADIUS_RATIO;

	// Start with fully transparent
	float alpha = 0.0;

	// Rounded corners - check if we're outside the rounded rectangle
	vec2 center = screen_size * 0.5;
	vec2 half_size = screen_size * 0.5;
	float corner_dist = rounded_box_sdf(pixel - center, half_size, corner_radius);
	if (corner_dist > 0.0) {
		alpha = 1.0;
	}

	if (is_ios) {
		// Dynamic Island
		float island_width, island_height, island_x, island_y;

		if (is_portrait) {
			island_width = screen_size.x * DYNAMIC_ISLAND_WIDTH_RATIO;
			island_height = screen_size.y * DYNAMIC_ISLAND_HEIGHT_RATIO;
			island_x = screen_size.x * 0.5;
			island_y = screen_size.y * DYNAMIC_ISLAND_TOP_OFFSET_RATIO + island_height * 0.5;
		} else {
			// Landscape - dynamic island on the left side
			island_width = screen_size.x * DYNAMIC_ISLAND_HEIGHT_RATIO;
			island_height = screen_size.y * DYNAMIC_ISLAND_WIDTH_RATIO;
			island_x = screen_size.x * DYNAMIC_ISLAND_TOP_OFFSET_RATIO + island_width * 0.5;
			island_y = screen_size.y * 0.5;
		}

		vec2 island_center = vec2(island_x, island_y);
		vec2 island_half_size = vec2(island_width, island_height) * 0.5;
		float island_dist = pill_sdf(pixel - island_center, island_half_size);

		if (island_dist < 0.0) {
			alpha = 1.0;
		}

		// Home indicator bar
		float home_width, home_height, home_x, home_y;

		if (is_portrait) {
			home_width = screen_size.x * HOME_INDICATOR_WIDTH_RATIO;
			home_height = screen_size.y * HOME_INDICATOR_HEIGHT_RATIO;
			home_x = screen_size.x * 0.5;
			home_y = screen_size.y * (1.0 - HOME_INDICATOR_BOTTOM_OFFSET_RATIO) - home_height * 0.5;
		} else {
			// Landscape - home indicator at bottom center
			home_width = screen_size.x * HOME_INDICATOR_WIDTH_RATIO;
			home_height = screen_size.y * HOME_INDICATOR_HEIGHT_RATIO * 2.0;
			home_x = screen_size.x * 0.5;
			home_y = screen_size.y * (1.0 - HOME_INDICATOR_BOTTOM_OFFSET_RATIO * 2.0) - home_height * 0.5;
		}

		vec2 home_center = vec2(home_x, home_y);
		vec2 home_half_size = vec2(home_width, home_height) * 0.5;
		float home_dist = pill_sdf(pixel - home_center, home_half_size);

		// Draw home indicator as a dark gray bar (not fully opaque)
		if (home_dist < 0.0) {
			alpha = max(alpha, 0.3);
		}
	} else {
		// Android - just show status bar and nav bar areas with slight tint
		float status_bar_height = screen_size.y * ANDROID_STATUS_BAR_RATIO;
		float nav_bar_height;

		if (is_portrait) {
			nav_bar_height = screen_size.y * ANDROID_NAV_BAR_RATIO;
		} else {
			nav_bar_height = screen_size.y * ANDROID_NAV_BAR_LANDSCAPE_RATIO;
		}

		// Status bar at top
		if (pixel.y < status_bar_height) {
			alpha = max(alpha, 0.15);
		}

		// Nav bar at bottom
		if (pixel.y > screen_size.y - nav_bar_height) {
			alpha = max(alpha, 0.15);
		}
	}

	COLOR = vec4(frame_color.rgb, alpha * frame_color.a);
}
