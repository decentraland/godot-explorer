#ifndef INC_VORONOI_EDGES
#define INC_VORONOI_EDGES

float random(vec2 st, float seed) {
    return fract(sin(dot(st.xy, vec2(12.9898,78.233)+seed))* 43758.5453123);
}

float seed_id(vec2 cell_id) {
	return dot(cell_id,vec2(1.0,78.12839)) + cell_id.x + cell_id.y;
}

vec2 random_center(vec2 cell_id, float seed) {
	return vec2(random(cell_id.xy,1.0+seed),random(cell_id.yx,78.12839+seed*2.0));
}

float smin(float a, float b, float k) {
    float h = max(k-abs(a-b),0.0);
    return min(a, b) - h*h*0.25/k ;
}


// Inigo quilez FTW 
// https://iquilezles.org/articles/voronoilines/
vec4 voronoiEdges( vec2 p, float smoothness, float skip_t, float smoothness_variation, float dissolve_t, out vec2 center)
{
	vec2 cell_id = floor(p);
	vec2 cell_coords = fract(p);

	float dist = 9999999.0;
	vec2 closest_offset = vec2(0.0);
	vec2 closest_center = vec2(0.0);

	for (float i = -1.0; i < 2.0; i+=1.0) 
	for (float j = -1.0; j < 2.0; j+=1.0) {
		vec2 ij = vec2(i,j);
		vec2 this_id = cell_id+ij;
		float seed = seed_id(this_id);
		if (fract(seed) < skip_t) continue;
		vec2 cell_center = random_center(this_id, seed)+ij - cell_coords;
		float new_min = dot(cell_center, cell_center);

		if (new_min < dist) {
			dist = new_min;
			closest_offset = ij;
			closest_center = cell_center;
		}
	}

	dist = 999999.0;

	for (float i = -1.0; i < 2.0; i+=1.0) 
	for (float j = -1.0; j < 2.0; j+=1.0) {
		vec2 ij = vec2(i,j) + closest_offset;
		vec2 this_id = cell_id+ij;
		float seed=seed_id(this_id);
		if (fract(seed) < skip_t) continue;
		vec2 cell_center = random_center(this_id, seed)+ij - cell_coords;
		float new_min = dot(0.5 * (closest_center + cell_center), normalize(cell_center - closest_center));
		dist = smin(dist, new_min, max(smoothness + fract(seed)*smoothness_variation, 0.00001));
	}
	vec2 final_id = cell_id+closest_offset;
	float seed= fract(seed_id(final_id));
	center = random_center(final_id, seed);
    return vec4(final_id,seed, dist * step(dissolve_t,seed));
}

#endif
