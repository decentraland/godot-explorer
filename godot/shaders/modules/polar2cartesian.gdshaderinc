#ifndef INC_POLAR2CARTESIAN
#define INC_POLAR2CARTESIAN

vec2 polar2cartesian(vec2 rt) {
	return vec2(rt.x * cos(rt.y),rt.x * sin(rt.y));
}

#endif
