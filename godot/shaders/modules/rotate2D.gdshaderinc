#ifndef INC_ROTATE2D
#define INC_ROTATE2D

vec2 rotate2D(vec2 v, float a) {
    float s = sin(a);
    float c = cos(a);
    mat2 m = mat2(vec2(c,-s),vec2(s,c));
    return m * v;
}

#endif
