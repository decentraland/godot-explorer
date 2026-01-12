shader_type canvas_item;

uniform vec2 joystick_position;
uniform vec2 tip_position;
uniform int state;

uniform float tip_radius;
uniform float tip_center_radius;
uniform float bubble_radius;
uniform float max_bubble_distance;
uniform float max_chain_length;

varying vec2 pixel_pos;

const int link_amount = 6;
const float link_max_alpha_at = 0.9;
const float link_max_size_at = 0.9;

void vertex() {
	pixel_pos = VERTEX;
}

float circle(vec2 uv, vec2 position, float radius, float feather)
{
	float dist = distance(uv, position);
	if (dist == 0.0) return 0.0;
	return 1.0 - smoothstep(radius, radius + feather, dist);
}

float interpolation(float value, float in_from, float in_to, float out_from, float out_to) {
	float t = clamp((value - in_from) / (in_to - in_from), 0.0, 1.0);
	return out_from + t * (out_to - out_from);
}

// 7 chain links
// 1 tip circle

void fragment() {
	vec2 control_size = pixel_pos / UV;
	vec2 canvas_point = UV * control_size;
	vec2 tip_limited_position = tip_position;
	if (length(tip_limited_position) > max_chain_length) {
		tip_limited_position = normalize(tip_limited_position) * max_chain_length;
	}
	
	COLOR.a = 0.0;
	
	if (state == 0) {
		float tip_border = circle(
			canvas_point,
			joystick_position+tip_limited_position,
			68.0,
			2.0);
		if (tip_border > 0.0) {
			COLOR = vec4(1.0, 1.0, 1.0, 1.0) * tip_border;
		}
		
		float tip = circle(
			canvas_point,
			joystick_position+tip_limited_position,
			63.0,
			2.0);
		COLOR = mix(COLOR, vec4(0.0, 0.0, 0.0, 0.7), tip);
		
		float tip_center_border = circle(
			canvas_point,
			joystick_position+tip_limited_position,
			27.0,
			2.0);
		COLOR = mix(COLOR, vec4(1.0, 1.0, 1.0, 1.0), tip_center_border);
		
		float tip_center = circle(
			canvas_point,
			joystick_position+tip_limited_position,
			22.5,
			2.0);
		if (tip_center > 0.0) {
			COLOR = vec4(0.0, 0.0, 0.0, 0.7) * tip_center;
		}
	} else if (state == 1) {
	
		float tip_distance = length(tip_limited_position) / max_chain_length;
		vec2 tip_vector = normalize(tip_limited_position) * -1.0;
		float link_separation = max(max_bubble_distance, tip_distance / float(link_amount+1));
		
		for (int n = 0; n<=link_amount-1; n++) {
			
			float chain_link_distance = max(0.0, tip_distance - (float(n+1) * 1.0/float(link_amount+1)) - (tip_radius / max_chain_length) * 0.6);
			if (chain_link_distance == 0.0) continue;
			
			float link_alpha = interpolation(chain_link_distance, 0.0, link_max_alpha_at, 0.25, 0.8);
			float link_radius = interpolation(chain_link_distance, 0.0, link_max_size_at, 5.0, bubble_radius);
			
			float chain_link = circle(
				canvas_point,
				joystick_position - (tip_vector * chain_link_distance * max_chain_length),
				link_radius,
				2.0);
			
			COLOR = mix(COLOR, vec4(0.0, 0.0, 0.0, link_alpha*0.3), chain_link);
		}
		
		float tip_border = circle(
			canvas_point,
			joystick_position+tip_limited_position,
			tip_radius + 3.0,
			2.0);
		if (tip_border > 0.0) {
			COLOR = vec4(1.0, 1.0, 1.0, 1.0) * tip_border;
		}
		
		float tip = circle(
			canvas_point,
			joystick_position+tip_limited_position,
			tip_radius,
			2.0);
		COLOR = mix(COLOR, vec4(0.0, 0.0, 0.0, 0.4), tip);
		
		float tip_center = circle(
			canvas_point,
			joystick_position+tip_limited_position,
			tip_center_radius,
			2.0);
		COLOR = mix(COLOR, vec4(1.0, 1.0, 1.0, 1.0), tip_center);
	}
}
