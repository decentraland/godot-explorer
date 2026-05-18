shader_type canvas_item;
//@A shader by 刚学没几天的小策划@SL
uniform float power : hint_range(0.2, 4.0, 0.1) = 1.0;
uniform float up : hint_range(0.0, 1.0, 0.001) = 0.1;
uniform float down : hint_range(0.0, 1.0, 0.001) = 0.1;
uniform float left : hint_range(0.0, 1.0, 0.001) = 0.1;
uniform float right : hint_range(0.0, 1.0, 0.001) = 0.1;
uniform float up_clip : hint_range(0.0, 1.0, 0.001) = 0.0;
uniform float down_clip : hint_range(0.0, 1.0, 0.001) = 0.0;
uniform float left_clip : hint_range(0.0, 1.0, 0.001) = 0.0;
uniform float right_clip : hint_range(0.0, 1.0, 0.001) = 0.0;
//uniform float angel : hint_range(0.0, 90.0, 0.1) = 0.0;

void fragment() {
	vec2 _rotated_uv = UV;
	//TODO: Add an "angel" parameter to change the clipping direction
	
	float _hor = 1.0;
	if (_rotated_uv.x<left) {
		if (left_clip<left && left_clip > 0.0) {
			if (_rotated_uv.x<left_clip){_hor = 0.0;}
			else {_hor *= pow( (_rotated_uv.x-left_clip)/(left-left_clip), power);}
		}
		else{
			_hor *= pow( _rotated_uv.x/left, power);
		}
	}
	if (_rotated_uv.x>1.0-right) {
		if (right_clip<right && right_clip > 0.0) {
			if (1.0-_rotated_uv.x < right_clip){_hor = 0.0;}
			else {_hor *= pow( (1.0-_rotated_uv.x-right_clip)/(right-right_clip), power);}
		}
		else{
			_hor *= pow( (1.0-_rotated_uv.x)/right, power);
		}
	}
	
	float _ver = 1.0;
	if (_rotated_uv.y<up) {
		if (up_clip<up && up_clip > 0.0) {
			if (_rotated_uv.y<up_clip){_ver = 0.0;}
			else {_ver *= pow( (_rotated_uv.y-up_clip)/(up-up_clip), power);}
		}
		else{
			_ver *= pow( _rotated_uv.y/up, power);
		}
	}
	if (_rotated_uv.y>1.0-down) {
		if (right_clip<down && down_clip > 0.0) {
			if (1.0-_rotated_uv.y < down_clip){_ver = 0.0;}
			else {_ver= pow( (1.0-_rotated_uv.y-down_clip)/(down-down_clip), power);}
		}
		else{
			_ver *= pow( (1.0-_rotated_uv.y)/down, power);
		}
	}
	
	COLOR = texture(TEXTURE,UV);
	COLOR.a = min(_hor*_ver, texture(TEXTURE,UV).a);
}
