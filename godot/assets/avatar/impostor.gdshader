shader_type spatial;
render_mode unshaded, cull_disabled, depth_prepass_alpha;

uniform sampler2DArray impostor_array : source_color, filter_linear, repeat_disable;
uniform vec3 tint_color : source_color = vec3(0.0, 0.0, 0.0);

const mat4 BAYER_4X4 = mat4(
	vec4( 0.0 / 16.0,  8.0 / 16.0,  2.0 / 16.0, 10.0 / 16.0),
	vec4(12.0 / 16.0,  4.0 / 16.0, 14.0 / 16.0,  6.0 / 16.0),
	vec4( 3.0 / 16.0, 11.0 / 16.0,  1.0 / 16.0,  9.0 / 16.0),
	vec4(15.0 / 16.0,  7.0 / 16.0, 13.0 / 16.0,  5.0 / 16.0)
);

float dither_threshold(vec2 frag_coord) {
	int x = int(mod(frag_coord.x, 4.0));
	int y = int(mod(frag_coord.y, 4.0));
	return BAYER_4X4[y][x];
}

void vertex() {
	// QuadMesh's default mesh sits in the local XY plane facing +Z. We
	// rebuild the vertex in object space so it always points at the camera
	// in the XZ world plane (Y-axis billboard) while keeping its world Y
	// upright. Assumes the instance transform has identity rotation and
	// only a translation, which is what AvatarScene writes per frame.
	vec3 inst_pos_world = MODEL_MATRIX[3].xyz;
	vec3 cam_pos_world = INV_VIEW_MATRIX[3].xyz;

	vec3 forward = vec3(
		cam_pos_world.x - inst_pos_world.x,
		0.0,
		cam_pos_world.z - inst_pos_world.z
	);
	if (length(forward) < 0.0001) {
		forward = vec3(0.0, 0.0, 1.0);
	} else {
		forward = normalize(forward);
	}
	vec3 up = vec3(0.0, 1.0, 0.0);
	vec3 right = normalize(cross(up, forward));

	vec3 local = VERTEX;
	VERTEX = right * local.x + up * local.y;
	NORMAL = forward;
}

void fragment() {
	float tint_strength = INSTANCE_CUSTOM.r;
	float fade_alpha = INSTANCE_CUSTOM.g;
	float layer_index = INSTANCE_CUSTOM.b;

	// Slot inactive — invisible.
	if (fade_alpha <= 0.001) {
		discard;
	}

	vec4 tex_color = texture(impostor_array, vec3(UV, layer_index));
	if (tex_color.a < 0.1) {
		discard;
	}

	// Dither cross-fade between full avatar and impostor.
	if (dither_threshold(FRAGCOORD.xy) > fade_alpha) {
		discard;
	}

	ALBEDO = mix(tex_color.rgb, tint_color, clamp(tint_strength, 0.0, 1.0));
	ALPHA = tex_color.a;
	ALPHA_SCISSOR_THRESHOLD = 0.5;
}
