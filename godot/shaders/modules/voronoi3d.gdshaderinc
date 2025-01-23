#ifndef INC_VORONOI3D
#define INC_VORONOI3D

float random(vec2 st, float seed) {
    return fract(sin(dot(st.xy, vec2(12.9898,78.233)+seed))* 43758.5453123);
}

vec3 random_center(vec3 cell_id, float seed) {
	return vec3(random(cell_id.xy,1.0+seed),random(cell_id.zy,78.12839+seed*2.0),random(cell_id.zx,128.59+seed*3.0));
}

vec4 voronoi3d(vec3 p){ 
	vec3 cell_id = floor(p);
	vec3 cell_coords = fract(p);
	vec4 val = vec4(99999999.0);
	for (float i = -1.0; i < 2.0; i+=1.0) 
	for (float j = -1.0; j < 2.0; j+=1.0) 
	for (float k = -1.0; k < 2.0; k+=1.0) {
		vec3 ijk=vec3(i,j,k);
		for (float seed = 0.0; seed < 3.0; seed+=1.0) {
			vec3 cell_center = random_center(cell_id+ijk,seed)+ijk;
			float new_min = length((cell_coords-cell_center)*2.0);
			float cond = step(val.w, new_min);
			val = vec4(mix(cell_id+ijk,val.xyz,cond), min(new_min,val.w));
		}
	}
	return val;
}

#endif
