shader_type canvas_item;

varying vec2 world_position;
uniform mat4 global_transform;
uniform sampler2D map_data;

uniform float size = 16.0;
uniform float line_width_px = 1.0;

uniform vec2 selected_tile = vec2(20.0, 20.0);

const vec3[] colors = {
	vec3(0.0, 0.0, 0.0), // without
	vec3(0.314,0.329,0.831), // district
	vec3(0.439,0.675,0.463), // plaza
	vec3(0.443,0.424,0.478), // road
	vec3(0.239,0.227,0.275), // onwed
	vec3(0.035,0.031,0.039), // unowned
	vec3(0.0, 0.0, 0.0),
	vec3(0.0, 0.0, 0.0)
};

void vertex() {
	world_position = VERTEX;
}

void fragment() {
	float line_width = line_width_px / size;
	vec2 frag_position = floor(world_position);
	float fx = frag_position.x / size, fy = (frag_position.y / size) + 1.0;
	float cx = floor(fx), cy = floor(fy);

	vec4 pixel_data = texelFetch(map_data, ivec2(int(cx), int(cy)), 0);
	int flagsR = int(pixel_data.r * 255.0);
	int flagsG = int(pixel_data.g * 255.0);

	bool topMask = (flagsR & 0x8) > 0;
	bool leftMask = (flagsR & 0x10) > 0;

	vec3 parcel_color;
	if (flagsG == 32) {
		parcel_color = colors[1];
	} else if (flagsG == 64) {
		parcel_color = colors[3];
	} else if (flagsG == 128) {
		parcel_color = colors[4];
	} else {
		parcel_color = colors[2];
	}

	vec4 resolved_color = vec4(parcel_color, COLOR.a);

	bool borderLeft = false;
	bool borderTop = false;

	if (topMask == false && leftMask == false) {
		borderLeft = true;
		borderTop = true;
	} else if (topMask && leftMask) {
		borderLeft = false;
		borderTop = false;
	} else {
		if (topMask == false) {
			borderTop = true;
		}
		if (leftMask == false) {
			borderLeft = true;
		}
	}

	if (borderLeft && (fx - cx < line_width)) {
		resolved_color = vec4(0.0, 0.0, 0.0, COLOR.a);
	}

	if (borderTop && (fy - cy < line_width)) {
		resolved_color = vec4(0.0, 0.0, 0.0, COLOR.a);
	}

	if (selected_tile.x == cx && selected_tile.y == cy) {
		resolved_color += vec4(0.7, 0.1, 0.1, COLOR.a);
	}

	COLOR = resolved_color;
}