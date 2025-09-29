shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_disabled, diffuse_burley, specular_schlick_ggx;

#include "res://shaders/modules/world_aligned.gdshaderinc"

varying vec3 world_position;
varying vec3 world_normal;

uniform sampler2D texture_albedo : source_color;
uniform float texture_scale : hint_range(0.1, 10.0) = 1.0;
uniform vec3 distortion = vec3(0.0);
uniform vec4 grass_rect = vec4(0.01, 0.51, 0.48, 0.48);

uniform sampler2D grass_heightmap : hint_default_black, filter_linear_mipmap, repeat_enable;
uniform float heightmap_scale : hint_range(-16.0, 16.0, 0.001) = -1.0;
uniform int heightmap_min_layers : hint_range(1, 64) = 3;
uniform int heightmap_max_layers : hint_range(1, 64) = 5;
uniform vec2 heightmap_flip = vec2(1.0, 1.0);
uniform float height_darkening : hint_range(0.0, 1.0, 0.01) = 0.1;
uniform float heightmap_texture_scale : hint_range(0.1, 10.0) = 1.5;

uniform float edge_fade_power : hint_range(0.1, 4.0) = 2.0;
uniform float alpha_scissor : hint_range(0.0, 1.0) = 0.1;
uniform float fade_curve : hint_range(0.1, 4.0) = 1.5;
uniform float heightmap_influence : hint_range(0.0, 1.0) = 0.5;

varying float vertex_distance;

void vertex() {
	world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	world_normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
	// Pass vertex color's red channel as distance (white=1.0 at floor, black=0.0 at edge)
	vertex_distance = COLOR.r;
}

void fragment() {
	vec2 aligned_uv = fract(world_aligned_uv(world_normal, world_position * texture_scale, distortion));
	vec2 grass_uv = grass_rect.xy + aligned_uv * grass_rect.zw;

	float final_height = 0.0;

	vec3 view_dir = normalize(normalize(-VERTEX + EYE_OFFSET) * mat3(TANGENT * heightmap_flip.x, -BINORMAL * heightmap_flip.y, NORMAL));
	float num_layers = mix(float(heightmap_max_layers), float(heightmap_min_layers), abs(dot(vec3(0.0, 0.0, 1.0), view_dir)));
	float layer_depth = 1.0 / num_layers;
	float current_layer_depth = 0.0;
	vec2 p = view_dir.xy * heightmap_scale * 0.01;
	vec2 delta = p / num_layers;
	vec2 ofs = grass_uv;
	vec2 heightmap_uv = aligned_uv * heightmap_texture_scale;
	float depth = texture(grass_heightmap, heightmap_uv).r;

	float current_depth = 0.0;
	while (current_depth < depth) {
		ofs -= delta;
		heightmap_uv = (ofs - grass_rect.xy) / grass_rect.zw * heightmap_texture_scale;
		depth = texture(grass_heightmap, heightmap_uv).r;
		current_depth += layer_depth;
	}

	vec2 prev_ofs = ofs + delta;
	vec2 prev_heightmap_uv = (prev_ofs - grass_rect.xy) / grass_rect.zw * heightmap_texture_scale;
	float after_depth = depth - current_depth;
	float before_depth = texture(grass_heightmap, prev_heightmap_uv).r - current_depth + layer_depth;

	float weight = after_depth / (after_depth - before_depth);
	grass_uv = mix(ofs, prev_ofs, weight);

	vec2 final_heightmap_uv = (grass_uv - grass_rect.xy) / grass_rect.zw * heightmap_texture_scale;
	final_height = texture(grass_heightmap, final_heightmap_uv).r;

	vec4 grass_color = textureLod(texture_albedo, grass_uv, 1.0);
	grass_color.rgb *= 1.0 - ((1.0 - final_height) * height_darkening);

	float distance_fade = pow(vertex_distance, fade_curve);
	float height_fade = final_height;
	float combined_fade = mix(distance_fade, distance_fade * height_fade, heightmap_influence);
	float edge_fade = pow(combined_fade, edge_fade_power);
	float alpha = grass_color.a * edge_fade;


	ALBEDO = grass_color.rgb;
	ALPHA = alpha;
  ALPHA_SCISSOR_THRESHOLD = alpha_scissor;
}
