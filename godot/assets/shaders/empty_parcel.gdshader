shader_type spatial;

#include "res://shaders/modules/world_aligned.gdshaderinc"

varying vec3 world_position;
varying vec3 world_normal;

uniform sampler2D texture_albedo : source_color;
uniform float texture_scale : hint_range(0.1, 10.0) = 1.0;
uniform vec3 distortion = vec3(0.0);
uniform vec4 grass_rect = vec4(0.01, 0.51, 0.48, 0.48);
uniform vec4 rock_rect = vec4(0.51, 0.51, 0.48, 0.48);  // Rock texture in the same atlas
uniform float slope_threshold : hint_range(0.0, 1.0) = 0.7;  // Threshold for grass/rock blend
uniform float slope_blend : hint_range(0.0, 0.5) = 0.15;  // Blend range for smooth transition

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

	// Calculate slope factor (dot product of normal with up vector)
	float slope = dot(normalize(world_normal), vec3(0.0, 1.0, 0.0));

	// Calculate blend factor with smooth transition
	// When slope is close to 1 (flat), use grass. When slope is low (steep), use rock
	float blend_factor = smoothstep(slope_threshold - slope_blend, slope_threshold + slope_blend, slope);

	// Sample both textures
	vec2 grass_uv = grass_rect.xy + aligned_uv * grass_rect.zw;
	vec2 rock_uv = rock_rect.xy + aligned_uv * rock_rect.zw;

	vec4 grass_color = textureLod(texture_albedo, grass_uv, 1.0);
	vec4 rock_color = textureLod(texture_albedo, rock_uv, 1.0);

	// Blend between grass and rock based on slope
	vec3 final_color = mix(rock_color.rgb, grass_color.rgb, blend_factor);

	ALBEDO = final_color;
	ALPHA = 1.0;
}
