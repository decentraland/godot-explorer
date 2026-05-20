shader_type canvas_item;

uniform sampler2D tex:source_color;

void fragment() {
	if (COLOR.a == 1.0) {
		COLOR = texture(tex, UV);
	}
}
