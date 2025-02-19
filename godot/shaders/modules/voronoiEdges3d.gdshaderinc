#ifndef INC_VORONOI3D_EDGES
#define INC_VORONOI3D_EDGES

float random(vec2 st, float seed) {
    return fract(sin(dot(st.xy, vec2(12.9898,78.233)+seed))* 43758.5453123);
}

float seed_id(vec3 cell_id) {
	return dot(cell_id,vec3(1.0,78.12839,128.59)) + cell_id.x + cell_id.y + cell_id.z;
}

vec3 random_center(vec3 cell_id, float seed) {
	return vec3(random(cell_id.xy,1.0+seed),random(cell_id.zy,78.12839+seed*2.0),random(cell_id.zx,128.59+seed*3.0));
}

float smin(float a, float b, float k) {
    float h = max(k-abs(a-b),0.0);
    return min(a, b) - h*h*0.25/k ;
}


// Inigo quilez FTW 
// https://iquilezles.org/articles/voronoilines/
vec4 voronoiEdges( vec3 p, float smoothness, float skip_t, float smoothness_variation, float dissolve_t)
{
	vec3 cell_id = floor(p);
	vec3 cell_coords = fract(p);

	float dist = 9999999.0;
	vec3 closest_offset = vec3(0.0);
	vec3 closest_center = vec3(0.0);

	for (float i = -1.0; i < 2.0; i+=1.0) 
	for (float j = -1.0; j < 2.0; j+=1.0) 
	for (float k = -1.0; k < 2.0; k+=1.0) {
		vec3 ijk = vec3(i,j,k);
		vec3 this_id = cell_id+ijk;
		float seed = seed_id(this_id);
		if (fract(seed) < skip_t) continue;
		vec3 cell_center = random_center(this_id, seed)+ijk - cell_coords;
		float new_min = dot(cell_center, cell_center);

		if (new_min < dist) {
			dist = new_min;
			closest_offset = ijk;
			closest_center = cell_center;
		}
	}

	dist = 999999.0;

	for (float i = -1.0; i < 2.0; i+=1.0) 
	for (float j = -1.0; j < 2.0; j+=1.0) 
	for (float k = -1.0; k < 2.0; k+=1.0) {
		vec3 ijk = vec3(i,j,k) + closest_offset;
		vec3 this_id = cell_id+ijk;
		float seed=seed_id(this_id);
		if (fract(seed) < skip_t) continue;
		vec3 cell_center = random_center(this_id, seed)+ijk - cell_coords;
		float new_min = dot(0.5 * (closest_center + cell_center), normalize(cell_center - closest_center));
		dist = smin(dist, new_min, max(smoothness + fract(seed)*smoothness_variation, 0.00001));
	}
	vec3 final_id = cell_id+closest_offset;
    return vec4(final_id, dist * step(dissolve_t,fract(seed_id(final_id))));
}

#endif
