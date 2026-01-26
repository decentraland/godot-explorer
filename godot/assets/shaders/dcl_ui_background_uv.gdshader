shader_type canvas_item;

// UV coordinates for each corner of the quad
uniform vec2 uv_top_left = vec2(0.0, 0.0);
uniform vec2 uv_top_right = vec2(1.0, 0.0);
uniform vec2 uv_bottom_left = vec2(0.0, 1.0);
uniform vec2 uv_bottom_right = vec2(1.0, 1.0);

void fragment() {
    vec2 screen_uv = UV;

    // Standard bilinear interpolation
    vec2 top_edge = mix(uv_top_left, uv_top_right, screen_uv.x);
    vec2 bottom_edge = mix(uv_bottom_left, uv_bottom_right, screen_uv.x);
    vec2 final_uv = mix(top_edge, bottom_edge, screen_uv.y);

    COLOR = texture(TEXTURE, final_uv);
}
