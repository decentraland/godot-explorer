shader_type spatial;
render_mode unshaded, cull_disabled, depth_prepass_alpha;

uniform sampler2D atlas : source_color, filter_linear, repeat_disable;
uniform int n_angles = 8;

varying flat int v_cell;
varying vec2 v_uv;

const float PI = 3.14159265358979;
const float TAU = 6.28318530717959;

void vertex() {
	// World-space instance origin and camera origin.
	vec3 inst_pos_world = MODEL_MATRIX[3].xyz;
	vec3 cam_pos_world = INV_VIEW_MATRIX[3].xyz;
	vec3 to_cam_world = cam_pos_world - inst_pos_world;

	// Camera direction in the instance's local frame — invert the
	// model's rotation (assume uniform scale, which is the DCL norm).
	mat3 model_rot = mat3(
		normalize(MODEL_MATRIX[0].xyz),
		normalize(MODEL_MATRIX[1].xyz),
		normalize(MODEL_MATRIX[2].xyz)
	);
	vec3 to_cam_local = transpose(model_rot) * to_cam_world;

	// Horizontal angle from instance-local +Z to camera (Y-axis sweep).
	// The bake placed cell i with the mesh rotated by i*(TAU/N) around Y,
	// so to find the closest cell we map the camera angle to the same
	// discrete grid.
	float angle = atan(to_cam_local.x, to_cam_local.z); // [-PI, PI]
	float t = angle / TAU + 0.5; // [0, 1]
	v_cell = int(floor(t * float(n_angles) + 0.5)) % n_angles;
	v_uv = UV;

	// Y-axis billboard: quad always faces the camera in the world XZ
	// plane while world up stays up. Reuses the pattern from the
	// avatar impostor shader.
	vec3 forward = vec3(to_cam_world.x, 0.0, to_cam_world.z);
	if (length(forward) < 0.0001) {
		forward = vec3(0.0, 0.0, 1.0);
	} else {
		forward = normalize(forward);
	}
	vec3 up = vec3(0.0, 1.0, 0.0);
	vec3 right = normalize(cross(up, forward));

	vec3 local = VERTEX;
	VERTEX = right * local.x + up * local.y;
	NORMAL = -forward;
}

void fragment() {
	// Atlas is N_ANGLES cells wide, 1 cell tall. Each cell occupies
	// 1/N of the U range.
	float n = float(n_angles);
	float u = (float(v_cell) + v_uv.x) / n;
	vec4 c = texture(atlas, vec2(u, v_uv.y));
	if (c.a < 0.1) {
		discard;
	}
	ALBEDO = c.rgb;
	ALPHA = c.a;
	ALPHA_SCISSOR_THRESHOLD = 0.5;
}
