// Shader by Polysthetic (https://youtu.be/-c38gFbGO6U)
shader_type spatial;
render_mode blend_add,depth_draw_opaque,cull_back,diffuse_burley,specular_schlick_ggx;

uniform sampler2D texture_albedo;
uniform float numberOfFrames = 5.0f;
uniform float numberOfTracks = 3.0f;

varying float lifetime;
varying float trackNumber;

void vertex() {
	mat4 mat_world = mat4(normalize(VIEW_MATRIX[0])*length(MODEL_MATRIX[0]), normalize(VIEW_MATRIX[1])*length(MODEL_MATRIX[1]), normalize(VIEW_MATRIX[2])*length(MODEL_MATRIX[2]), MODEL_MATRIX[3]);
	mat_world = mat_world * mat4( vec4(cos(INSTANCE_CUSTOM.x),-sin(INSTANCE_CUSTOM.x), 0.0, 0.0), vec4(sin(INSTANCE_CUSTOM.x), cos(INSTANCE_CUSTOM.x), 0.0, 0.0),vec4(0.0, 0.0, 1.0, 0.0),vec4(0.0, 0.0, 0.0, 1.0));
	MODELVIEW_MATRIX = INV_VIEW_MATRIX * mat_world;
	
	lifetime = INSTANCE_CUSTOM.y;
	trackNumber = float((INSTANCE_ID) % int(numberOfTracks));
}

void fragment() {
	float frameCoordinate = floor(lifetime * numberOfFrames) / numberOfFrames;
	float trackCoordinate = trackNumber / numberOfTracks;
	vec2 scaledUV = vec2(1.0f / numberOfFrames, 1.0f / numberOfTracks);
	vec2 shiftedUV = vec2(frameCoordinate, trackCoordinate);

	vec4 colour = texture(texture_albedo, UV * scaledUV + shiftedUV);

	ALBEDO = vec3(0.0f);
	ALPHA = colour.a;
	EMISSION = colour.rgb;
}