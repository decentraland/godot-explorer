shader_type canvas_item;

// SDK uvs, clockwise from bottom-left, with v pointing up:
// uvs_0 = vec4(bl.x, bl.y, tl.x, tl.y)
// uvs_1 = vec4(tr.x, tr.y, br.x, br.y)
uniform vec4 uvs_0 = vec4(0.0, 0.0, 0.0, 1.0);
uniform vec4 uvs_1 = vec4(1.0, 1.0, 1.0, 0.0);

// The inherited modulate (UiBackground.color). Fragment COLOR is already
// multiplied by the texel at the unmapped UV, so it is captured here.
varying vec4 modulate;

void vertex() {
    modulate = COLOR;
}

void fragment() {
    // Bilinear interpolation matching bevy-explorer's approach
    vec2 uv = mix(
        mix(uvs_0.xy, uvs_1.zw, UV.x),  // bottom edge: BL to BR
        mix(uvs_0.zw, uvs_1.xy, UV.x),  // top edge: TL to TR
        1.0 - UV.y                       // UV.y = 0 is the top of the rect
    ) * vec2(1.0, -1.0) + vec2(0.0, 1.0); // SDK v up -> Godot v down

    COLOR = texture(TEXTURE, uv) * modulate;
}
