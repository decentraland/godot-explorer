#ifndef INC_GAUSS_SOBEL
#define INC_GAUSS_SOBEL

#include "./gaussian.gdshaderinc"

float gauss_sobel(sampler2D tex, vec2 uv, vec2 pix_size) {
	float pix[9]; // 3 x 3

	// ガウシアンフィルタ
	for( int y=0; y<3; y ++ ) {
		for( int x=0; x<3; x ++ ) {
			pix[y*3+x] = gaussian(tex, uv + vec2(float(x-1), float(y-1) ) * pix_size, pix_size, vec4(1.0));
		}
	}

	vec2 sobel_src = vec2(
		(
			pix[0] * -1.0
		+	pix[3] * -2.0
		+	pix[6] * -1.0
		+	pix[2] * 1.0
		+	pix[5] * 2.0
		+	pix[8] * 1.0
		)
	,	(
			pix[0] * -1.0
		+	pix[1] * -2.0
		+	pix[2] * -1.0
		+	pix[6] * 1.0
		+	pix[7] * 2.0
		+	pix[8] * 1.0
		)
	);
	return clamp(sqrt(dot(sobel_src, sobel_src)), 0.0, 1.0 );
}

#endif
