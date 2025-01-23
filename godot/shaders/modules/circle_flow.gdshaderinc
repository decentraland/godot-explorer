
#ifndef INC_CIRCLE_FLOW
#define INC_CIRCLE_FLOW

#include "./cartesian2polar.gdshaderinc"

vec3 circle_flow(vec2 uv, float middle, float extents) {
	vec2 polar = cartesian2polar(uv);
	float influence = smoothstep(middle-extents,middle,polar.x) * smoothstep(middle+extents,middle,polar.x);
	return vec3(.5 * influence * vec2(-sin(polar.y),cos(polar.y)),0.0)+vec3(.5);
}

#endif
