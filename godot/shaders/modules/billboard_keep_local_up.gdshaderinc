#ifndef INC_BILLBOARD_LOCAL_UP
#define INC_BILLBOARD_LOCAL_UP

mat4 billboard_keep_local_up(mat4 world_mat, mat4 camera_mat) {
	vec3 origin = world_mat[3].xyz;
	vec3 cam_origin = camera_mat[3].xyz;

	vec3 cam_to_mesh = normalize(origin-cam_origin);
	mat3 basis = mat3(world_mat);
	vec3 local_up = normalize(basis[1]);
	vec3 projected = dot(cam_to_mesh,local_up)*local_up;
	vec3 target_z = -normalize(cam_to_mesh-projected);
	vec3 target_x = -cross(target_z, local_up);

	return mat4(
		vec4(target_x,0.0)*length(basis[0]),
		vec4(local_up,0.0)*length(basis[1]),
		vec4(target_z,0.0)*length(basis[2]),
		world_mat[3]
	);
}

#endif
