class_name TestingTools
extends DclTestingTools


func async_take_and_compare_snapshot(
	scene_id: int,
	src_stored_snapshot: String,
	camera_position: Vector3,
	camera_target: Vector3,
	screenshot_size: Vector2,
	method: Dictionary,
	dcl_rpc_sender: DclRpcSenderTakeAndCompareSnapshotResponse
):
	prints(
		"async_take_and_compare_snapshot",
		scene_id,
		src_stored_snapshot,
		camera_position,
		camera_target,
		screenshot_size,
		method,
		dcl_rpc_sender
	)

	# TODO: make this configurable
	var hide_player := true

	var base_path := src_stored_snapshot.replace(" ", "_").replace("/", "_").replace("\\", "_").to_lower()
	var snapshot_path := "user://snapshot_" + base_path
	if not snapshot_path.ends_with(".png"):
		snapshot_path += ".png"

	RenderingServer.set_default_clear_color(Color(0, 0, 0, 0))
	var viewport = get_viewport()
	var camera = viewport.get_camera_3d()
	var previous_camera_position = camera.global_position
	var previous_camera_rotation = camera.global_rotation
	var previous_viewport_size = viewport.size

	viewport.size = screenshot_size
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

	var existing_snapshot: Image = null
	var content_mapping = Global.scene_runner.get_scene_content_mapping(scene_id)
	var promise = Global.content_manager.fetch_texture(src_stored_snapshot, content_mapping)
	var res = await promise.async_awaiter()

	if res is Promise.Error:
		printerr("Fetch snapshot texture error, doesn't it exist?")
	else:
		existing_snapshot = Global.content_manager.get_image_from_texture_or_null(
			src_stored_snapshot, content_mapping
		)

	viewport_img.save_png(snapshot_path)

	var result = {"stored_snapshot_found": existing_snapshot != null}
	if existing_snapshot != null:
		compare(method, existing_snapshot, viewport_img, result)

	dcl_rpc_sender.send(result)


func compare(method: Dictionary, image_a: Image, image_b: Image, result: Dictionary) -> void:
	if method.get("grey_pixel_diff") != null:
		var similarity = self.compute_image_similarity(image_a, image_b)
		result["grey_pixel_diff"] = {"similarity": similarity}

func _process(delta):
	#if not is_test_mode_active():
		#return
		
	if Global.scene_fetcher.is_parcel_scene_loaded(52, -52):
		if Global.scene_runner.scene_tests_finished():
			get_tree().quit()
	
