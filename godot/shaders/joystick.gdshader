shader_type canvas_item;

uniform vec2 joystick_position;
uniform vec2 tip_position;
uniform bool show_guide;

varying vec2 pixel_pos;

void vertex() {
	pixel_pos = VERTEX;
}

float circle(vec2 uv, vec2 position, float radius, float feather)
{
	float dist = distance(uv, position);
	if (dist ==0.0) return 0.0;
	return 1.0 - smoothstep(radius, radius + feather, dist);
}

// 7 chain links
// 1 tip circle

void fragment() {
	vec2 control_size = pixel_pos / UV;
	vec2 canvas_point = UV * control_size;
	COLOR.a = 0.0;
	
	if (show_guide) {
		float tip_border = circle(
			canvas_point,
			joystick_position+tip_position,
			68.0,
			2.0);
		if (tip_border > 0.0) {
			COLOR = vec4(1.0, 1.0, 1.0, 1.0) * tip_border;
		}
		
		float tip = circle(
			canvas_point,
			joystick_position+tip_position,
			63.0,
			2.0);
		COLOR = mix(COLOR, vec4(0.0, 0.0, 0.0, 0.7), tip);
		
		float tip_center_border = circle(
			canvas_point,
			joystick_position+tip_position,
			27.0,
			2.0);
		COLOR = mix(COLOR, vec4(1.0, 1.0, 1.0, 1.0), tip_center_border);
		
		float tip_center = circle(
			canvas_point,
			joystick_position+tip_position,
			22.5,
			2.0);
		if (tip_center > 0.0) {
			COLOR = vec4(0.0, 0.0, 0.0, 0.7) * tip_center;
		}
	} else {
	
		float tip_distance = length(tip_position);
		vec2 tip_vector = normalize(tip_position) * -1.0;
		float link_separation = max(50.0, tip_distance / 8.0);
		
		for (int n = 0; n<=7; n++) {
			
			float chain_link_distance = max(0.0, tip_distance - (float(n+1) * link_separation));
			float link_alpha = smoothstep(00.0, 180.0, chain_link_distance);
			float link_radius = smoothstep(00.0, 200.0, chain_link_distance) * 20.0;
			
			if (link_alpha == 0.0) continue;
			
			float chain_link = circle(
				canvas_point,
				joystick_position - (tip_vector * chain_link_distance),
				link_radius,
				2.0);
			
			COLOR = mix(COLOR, vec4(0.0, 0.0, 0.0, link_alpha*0.3), chain_link);
		}
		
		float tip_border = circle(
			canvas_point,
			joystick_position+tip_position,
			53.0,
			2.0);
		if (tip_border > 0.0) {
			COLOR = vec4(1.0, 1.0, 1.0, 1.0) * tip_border;
		}
		
		float tip = circle(
			canvas_point,
			joystick_position+tip_position,
			50.0,
			2.0);
		COLOR = mix(COLOR, vec4(0.0, 0.0, 0.0, 0.4), tip);
		
		float tip_center = circle(
			canvas_point,
			joystick_position+tip_position,
			20.0,
			2.0);
		COLOR = mix(COLOR, vec4(1.0, 1.0, 1.0, 1.0), tip_center);
	}
}
