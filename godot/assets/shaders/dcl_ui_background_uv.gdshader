shader_type canvas_item;

// UV coordinates packed like bevy-explorer:
// uvs_0 = vec4(tl.x, tl.y, bl.x, bl.y)
// uvs_1 = vec4(br.x, br.y, tr.x, tr.y)
uniform vec4 uvs_0 = vec4(0.0, 0.0, 0.0, 1.0);
uniform vec4 uvs_1 = vec4(1.0, 1.0, 1.0, 0.0);

void fragment() {
    // Bilinear interpolation matching bevy-explorer's approach
    vec2 uv = mix(
        mix(uvs_0.xy, uvs_1.zw, UV.x),  // top edge: TL to TR
        mix(uvs_0.zw, uvs_1.xy, UV.x),  // bottom edge: BL to BR
        1.0 - UV.y                       // inverted Y interpolation
    ) * vec2(1.0, -1.0) + vec2(0.0, 1.0); // flip Y

    COLOR = texture(TEXTURE, uv);
}
