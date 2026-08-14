shader_type canvas_item;

// Circular "orb" button — fully procedural, resolution independent.
// STYLIZED glass: spherical shading + specular highlight + fresnel rim.
// Does NOT sample the backdrop, so it costs nothing extra and never needs a
// BackBufferCopy. The "glass" is faked with lighting only.
//
// Draw this on a *square* Control/ColorRect. Leave a margin (body_radius < 1)
// so the outer glow has room and is not clipped by the control rect.

// --- shape ---
uniform float body_radius : hint_range(0.1, 1.0) = 0.72;
uniform float depth : hint_range(0.0, 1.0) = 0.85; // sphere curvature (Glass "Depth")

// --- radial fill (two stops, per spec) ---
uniform vec4 fill_inner : source_color = vec4(0.145, 0.102, 0.231, 1.0); // #251A3B
uniform vec4 fill_outer : source_color = vec4(0.086, 0.082, 0.094, 1.0); // #161518
uniform float fill_opacity : hint_range(0.0, 1.0) = 0.9;
uniform float fill_start : hint_range(0.0, 1.0) = 0.0; // radial inner-stop position (0% / 22%)

// --- outline ---
uniform vec4 outline_color : source_color = vec4(0.874, 0.816, 1.0, 1.0); // #DFD0FF
uniform float outline_width : hint_range(0.0, 0.08) = 0.012;
uniform bool outline_angular = false; // true => conic gradient (idle state)

// --- outer glow ---
uniform vec4 glow_color : source_color = vec4(0.874, 0.816, 1.0, 1.0); // #DF9CFF-ish
uniform float glow_strength : hint_range(0.0, 2.0) = 0.0;
uniform float glow_softness : hint_range(0.01, 0.6) = 0.18;

// --- inner shadow ---
uniform vec4 inner_shadow_color : source_color = vec4(0.819, 0.678, 1.0, 1.0); // #D1ADFF
uniform float inner_shadow_strength : hint_range(0.0, 1.0) = 0.6;
uniform vec2 inner_shadow_offset = vec2(0.04, 0.04);
uniform float inner_shadow_blur : hint_range(0.01, 0.4) = 0.13;

// --- light / stylized glass ---
uniform float light_angle_deg = -45.0;
uniform float spec_strength : hint_range(0.0, 2.0) = 0.7;
uniform float spec_sharpness : hint_range(1.0, 200.0) = 40.0;
uniform float fresnel_strength : hint_range(0.0, 1.0) = 0.25;

uniform float opacity : hint_range(0.0, 1.0) = 1.0;


vec3 light_dir() {
	float a = radians(light_angle_deg);
	return normalize(vec3(cos(a), sin(a), 0.7));
}

void fragment() {
	vec2 p = (UV - 0.5) * 2.0;
	float r = length(p);
	float aa = fwidth(r) * 1.5;

	float inside = 1.0 - smoothstep(body_radius - aa, body_radius + aa, r);

	// fake sphere normal from the SDF disk
	float nr = clamp(r / body_radius, 0.0, 1.0);
	float z = sqrt(max(0.0, 1.0 - nr * nr));
	vec3 nrm = normalize(vec3((p / body_radius) * depth, z));
	vec3 lgt = light_dir();
	vec3 view = vec3(0.0, 0.0, 1.0);

	// radial fill; fill_start shifts the inner stop (per spec: 0% or 22%).
	// per-stop alpha (the "- 90%" in the spec) is honoured here, not just rgb.
	float ft = smoothstep(fill_start, 1.0, nr);
	vec3 col = mix(fill_inner.rgb, fill_outer.rgb, ft);
	float stop_a = mix(fill_inner.a, fill_outer.a, ft);

	// spherical diffuse shading
	float ndl = clamp(dot(nrm, lgt), 0.0, 1.0);
	col *= mix(0.55, 1.15, ndl);

	// inner shadow near the rim, opposite the offset
	float rs = length(p - inner_shadow_offset * 2.0);
	float ish = smoothstep(body_radius - inner_shadow_blur, body_radius, rs) * inside;
	col = mix(col, inner_shadow_color.rgb, ish * inner_shadow_strength * 0.6);

	// specular highlight (the stylized "glass")
	vec3 hlf = normalize(lgt + view);
	float spec = pow(clamp(dot(nrm, hlf), 0.0, 1.0), spec_sharpness) * spec_strength;
	col += spec * inside;

	// fresnel rim
	float fres = pow(1.0 - clamp(dot(nrm, view), 0.0, 1.0), 3.0) * fresnel_strength;
	col += glow_color.rgb * fres * inside;

	float alpha = stop_a * fill_opacity * inside;

	// outline (solid or conic/angular gradient)
	float band = abs(r - body_radius);
	float ring = 1.0 - smoothstep(outline_width, outline_width + aa, band);
	if (outline_angular) {
		float t = fract(atan(p.y, p.x) / TAU + 1.0);
		// approximate the 15/35/65/85% alternating-alpha sweep from the spec
		float ao = mix(0.3, 1.0, 0.5 + 0.5 * sin(t * TAU * 2.0));
		col = mix(col, outline_color.rgb, ring * ao);
		alpha = max(alpha, ring * ao);
	} else {
		col = mix(col, outline_color.rgb, ring * outline_color.a);
		alpha = max(alpha, ring * outline_color.a);
	}

	// outer glow (only outside the body)
	float glow = smoothstep(body_radius + glow_softness, body_radius, r) * (1.0 - inside);
	glow *= glow_strength;
	col += glow_color.rgb * glow;
	alpha = max(alpha, glow * glow_color.a);

	COLOR = vec4(col, clamp(alpha, 0.0, 1.0) * opacity);
}
