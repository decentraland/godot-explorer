#ifndef INC_GAUSSIAN
#define INC_GAUSSIAN

const vec3 MONOCHROME_SCALE = vec3( 0.298912, 0.586611, 0.114478 );
const float GAUSS_KERNEL[25] = { 0.00390625, 0.015625, 0.0234375, 0.015625, 0.00390625, 0.015625, 0.0625, 0.09375, 0.0625, 0.015625, 0.0234375, 0.09375, 0.140625, 0.09375, 0.0234375, 0.015625, 0.0625, 0.09375, 0.0625, 0.015625, 0.00390625, 0.015625, 0.0234375, 0.015625, 0.00390625 };

float gaussian( sampler2D tex, vec2 uv, vec2 pix_size, vec4 mask)
{
	float p = 0.0;

	for( int y=-2; y<=2; y++ ) {
		for( int x=-2; x<=2; x ++ ) {
			p += dot(
				MONOCHROME_SCALE, 
				(
					texture( 
						tex, 
						uv + vec2(float(x), float(y) ) * pix_size
					) * mask
				).rgb 
			) * GAUSS_KERNEL[(y+2)*5 + (x+2)];
		}
	}

	return p;
}

vec4 gaussian_col(sampler2D tex, vec2 uv, vec2 pix_size)
{
	vec4 p = vec4(.0);

	for( int y=-2; y<=2; y++) 
	for( int x=-2; x<=2; x++) {
		p += texture(tex, uv + vec2(float(x), float(y)) * pix_size) * GAUSS_KERNEL[(y+2)*5 + (x+2)];
	}
	return p;
}

#endif
