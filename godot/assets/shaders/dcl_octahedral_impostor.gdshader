shader_type spatial;
render_mode unshaded, cull_disabled, depth_prepass_alpha;

uniform sampler2D atlas : source_color, filter_linear, repeat_disable;
uniform int n_cells = 8;
uniform bool is_hemi = true;

// Sun direction + colors for the manual Lambert lighting. Driven by
// the same global shader params as the sky / main directional light
// (sky_base.gd pushes them every frame from the day/night cycle).
// Staying in `unshaded` keeps Godot's PBR pipeline out of the picture
// (which empirically choked Mali shader compilation when 288 unique
// ShaderMaterials hit the load queue at once).
global uniform vec3 sun_direction;
global uniform vec3 sun_color;
global uniform vec3 ambient_color;

varying vec2 v_uv;
varying flat vec2 v_frame1;
varying flat vec2 v_frame2;
varying flat vec2 v_frame3;
varying vec4 v_blend_weights;

// Direction → grid coordinates in [-1, 1]². Hemi mode uses the
// rotated-diamond unfolding (zenith at center, horizon at borders);
// full-sphere uses the standard Cigolle/Brucks fold.
vec2 dir_to_grid(vec3 d) {
	if (is_hemi) {
		d.y = max(d.y, 0.001);
		d = normalize(d);
		vec3 octant = vec3(d.x >= 0.0 ? 1.0 : -1.0, 1.0, d.z >= 0.0 ? 1.0 : -1.0);
		float s = dot(d, octant);
		vec3 oct = d / s;
		return vec2(oct.x + oct.z, oct.z - oct.x);
	} else {
		vec3 octant = vec3(d.x >= 0.0 ? 1.0 : -1.0, d.y >= 0.0 ? 1.0 : -1.0, d.z >= 0.0 ? 1.0 : -1.0);
		float s = dot(d, octant);
		vec3 oct = d / s;
		if (oct.y < 0.0) {
			vec2 abs_oct = vec2(abs(oct.x), abs(oct.z));
			oct.xz = octant.xz * vec2(1.0 - abs_oct.y, 1.0 - abs_oct.x);
		}
		return oct.xz;
	}
}

// Brucks-canonical three-corner barycentric weights across the
// cell's (0,0)→(1,1) diagonal. Returns (w_floor, w_mid, w_ceil, mid_selector)
// where mid_selector picks between (0,1) and (1,0) for the mid corner.
vec4 quad_blend_weights(vec2 f) {
	vec4 r;
	r.x = min(1.0 - f.x, 1.0 - f.y); // floor (0,0) weight
	r.y = abs(f.x - f.y);             // mid corner weight
	r.z = min(f.x, f.y);              // ceil (1,1) weight
	r.w = ceil(f.x - f.y);            // 0 → mid=(0,1), 1 → mid=(1,0)
	return r;
}

void vertex() {
	// Camera direction in model-local space, using the ORTHONORMALIZED
	// model basis (rotation only). The previous `inverse(MODEL_MATRIX)`
	// folded the model's scale into the lookup: under non-uniform scale
	// (common on DCL props) S^-1 skews the direction before normalize,
	// so dir_to_grid picks the wrong octahedral cell and the impostor
	// shows the prop from the wrong angle — i.e. its apparent rotation
	// stops matching a rotated/scaled source mesh. Normalizing each
	// basis column drops scale and leaves pure rotation, which is the
	// frame the atlas was baked in.
	mat3 model_rot = mat3(
		normalize(MODEL_MATRIX[0].xyz),
		normalize(MODEL_MATRIX[1].xyz),
		normalize(MODEL_MATRIX[2].xyz)
	);
	vec3 to_cam_world = INV_VIEW_MATRIX[3].xyz - MODEL_MATRIX[3].xyz;
	vec3 dir_local = normalize(transpose(model_rot) * to_cam_world);

	// Map view direction → grid position in [0, N-1].
	vec2 frames_minus_one = vec2(float(n_cells) - 1.0);
	vec2 raw = dir_to_grid(dir_local);
	vec2 grid_uv = clamp((raw + 1.0) * 0.5, vec2(0.0), vec2(1.0));
	vec2 grid = clamp(grid_uv * frames_minus_one, vec2(0.0), frames_minus_one);
	vec2 grid_floor = min(floor(grid), frames_minus_one);
	vec2 grid_fract = grid - grid_floor;

	v_frame1 = grid_floor;
	v_blend_weights = quad_blend_weights(grid_fract);
	v_frame2 = clamp(
		v_frame1 + mix(vec2(0.0, 1.0), vec2(1.0, 0.0), v_blend_weights.w),
		vec2(0.0), frames_minus_one);
	v_frame3 = clamp(v_frame1 + vec2(1.0), vec2(0.0), frames_minus_one);
	v_uv = UV;

	// Spherical billboard in MODEL-LOCAL space. Stable up direction
	// is world-Y mapped into model-local; falls back to world-Z when
	// the camera direction collapses onto Y to keep the cross
	// product well-conditioned. Same convention as the bake.
	vec3 forward_local = dir_local;
	// model_rot already computed above (orthonormalized basis).
	vec3 up_local = normalize(transpose(model_rot) * vec3(0.0, 1.0, 0.0));
	if (abs(dot(forward_local, up_local)) > 0.999) {
		up_local = normalize(transpose(model_rot) * vec3(0.0, 0.0, 1.0));
	}
	vec3 right_local = normalize(cross(up_local, forward_local));
	vec3 up_proj_local = normalize(cross(forward_local, right_local));

	vec3 local = VERTEX;
	VERTEX = right_local * local.x + up_proj_local * local.y;
	NORMAL = -forward_local;
}

