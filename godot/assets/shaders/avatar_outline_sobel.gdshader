shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_never, depth_test_disabled;

#include "res://shaders/modules/rgb2hsv.gdshaderinc"
#include "res://shaders/modules/hsv2rgb.gdshaderinc"

uniform sampler2D depth_texture : hint_depth_texture, filter_linear_mipmap;
uniform sampler2D extracted_texture : hint_default_white, filter_linear_mipmap;  // The viewport texture with avatars
uniform sampler2D screen_texture : hint_screen_texture, filter_linear_mipmap;
uniform float max_distance : hint_range(0.0, 100.0) = 10.0;
uniform float outline_thickness : hint_range(0.1, 5.0) = 1.0;
uniform float hue_rotation_speed : hint_range(0.1, 5.0) = 1.0;
uniform float saturation : hint_range(0.5, 1.0) = 1.0;
uniform float value : hint_range(0.5, 1.0) = 1.0;

void vertex() {
	// Make this a fullscreen quad
	POSITION = vec4(VERTEX.xy, 1.0, 1.0);
}

float sobel_edge_detection(vec2 uv, mat4 inv_proj_mat) {
	vec2 texel_size = outline_thickness / vec2(textureSize(extracted_texture, 0));
	float depth = texture(depth_texture, uv).r;
#if CURRENT_RENDERER == RENDERER_COMPATIBILITY
  vec3 real_ndc = vec3(uv, depth) * 2.0 - 1.0;
#else
  vec3 real_ndc = vec3(uv * 2.0 - 1.0, depth);
#endif

  vec4 real_view = inv_proj_mat * vec4(real_ndc, 1.0);
  real_view.xyz /= real_view.w;
  float real_linear_depth = -real_view.z;

	// Sobel kernels for edge detection
	float sobel_x = 0.0;
	float sobel_y = 0.0;

	// Sample the 3x3 neighborhood
	for (int y = -1; y <= 1; y++) {
		for (int x = -1; x <= 1; x++) {
      vec2 offset = vec2(float(x),float(y)) * texel_size;
      float extracted_depth = texture(extracted_texture, uv + offset).r;
#if CURRENT_RENDERER == RENDERER_COMPATIBILITY
      vec3 ndc = vec3(uv, extracted_depth) * 2.0 - 1.0;
#else
      vec3 ndc = vec3(uv * 2.0 - 1.0, extracted_depth);
#endif

      vec4 view = inv_proj_mat * vec4(ndc, 1.0);
      view.xyz /= view.w;
      float linear_depth = -view.z;

      float sample_depth = mix(
        9999.9,
        linear_depth,
        smoothstep(linear_depth, linear_depth + .1*linear_depth, real_linear_depth - .1*real_linear_depth)
      );
      float base_mask = smoothstep(max_distance+1.0,max_distance,linear_depth);
      float mask = smoothstep(linear_depth-linear_depth*.2, linear_depth, real_linear_depth);
      mask = smoothstep(.0,.25,mask) * base_mask;

			// Sobel X kernel: [-1 0 1; -2 0 2; -1 0 1]
			if (x == -1) {
				sobel_x -= mask * (1.0 + float(y == 0));
			} else if (x == 1) {
				sobel_x += mask * (1.0 + float(y == 0));
			}
			// Sobel Y kernel: [-1 -2 -1; 0 0 0; 1 2 1]
			if (y == -1) {
				sobel_y -= mask * (1.0 + float(x == 0));
			} else if (y == 1) {
				sobel_y += mask * (1.0 + float(x == 0));
			}
		}
	}
	// Calculate edge magnitude
	return length(vec2(sobel_x, sobel_y));
}

void fragment() {
	float edge = sobel_edge_detection(SCREEN_UV, INV_PROJECTION_MATRIX);

	// Sample the screen color at this position
	vec3 screen_color = texture(screen_texture, SCREEN_UV).rgb;

	// Convert screen color to HSV
	vec3 hsv = rgb2hsv(screen_color);

	// Rotate hue by 180 degrees (0.5 in normalized space) and ensure high saturation/value
	hsv.x = fract(hsv.x + 0.5);
	hsv.y = max(hsv.y, saturation); // Ensure high saturation
	hsv.z = max(hsv.z, value); // Ensure high value/brightness

	// Convert back to RGB
	vec3 outline_color = hsv2rgb(hsv);

	ALBEDO = outline_color;
	ALPHA = edge;
}
