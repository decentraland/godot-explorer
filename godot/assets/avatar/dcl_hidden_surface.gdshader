shader_type spatial;
render_mode unshaded;

// Draws nothing in any pass (colour, depth or shadow): every vertex collapses to one point.
void vertex() {
	VERTEX = vec3(0.0);
}
