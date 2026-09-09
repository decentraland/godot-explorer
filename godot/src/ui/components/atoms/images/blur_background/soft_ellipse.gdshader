shader_type canvas_item;

// Soft elliptical glow ("blurred ellipse") done analytically — ZERO texture fetches, the cheapest
// possible: alpha falls off radially from the center to the rect edges, so on a non-square rect
// it reads as an ellipse. Meant for a ColorRect (the glow colour = its Color / Modulate).
//
// To show "half" the ellipse (the light bleeding in from an edge, as in the Discover card): place
// the rect so its centre sits on the card edge and let the card (clip_contents) crop the rest —
// no separate mask needed.

// Radius (0..1) of the fully-opaque core before the falloff starts. 0 = the glow peaks only at the
// exact centre; higher = a larger solid-bright centre.
uniform float core : hint_range(0.0, 1.0, 0.01) = 0.0;
// Shapes the falloff curve: 1.0 ≈ smooth linear; >1 tightens the glow toward the centre; <1 spreads it.
uniform float falloff : hint_range(0.1, 4.0, 0.05) = 1.0;

void fragment() {
	// 0 at centre → 1 at the rect edges. Dividing by half-size normalises per-axis, so a wide rect
	// gives a wide ellipse.
	float d = length((UV - vec2(0.5)) / vec2(0.5));
	float mask = 1.0 - smoothstep(core, 1.0, d);
	mask = pow(mask, falloff);
	COLOR.a *= mask;
}
