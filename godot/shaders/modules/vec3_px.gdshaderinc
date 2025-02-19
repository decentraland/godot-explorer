#ifndef INC_VEC3PX
#define INC_VEC3PX

#include "./number_px.gdshaderinc"

vec3 DrawVec3AtPxPos(vec2 pxCoord, vec2 pxPos, vec3 vec, float scale, int decimalCount) {
	pxCoord += vec2(0.0,-5.0-5.0*scale);
	return DrawNumberAtPxPos(pxCoord, pxPos, vec.x,scale,decimalCount) * vec3(.8,0.2,0.2) +
		DrawNumberAtPxPos(pxCoord, pxPos - vec2(0.0,5.0 + 5.0*scale), vec.y,scale,decimalCount) * vec3(0.2,0.8,0.2) +
		DrawNumberAtPxPos(pxCoord, pxPos - vec2(0.0,10.0 + 5.0*scale * 2.0), vec.z,scale,decimalCount) * vec3(0.2,0.2,0.8);
}

#endif
