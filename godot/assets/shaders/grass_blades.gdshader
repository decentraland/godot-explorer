shader_type spatial;
render_mode depth_draw_opaque, cull_disabled, diffuse_burley, specular_schlick_ggx;

#include "res://shaders/modules/world_aligned.gdshaderinc"

varying vec3 world_position;

uniform sampler2D texture_albedo : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform float alpha_scissor : hint_range(0.0, 1.0) = 0.5;
uniform sampler2D grass_atlas_texture : source_color;
uniform float texture_scale : hint_range(0.1, 10.0) = 1.0;
uniform vec3 distortion = vec3(0.0);
uniform vec4 grass_rect = vec4(0.01, 0.51, 0.48, 0.48);
uniform float gradient_power : hint_range(0.1, 4.0) = 1.5;
uniform float gradient_strength : hint_range(0.0, 1.0) = 0.4;
uniform float wind_strength : hint_range(0.0, 2.0) = 0.3;
uniform float wind_speed : hint_range(0.0, 10.0) = 2.0;
uniform vec2 wind_direction = vec2(1.0, 0.5);
uniform float wind_scale : hint_range(0.1, 10.0) = 2.0;
uniform float wind_turbulence : hint_range(0.0, 1.0) = 0.5;
uniform float billboard_mix : hint_range(0.0, 1.0) = 0.3;

float simple_noise(vec2 p) {
	return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}

float smooth_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);

	float a = simple_noise(i);
	float b = simple_noise(i + vec2(1.0, 0.0));
	float c = simple_noise(i + vec2(0.0, 1.0));
	float d = simple_noise(i + vec2(1.0, 1.0));

	return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

void vertex() {
	world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;

	vec3 camera_pos = INV_VIEW_MATRIX[3].xyz;
	float distance_to_camera = length(world_position - camera_pos);

	float fade_start = 16.0;
	float fade_end = 24.0;
	float distance_scale = 1.0 - smoothstep(fade_start, fade_end, distance_to_camera);

	VERTEX *= distance_scale;

	vec2 wind_dir = normalize(wind_direction);
	vec2 wind_coords = world_position.xz * wind_scale + wind_dir * TIME * wind_speed;
	float main_wind = smooth_noise(wind_coords) * 2.0 - 1.0;

	vec2 turb_coords = world_position.xz * wind_scale * 2.3 + wind_dir.yx * TIME * wind_speed * 1.5;
	float turbulence = (smooth_noise(turb_coords) * 2.0 - 1.0) * wind_turbulence;

	float wind_offset = (main_wind + turbulence) * wind_strength;
	float height_factor = max(0.0, VERTEX.y);
	VERTEX.x += wind_dir.x * wind_offset * height_factor;
	VERTEX.z += wind_dir.y * wind_offset * height_factor;
	VERTEX.y += abs(wind_offset) * 0.1 * height_factor;

	vec3 view_dir_local = normalize((inverse(MODEL_MATRIX) * vec4(camera_pos - world_position, 0.0)).xyz);
	vec3 blade_up = vec3(0.0, 1.0, 0.0);
	vec3 blade_right = vec3(1.0, 0.0, 0.0);

	float up_view_alignment = abs(dot(blade_up, view_dir_local));
	float blend_strength = billboard_mix * up_view_alignment;

	vec3 camera_up_world = INV_VIEW_MATRIX[1].xyz;
	vec3 camera_forward_world = -INV_VIEW_MATRIX[2].xyz;
	vec3 camera_up_local = (inverse(MODEL_MATRIX) * vec4(camera_up_world, 0.0)).xyz;
	vec3 camera_forward_local = (inverse(MODEL_MATRIX) * vec4(camera_forward_world, 0.0)).xyz;

	vec3 new_up = normalize(mix(blade_up, camera_up_local, blend_strength));
	vec3 new_right = normalize(mix(blade_right, camera_forward_local, blend_strength));

	new_right = normalize(new_right - dot(new_right, new_up) * new_up);
	vec3 new_forward = normalize(cross(new_right, new_up));

	mat3 rotation_matrix = mat3(new_right, new_up, new_forward);
	VERTEX = rotation_matrix * VERTEX;

	VERTEX.y -= billboard_mix * up_view_alignment;

	NORMAL = vec3(0.0, 1.0, 0.0);
}

void fragment() {
	float alpha_mask = texture(texture_albedo, UV).r;

	vec3 world_normal = vec3(0.0, 1.0, 0.0);
	vec2 aligned_uv = fract(world_aligned_uv(world_normal, world_position * texture_scale, distortion));
	vec2 grass_uv = grass_rect.xy + aligned_uv * grass_rect.zw;
  vec4 grass_color = textureLod(grass_atlas_texture, grass_uv, 1.0);

	float gradient = pow(UV.y, gradient_power);
	vec3 darkened_color = grass_color.rgb * mix(1.0 - gradient_strength, 1.0, gradient);

	ALPHA = alpha_mask;
	ALPHA_SCISSOR_THRESHOLD = alpha_scissor;
	ALBEDO = darkened_color;
	NORMAL = (VIEW_MATRIX * vec4(0.0, 1.0, 0.0, 0.0)).xyz;
}
