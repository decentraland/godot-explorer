shader_type spatial;

#include "res://shaders/modules/world_aligned.gdshaderinc"

varying vec3 world_position;
varying vec3 world_normal;

uniform sampler2D texture_albedo : source_color;
uniform float texture_scale : hint_range(0.1, 10.0) = 1.0;
uniform vec3 distortion = vec3(0.0);
uniform vec4 grass_rect = vec4(0.01, 0.51, 0.48, 0.48);

uniform sampler2D darkening_texture : hint_default_black, filter_linear_mipmap, repeat_enable;
uniform float darkening_strength : hint_range(0.0, 1.0, 0.01) = 0.3;
uniform float darkening_texture_scale : hint_range(0.1, 10.0) = 1.0;

global uniform sampler2D parcel_data_texture;
global uniform vec2 current_parcel_origin;

void vertex() {
	world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	world_normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
}

void fragment() {
	vec2 aligned_uv = fract(world_aligned_uv(world_normal, world_position * texture_scale, distortion));
	vec2 grass_uv = grass_rect.xy + aligned_uv * grass_rect.zw;

	// Sample base grass texture
	vec4 grass_color = textureLod(texture_albedo, grass_uv, 1.0);

	// Sample darkening texture
	vec2 darkening_uv = aligned_uv * darkening_texture_scale;
	float darkening_value = texture(darkening_texture, darkening_uv).r;

	// Apply darkening based on texture value
	grass_color.rgb *= 1.0 - (darkening_value * darkening_strength);

	ALBEDO = grass_color.rgb;
}
