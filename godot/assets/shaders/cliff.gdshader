shader_type spatial;

#include "res://shaders/modules/world_aligned.gdshaderinc"

varying vec3 world_position;
varying vec3 world_normal;

uniform sampler2D texture_albedo : source_color;
uniform sampler2D noise_texture : hint_default_black;
uniform float noise_scale : hint_range(0.0, 10.0) = 0.01;
uniform vec3 distortion = vec3(0.0);

uniform vec2 texture_coord_1 = vec2(0.75, 0.75);
uniform vec2 texture_coord_2 = vec2(0.55, 0.55);

void vertex() {
	world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	world_normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
}

void fragment() {
	vec2 aligned_uv = fract(world_aligned_uv(world_normal, world_position * noise_scale, distortion));
	float t = texture(noise_texture, aligned_uv).r;
	vec2 final_uv = mix(texture_coord_1, texture_coord_2, t);
	vec3 final_color = texture(texture_albedo, final_uv).rgb;
	ALBEDO = final_color;
}
