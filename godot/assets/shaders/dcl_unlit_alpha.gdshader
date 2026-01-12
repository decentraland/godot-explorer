shader_type spatial;
render_mode unshaded, cull_back, depth_prepass_alpha;

uniform sampler2D albedo_texture : source_color, filter_linear_mipmap, repeat_enable, hint_default_white;
uniform sampler2D alpha_texture : filter_linear_mipmap, repeat_enable;
uniform vec4 diffuse_color : source_color = vec4(1.0, 1.0, 1.0, 1.0);
uniform vec2 uv_offset = vec2(0.0, 0.0);
uniform vec2 uv_scale = vec2(1.0, 1.0);

void fragment() {
    vec2 uv = UV * uv_scale + uv_offset;
    vec4 albedo = texture(albedo_texture, uv) * diffuse_color;

    ALBEDO = albedo.rgb;
    ALPHA = texture(alpha_texture, uv).r;
}
