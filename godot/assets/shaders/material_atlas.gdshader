shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx;

// Per-layer albedo. Each layer holds one source material's albedo texture
// (resized at pack time to a uniform CELL_SIZE, default 512x512). Sampled
// using the layer index baked per-vertex into CUSTOM0.x.
uniform sampler2DArray albedo_atlas : source_color, filter_linear_mipmap, repeat_disable;

// Per-layer scalar parameters packed in a 1xN texture. Row = layer.
//   .r = metallic
//   .g = roughness
//   .b = emissive intensity (scaled emission color)
//   .a = alpha cutoff (for ALPHA_SCISSOR; ignored otherwise)
uniform sampler2D material_params : filter_nearest, repeat_disable, hint_default_white;

// Per-layer color factors (albedo tint + emissive tint). Same indexing.
uniform sampler2D material_colors : filter_nearest, repeat_disable, hint_default_white;

// Total layers actually populated. Used to clamp out-of-range indices.
uniform int layer_count : hint_range(1, 256) = 1;

// CUSTOM0 is the default RGBA8_UNORM custom channel — 4 bytes per vertex,
// each component normalized 0..1 in the vertex stage. The atlas layer index
// (0..255) is packed in the R byte, so multiply by 255 to recover.
varying flat float v_layer;

void vertex() {
    v_layer = CUSTOM0.x * 255.0;
}

void fragment() {
    int layer = clamp(int(v_layer + 0.5), 0, layer_count - 1);

    vec4 sampled = texture(albedo_atlas, vec3(UV, float(layer)));
    vec4 albedo_factor = texelFetch(material_colors, ivec2(0, layer), 0);
    vec4 params = texelFetch(material_params, ivec2(0, layer), 0);

    ALBEDO = sampled.rgb * albedo_factor.rgb;
    ALPHA = sampled.a * albedo_factor.a;
    METALLIC = params.r;
    ROUGHNESS = params.g;
    EMISSION = albedo_factor.rgb * params.b;
}
