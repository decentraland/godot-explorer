// Material override applied to mesh copies in the impostor normal bake.
// Encodes the per-fragment view-space NORMAL into octahedral RG and packs
// roughness (B) + metallic (A) so the runtime impostor shader can do
// per-pixel Lambert lighting + cheap PBR-like response.
//
// View-space normals: the bake camera at +Z looking at origin renders each
// rotated mesh copy, and Godot's spatial fragment-stage NORMAL is in
// MODELVIEW space at that point — exactly what the runtime impostor wants
// when it samples the cell whose direction matches the current camera (the
// cell tri-blend is closest to the runtime view direction).
shader_type spatial;
render_mode unshaded;

uniform float src_roughness : hint_range(0.0, 1.0) = 0.7;
uniform float src_metallic : hint_range(0.0, 1.0) = 0.0;

void fragment() {
	vec3 n = normalize(NORMAL);

	// Octahedral encode XY-of-octahedron into a unit square. Same fold
	// the runtime shader's `dir_to_grid` uses for full-sphere mode so
	// encoding-decoding round-trips cleanly.
	vec3 octant = vec3(n.x >= 0.0 ? 1.0 : -1.0, n.y >= 0.0 ? 1.0 : -1.0, n.z >= 0.0 ? 1.0 : -1.0);
	float s = max(1e-4, dot(n, octant));
	vec3 oct = n / s;
	if (oct.y < 0.0) {
		vec2 abs_xz = vec2(abs(oct.x), abs(oct.z));
		oct.xz = octant.xz * vec2(1.0 - abs_xz.y, 1.0 - abs_xz.x);
	}
	vec2 enc = oct.xz * 0.5 + 0.5;

	ALBEDO = vec3(enc.x, enc.y, src_roughness);
	ALPHA = src_metallic;
}
