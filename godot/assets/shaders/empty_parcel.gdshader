shader_type spatial;

#include "res://shaders/modules/world_aligned.gdshaderinc"

varying vec3 world_position;
varying vec3 world_normal;

uniform sampler2D texture_albedo : source_color;
uniform float texture_scale : hint_range(0.1, 10.0) = 1.0;
uniform vec3 distortion = vec3(0.0);
uniform vec4 grass_rect = vec4(0.01, 0.51, 0.48, 0.48);

global uniform sampler2D parcel_data_texture;
global uniform vec2 current_parcel_origin;

void vertex() {
	// Calculate world position in vertex shader
	world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;

	// Calculate world normal in vertex shader
	world_normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);

}

void fragment() {
	// Calculate world-aligned UV coordinates using interpolated world position and normal
	vec2 aligned_uv = fract(world_aligned_uv(world_normal, world_position * texture_scale, distortion));

	// Map UV to grass quadrant using uniform rect (xy: offset, zw: size)
	vec2 grass_uv = grass_rect.xy + aligned_uv * grass_rect.zw;

	vec4 tex_color = textureLod(texture_albedo, grass_uv, 1.0);

	ALBEDO = tex_color.rgb;
	ALPHA = 1.0;
}