// Octahedral RG → unit normal. Mirrors the encode the normal-bake
// material does in fragment-stage.
vec3 octa_decode_normal(vec2 rg) {
	vec2 oct = rg * 2.0 - 1.0;
	float y = 1.0 - abs(oct.x) - abs(oct.y);
	vec2 xz = oct;
	if (y < 0.0) {
		vec2 sgn = vec2(xz.x >= 0.0 ? 1.0 : -1.0, xz.y >= 0.0 ? 1.0 : -1.0);
		xz = sgn * (1.0 - vec2(abs(xz.y), abs(xz.x)));
	}
	return normalize(vec3(xz.x, y, xz.y));
}

void fragment() {
	float n = float(n_cells);
	// Cell UV in [0,1]; mapped onto the packed atlas by halving v
	// (albedo = top half) and shifting +0.5 (normal+ORM = bottom half).
	vec2 cell_size = vec2(1.0) / vec2(n);
	vec2 cuv1 = (v_frame1 + v_uv) * cell_size;
	vec2 cuv2 = (v_frame2 + v_uv) * cell_size;
	vec2 cuv3 = (v_frame3 + v_uv) * cell_size;
	vec4 a1 = texture(atlas, vec2(cuv1.x, cuv1.y * 0.5));
	vec4 a2 = texture(atlas, vec2(cuv2.x, cuv2.y * 0.5));
	vec4 a3 = texture(atlas, vec2(cuv3.x, cuv3.y * 0.5));
	vec4 c = a1 * v_blend_weights.x + a2 * v_blend_weights.y + a3 * v_blend_weights.z;

	if (c.a < 0.1) {
		discard;
	}

	// Normal+ORM from the bottom half. The bake material stored
	// octa(view_normal) in RG, roughness B, metallic A.
	vec4 m1 = texture(atlas, vec2(cuv1.x, cuv1.y * 0.5 + 0.5));
	vec4 m2 = texture(atlas, vec2(cuv2.x, cuv2.y * 0.5 + 0.5));
	vec4 m3 = texture(atlas, vec2(cuv3.x, cuv3.y * 0.5 + 0.5));
	vec4 nrm_pkg = m1 * v_blend_weights.x + m2 * v_blend_weights.y + m3 * v_blend_weights.z;
	vec3 nrm = octa_decode_normal(nrm_pkg.rg);

	// Half-Lambert against the sun so the impostor never goes fully
	// black even where the baked normal is imperfect. The baked normal
	// is in bake-time view space (= runtime camera→impostor direction),
	// so transform the world-space global sun_direction into view space
	// before the dot product.
	vec3 sun_dir_view = normalize((VIEW_MATRIX * vec4(sun_direction, 0.0)).xyz);
	float ndl = dot(nrm, sun_dir_view) * 0.5 + 0.5;
	vec3 lit = c.rgb * (ambient_color + sun_color * ndl);
	ALBEDO = lit;
	ALPHA = c.a;
	ALPHA_SCISSOR_THRESHOLD = 0.5;
}
