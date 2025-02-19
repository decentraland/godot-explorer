#ifndef INC_SATURATE
#define INC_SATURATE

float saturate(float val) {
	return clamp(val,0.0,1.0);
}

#endif
