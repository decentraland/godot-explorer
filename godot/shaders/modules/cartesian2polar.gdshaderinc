#ifndef INC_CARTESIAN2POLAR
#define INC_CARTESIAN2POLAR

vec2 cartesian2polar(vec2 uv) {
	return vec2(length(uv),atan(uv.y,uv.x));
}

#endif
