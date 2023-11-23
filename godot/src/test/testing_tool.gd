extends DclTestingTools
class_name TestingTools


func take_and_compare_snapshot(
	id: String,
	camera_position: Vector3,
	camera_target: Vector3,
	snapshot_frame_size: Vector2,
	tolerance: float,
	dcl_rpc_sender: DclRpcSender
):
	prints(
		"take_and_compare_snapshot",
		id,
		camera_position,
		camera_target,
		snapshot_frame_size,
		tolerance,
		dcl_rpc_sender
	)

	# TODO: make this configurable
	var hide_player := true
	var update_snapshot := false
	var create_snapshot_if_does_not_exist := true

	var snapshot_path := "user://snapshot_" + id.replace(" ", "_") + ".png"

	var existing_snapshot: Image = null
	if not update_snapshot and FileAccess.file_exists(snapshot_path):
		existing_snapshot = Image.load_from_file(snapshot_path)

	RenderingServer.set_default_clear_color(Color(0, 0, 0, 0))
	var viewport = get_viewport()
	var camera = viewport.get_camera_3d()
	var previous_camera_position = camera.global_position
	var previous_camera_rotation = camera.global_rotation
	var previous_viewport_size = viewport.size

	viewport.size = snapshot_frame_size
	camera.global_position = camera_position
	camera.look_at(camera_target)

	get_node("/root/explorer").set_visible_ui(false)
	if hide_player:
		get_node("/root/explorer/Player").hide()

	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame

	var viewport_img := viewport.get_texture().get_image()

	get_node("/root/explorer").set_visible_ui(true)
	if hide_player:
		get_node("/root/explorer/Player").show()

	viewport.size = previous_viewport_size
	camera.global_position = previous_camera_position
	camera.global_rotation = previous_camera_rotation

	var similarity := 0.0
	var updated := false

	if existing_snapshot != null:
		similarity = self.compute_image_similarity(existing_snapshot, viewport_img)
		prints("similarity factor ", similarity)

	if update_snapshot or (existing_snapshot == null and create_snapshot_if_does_not_exist):
		viewport_img.save_png(snapshot_path)
		updated = true

	(
		dcl_rpc_sender
		. send(
			{
				"is_match": similarity >= tolerance,
				"similarity": similarity,
				"was_exist": existing_snapshot != null,
				"replaced": updated,
			}
		)
	)
