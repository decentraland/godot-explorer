shader_type canvas_item;

uniform float scale = 3.0;
uniform float time_scale = 3.5;
uniform vec4 bar_color : source_color = vec4(1, 0, 0, 1);
uniform vec4 background_color : source_color = vec4(1, 0, 0, 1);

void vertex() {
	// Called for every vertex the material is visible on.
}

float interpolation(float value, float in_from, float in_to, float out_from, float out_to) {
	float t = clamp((value - in_from) / (in_to - in_from), 0.0, 1.0);
	return out_from + t * (out_to - out_from);
}

void fragment() {
	float level = sin(UV.x*scale + TIME*time_scale);
	level = interpolation(level, 0.92, 1.0, 0.0, 1.0);
	//COLOR = mix(background_color, bar_color, level);
	if (COLOR.a > 0.0) {
		COLOR += mix(vec4(0,0,0,0), bar_color, level);
	}
	
}
