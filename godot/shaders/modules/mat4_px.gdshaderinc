#ifndef INC_MAT4_PX
#define INC_MAT4_PX

#include "./number_px.gdshaderinc"

vec3 DrawMat4AtPxPos(vec2 pxCoord, vec2 pxPos, mat4 mat, float scale, int decimalCount) {
	pxCoord+=vec2(50.0,-50.0)*scale;
	float acc = 0.0;
	float sep= 20.0 + 15.0 * scale;
	acc += DrawNumberAtPxPos(pxCoord, pxPos - vec2(-0.0 * sep, 0.0 * sep), mat[0][0], scale,decimalCount);
	acc += DrawNumberAtPxPos(pxCoord, pxPos - vec2(-0.0 * sep, 1.0 * sep), mat[1][0], scale,decimalCount);
	acc += DrawNumberAtPxPos(pxCoord, pxPos - vec2(-0.0 * sep, 2.0 * sep), mat[2][0], scale,decimalCount);
	acc += DrawNumberAtPxPos(pxCoord, pxPos - vec2(-0.0 * sep, 3.0 * sep), mat[3][0], scale,decimalCount);
	acc += DrawNumberAtPxPos(pxCoord, pxPos - vec2(-1.0 * sep, 0.0 * sep), mat[0][1], scale,decimalCount);
	acc += DrawNumberAtPxPos(pxCoord, pxPos - vec2(-1.0 * sep, 1.0 * sep), mat[1][1], scale,decimalCount);
	acc += DrawNumberAtPxPos(pxCoord, pxPos - vec2(-1.0 * sep, 2.0 * sep), mat[2][1], scale,decimalCount);
	acc += DrawNumberAtPxPos(pxCoord, pxPos - vec2(-1.0 * sep, 3.0 * sep), mat[3][1], scale,decimalCount);
	acc += DrawNumberAtPxPos(pxCoord, pxPos - vec2(-2.0 * sep, 0.0 * sep), mat[0][2], scale,decimalCount);
	acc += DrawNumberAtPxPos(pxCoord, pxPos - vec2(-2.0 * sep, 1.0 * sep), mat[1][2], scale,decimalCount);
	acc += DrawNumberAtPxPos(pxCoord, pxPos - vec2(-2.0 * sep, 2.0 * sep), mat[2][2], scale,decimalCount);
	acc += DrawNumberAtPxPos(pxCoord, pxPos - vec2(-2.0 * sep, 3.0 * sep), mat[3][2], scale,decimalCount);
	acc += DrawNumberAtPxPos(pxCoord, pxPos - vec2(-3.0 * sep, 0.0 * sep), mat[0][3], scale,decimalCount);
	acc += DrawNumberAtPxPos(pxCoord, pxPos - vec2(-3.0 * sep, 1.0 * sep), mat[1][3], scale,decimalCount);
	acc += DrawNumberAtPxPos(pxCoord, pxPos - vec2(-3.0 * sep, 2.0 * sep), mat[2][3], scale,decimalCount);
	acc += DrawNumberAtPxPos(pxCoord, pxPos - vec2(-3.0 * sep, 3.0 * sep), mat[3][3], scale,decimalCount);
	return vec3(acc);
}

#endif
