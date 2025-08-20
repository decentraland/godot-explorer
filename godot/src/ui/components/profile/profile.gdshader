shader_type canvas_item;

uniform vec4 background_color : source_color;
uniform vec2 centers[4];
uniform float radii[4];
uniform vec4 colors[4];
uniform float time_scale = 1.0;

void fragment() {
    vec2 uv = UV;
    vec4 col = background_color;

    for (int i = 0; i < centers.length(); i++) {

        vec2 center = centers[i];
        float t = TIME * time_scale;


        center.x += 0.05 * sin(t + float(i) * 2.0);
        center.y += 0.05 * cos(t + float(i) * 1.5);

        float dist = distance(uv, center);
        float alpha = clamp(1.0 - dist / radii[i], 0.0, 1.0);


        col = mix(col, colors[i], alpha * colors[i].a);
    }

    COLOR = col;
}
