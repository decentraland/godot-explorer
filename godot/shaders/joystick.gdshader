shader_type canvas_item;

// Orb-styled virtual joystick (matches the HUD button aesthetic): a radial-gradient fill at
// ~45% opacity, a thin #DFD0FF stroke on the base, and a lilac inner-shadow "sheen" that gives
// each circle a 3D sphere look. The moving thumb leaves a trail of smaller circles that fade.

uniform vec2 joystick_position;
uniform vec2 tip_position;
uniform int state;

uniform float tip_radius;
uniform float tip_center_radius;
uniform float bubble_radius;
uniform float max_bubble_distance;
uniform float max_chain_length;

varying vec2 pixel_pos;

const int link_amount = 6;
const float link_max_size_at = 0.9;

// Palette (from Figma). ORB_INNER/OUTER = radial gradient stops; LILAC = inner shadow; OUTLINE = stroke.
const vec3 ORB_INNER = vec3(0.1451, 0.1020, 0.2314);  // #251A3B (radial center)
const vec3 ORB_OUTER = vec3(0.0863, 0.0824, 0.0941);  // #161518 (radial edge)
const vec3 ORB_LILAC = vec3(0.8196, 0.6784, 1.0);     // #D1ADFF (inner-shadow sheen)
const vec3 ORB_OUTLINE = vec3(0.8745, 0.8157, 1.0);   // #DFD0FF (stroke)

void vertex() {
	pixel_pos = VERTEX;
}

// Alpha-over composite: src on top of dst (both straight/non-premultiplied rgba).
vec4 over(vec4 src, vec4 dst) {
	float a = src.a + dst.a * (1.0 - src.a);
	if (a <= 0.0) {
		return vec4(0.0);
	}
	vec3 rgb = (src.rgb * src.a + dst.rgb * dst.a * (1.0 - src.a)) / a;
	return vec4(rgb, a);
}

// Radial-gradient orb with a top-left inner-shadow sheen. fill_alpha = overall opacity;
// outline > 0 adds the thin #DFD0FF ring (base only).
vec4 orb(vec2 p, vec2 center, float radius, float fill_alpha, float outline) {
	float dist = distance(p, center);
	float feather = 1.5;
	float cov = 1.0 - smoothstep(radius - feather, radius, dist);

	float t = clamp(dist / radius, 0.0, 1.0);
	vec3 col = mix(ORB_INNER, ORB_OUTER, t);
	float a = fill_alpha * cov;

	// Inner shadow: a thin lilac rim biased to the top-left (the shadow offset is +x/+y, so the
	// inset sheen lands on the opposite inner edge) — the subtle sphere highlight.
	float rim = smoothstep(radius - 5.0, radius - 1.0, dist) * cov;
	vec2 dir = (p - center) / max(dist, 0.001);
	float bias = clamp(dot(dir, vec2(-0.7071, -0.7071)), 0.0, 1.0);
	float sheen = rim * bias * 0.5;
	col = mix(col, ORB_LILAC, sheen);
	a = max(a, sheen);

	vec4 result = vec4(col, a);

	if (outline > 0.0) {
		float aa = 0.6;
		float ring = clamp(
			(1.0 - smoothstep(radius - aa, radius + aa, dist))
			- (1.0 - smoothstep(radius - 1.0 - aa, radius - 1.0 + aa, dist)),
			0.0,
			1.0
		);
		result = over(vec4(ORB_OUTLINE, ring), result);
	}
	return result;
}

// The resting center thumb: a translucent lilac disc (#DFD0FF @ 50%) with a solid #DFD0FF stroke.
vec4 thumb_dot(vec2 p, vec2 center, float radius) {
	float dist = distance(p, center);
	float feather = 1.5;
	float cov = 1.0 - smoothstep(radius - feather, radius, dist);
	vec4 result = vec4(ORB_OUTLINE, 0.5 * cov);
	float aa = 0.6;
	float ring = clamp(
		(1.0 - smoothstep(radius - aa, radius + aa, dist))
		- (1.0 - smoothstep(radius - 1.0 - aa, radius - 1.0 + aa, dist)),
		0.0,
		1.0
	);
	result = over(vec4(ORB_OUTLINE, ring), result);
	return result;
}

void fragment() {
	vec2 control_size = pixel_pos / UV;
	vec2 canvas_point = UV * control_size;
	vec2 tip_limited_position = tip_position;
	if (length(tip_limited_position) > max_chain_length) {
		tip_limited_position = normalize(tip_limited_position) * max_chain_length;
	}

	vec4 acc = vec4(0.0);

	if (state == 0) {
		// Resting (Default): the 170px base orb + a lilac center thumb (#DFD0FF @ 50% + stroke).
		vec2 c = joystick_position + tip_limited_position;
		acc = over(orb(canvas_point, c, 85.0, 0.45, 1.0), acc);
		acc = over(thumb_dot(canvas_point, c, 32.0), acc);
	} else if (state == 1) {
		// Fixed comet tail behind the thumb: diameters 32, 32, 18, 12, 8 (the 32 only twice), with a
		// 40 px EDGE-to-edge gap between bubbles (20 px from the thumb to the first). Anchored to the
		// thumb, clamped so it never overshoots the origin. Opacity fades 45% (thumb) -> 10% (tail).
		float chain = length(tip_limited_position);
		vec2 fwd = (chain > 0.001) ? tip_limited_position / chain : vec2(0.0);
		vec2 thumb_pos = joystick_position + tip_limited_position;
		float prev_edge = 40.0;  // thumb radius (edge of the 80px thumb from its center)
		for (int j = 0; j < 5; j++) {
			float r = (j <= 1) ? 16.0 : ((j == 2) ? 9.0 : ((j == 3) ? 6.0 : 4.0));
			float gap = (j == 0) ? 20.0 : max_bubble_distance;
			float center = prev_edge + gap + r;
			if (center > chain) {
				break;
			}
			vec2 pos = thumb_pos - fwd * center;
			float alpha = mix(0.45, 0.10, float(j) / 4.0);
			acc = over(orb(canvas_point, pos, r, alpha, 0.0), acc);
			prev_edge = center + r;
		}

		// Thumb: an 80px orb (fill 50%, opaque outline) — just the circle, no inner dot.
		acc = over(orb(canvas_point, joystick_position + tip_limited_position, 40.0, 0.50, 1.0), acc);
	}

	COLOR = acc;
}
