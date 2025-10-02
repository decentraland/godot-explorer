shader_type spatial;
render_mode cull_back, depth_draw_always;

#include "res://shaders/modules/world_aligned.gdshaderinc"

varying vec3 world_position;
varying vec3 world_normal;
varying float vertex_falloff;

uniform sampler2D texture_albedo : source_color;
uniform float texture_scale : hint_range(0.1, 10.0) = 0.1;
uniform vec3 distortion = vec3(0.0);
uniform vec4 grass_rect = vec4(0.01, 0.51, 0.48, 0.48);

uniform sampler2D darkening_texture : hint_default_black, filter_linear_mipmap, repeat_enable;
uniform float darkening_strength : hint_range(0.0, 1.0, 0.01) = 1.0;
uniform float darkening_texture_scale : hint_range(0.1, 10.0) = 2.0;

// Shell texture parameters
uniform float shell_threshold : hint_range(0.0, 1.0, 0.01) = 0.0;
uniform float shell_grow : hint_range(0.0, 0.5, 0.01) = 0.0;

void vertex() {
	world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	world_normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);

	// Get vertex falloff from vertex color (stored in COLOR.r)
	vertex_falloff = COLOR.r;

	// Apply shell growth along normal, modulated by vertex falloff
	VERTEX += NORMAL * shell_grow * vertex_falloff;
}

void fragment() {
	vec2 aligned_uv = fract(world_aligned_uv(world_normal, world_position * texture_scale, distortion));
	vec2 grass_uv = grass_rect.xy + aligned_uv * grass_rect.zw;

	// Sample base grass texture
	vec4 grass_color = textureLod(texture_albedo, grass_uv, 1.0);

	// Sample darkening texture (used as alpha mask for shell)
	vec2 darkening_uv = aligned_uv * darkening_texture_scale;
	float darkening_value = texture(darkening_texture, darkening_uv).r;

	// Apply darkening based on texture value
	grass_color.rgb *= 1.0 - (darkening_value * darkening_strength);

	ALBEDO = grass_color.rgb;
	ALPHA = darkening_value;
	ALPHA_SCISSOR_THRESHOLD = shell_threshold;
}
