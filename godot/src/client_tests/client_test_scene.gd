extends Node3D

# Client test for avatar rendering with snapshot comparison

var avatar_preview_instance: Control
var snapshot_folder: String = ""
var snapshot_comparison_folder: String = ""
var test_results: Array = []


func _ready():
	print("🔧 Client Test Mode - Avatar Snapshot Testing")

	# Setup snapshot folders
	_setup_snapshot_folders()

	# Disable all unnecessary systems
	if Global.has_method("set_scene_log_enabled"):
		Global.set_scene_log_enabled(false)

	if Global.has_method("scene_runner") and Global.scene_runner:
		Global.scene_runner.set_pause(true)

	# Create avatar preview directly
	var AvatarPreviewScene = preload("res://src/ui/components/backpack/avatar_preview.tscn")
	avatar_preview_instance = AvatarPreviewScene.instantiate()
	avatar_preview_instance.hide_name = true
	avatar_preview_instance.can_move = false
	avatar_preview_instance.with_light = false
	add_child(avatar_preview_instance)

	# Apply visual enhancements
	var viewport: Viewport = avatar_preview_instance.get_node("SubViewport")
	viewport.use_debanding = true
	viewport.scaling_3d_scale = 2.0
	RenderingServer.screen_space_roughness_limiter_set_active(true, 4.0, 1.0)

	# Wait a bit then load a test avatar
	await get_tree().create_timer(1.0).timeout
	_load_test_avatar()


func _setup_snapshot_folders():
	if OS.has_feature("editor"):
		snapshot_folder = ProjectSettings.globalize_path("res://../tests/snapshots/client/")
	else:
		snapshot_folder = OS.get_user_data_dir() + "/snapshots/client/"

	if not snapshot_folder.ends_with("/"):
		snapshot_folder += "/"

	snapshot_comparison_folder = snapshot_folder + "comparison/"

	if not DirAccess.dir_exists_absolute(snapshot_folder):
		DirAccess.make_dir_recursive_absolute(snapshot_folder)

	if not DirAccess.dir_exists_absolute(snapshot_comparison_folder):
		DirAccess.make_dir_recursive_absolute(snapshot_comparison_folder)

	prints('snapshot_folder="' + snapshot_folder + '"')
	prints('snapshot_comparison_folder="' + snapshot_comparison_folder + '"')


func _load_test_avatar():
	print("Loading test avatar...")

	# Create a test avatar using DclUserProfile for proper structure
	var profile: DclUserProfile = DclUserProfile.new()
	var test_avatar_data = profile.get_avatar()

	test_avatar_data.set_body_shape("urn:decentraland:off-chain:base-avatars:BaseFemale")
	test_avatar_data.set_skin_color(Color(0.490, 0.365, 0.278))
	test_avatar_data.set_hair_color(Color(0.596, 0.373, 0.216))
	test_avatar_data.set_eyes_color(Color(0.3, 0.8, 0.5))

	# Set wearables as PackedStringArray
	test_avatar_data.set_wearables(
		PackedStringArray(
			[
				"urn:decentraland:off-chain:base-avatars:f_sweater",
				"urn:decentraland:off-chain:base-avatars:f_jeans",
				"urn:decentraland:off-chain:base-avatars:bun_shoes",
				"urn:decentraland:off-chain:base-avatars:standard_hair",
				"urn:decentraland:off-chain:base-avatars:f_eyes_01",
				"urn:decentraland:off-chain:base-avatars:f_eyebrows_00"
			]
		)
	)

	# Update avatar
	print("Updating avatar...")
	await avatar_preview_instance.avatar.async_update_avatar(test_avatar_data, "")
	print("Avatar updated!")

	# Disable animations to ensure consistent snapshots
	# This is the same approach used in avatar_preview.gd's async_get_viewport_image
	avatar_preview_instance.avatar.emote_controller.freeze_on_idle()
	avatar_preview_instance.avatar.rotation.y = 0.0

	# Wait a bit then run tests
	await get_tree().create_timer(2.0).timeout

	# Test without outline
	var result_no_outline = await _capture_and_compare_avatar("avatar_no_outline")
	test_results.push_back(result_no_outline)

	# Enable outline and test again
	avatar_preview_instance.enable_outline()
	print("Outline enabled!")
	await get_tree().create_timer(1.0).timeout

	var result_with_outline = await _capture_and_compare_avatar("avatar_with_outline")
	test_results.push_back(result_with_outline)

	# Report results
	_report_results()

	# Exit after tests
	print("\nTest complete. Exiting...")
	get_tree().quit()


func _capture_and_compare_avatar(test_name: String) -> Dictionary:
	print("Testing %s..." % test_name)

	var viewport: Viewport = avatar_preview_instance.get_node("SubViewport")

	# Wait for rendering
	await RenderingServer.frame_post_draw
	await get_tree().process_frame
	await get_tree().process_frame

	var captured_image = viewport.get_texture().get_image()

	# Setup paths
	var base_path = test_name + ".png"
	var current_snapshot_path = snapshot_comparison_folder + base_path
	var existing_snapshot_path = snapshot_folder + base_path
	var diff_path = snapshot_comparison_folder + test_name + ".diff.png"

	# Save current capture
	captured_image.save_png(current_snapshot_path)
	print("Current snapshot saved to: ", current_snapshot_path)

	# Check if existing snapshot exists
	var result = {
		"test_name": test_name, "stored_snapshot_found": false, "similarity": 0.0, "passed": false
	}

	if FileAccess.file_exists(existing_snapshot_path):
		result.stored_snapshot_found = true
		var existing_snapshot = Image.load_from_file(existing_snapshot_path)

		# Compare images
		var similarity = _compute_image_similarity(existing_snapshot, captured_image, diff_path)
		result.similarity = similarity
		result.passed = similarity > 0.99  # 99% similarity threshold

		if result.passed:
			print("✅ %s: PASSED (similarity: %.2f%%)" % [test_name, similarity * 100])
		else:
			print("❌ %s: FAILED (similarity: %.2f%%)" % [test_name, similarity * 100])
			print("   Diff saved to: ", diff_path)
	else:
		print("⚠️ %s: No reference snapshot found at %s" % [test_name, existing_snapshot_path])
		print("   Creating initial snapshot...")
		# Save as the reference snapshot for future runs
		captured_image.save_png(existing_snapshot_path)

	return result


func _compute_image_similarity(image_a: Image, image_b: Image, diff_path: String) -> float:
	if image_a.get_size() != image_b.get_size():
		print("Images have different sizes!")
		return 0.0

	var width = image_a.get_width()
	var height = image_a.get_height()
	var diff_image = Image.create(width, height, false, Image.FORMAT_RGBA8)

	var matching_pixels = 0
	var total_pixels = width * height

	for y in range(height):
		for x in range(width):
			var pixel_a = image_a.get_pixel(x, y)
			var pixel_b = image_b.get_pixel(x, y)

			# Calculate difference
			var diff = (
				abs(pixel_a.r - pixel_b.r)
				+ abs(pixel_a.g - pixel_b.g)
				+ abs(pixel_a.b - pixel_b.b)
				+ abs(pixel_a.a - pixel_b.a)
			)

			if diff < 0.01:  # Tolerance for floating point comparison
				matching_pixels += 1
				diff_image.set_pixel(x, y, Color.BLACK)
			else:
				# Highlight differences in red
				diff_image.set_pixel(x, y, Color.RED)

	# Save diff image
	diff_image.save_png(diff_path)

	return float(matching_pixels) / float(total_pixels)


func _report_results():
	var separator = "=================================================="
	print("\n" + separator)
	print("CLIENT TEST RESULTS")
	print(separator)

	var all_passed = true
	for result in test_results:
		if result.stored_snapshot_found:
			if result.passed:
				print("✅ %s: PASSED" % result.test_name)
			else:
				print(
					(
						"❌ %s: FAILED (similarity: %.2f%%)"
						% [result.test_name, result.similarity * 100]
					)
				)
				all_passed = false
		else:
			print("⚠️ %s: SKIPPED (no reference snapshot)" % result.test_name)

	print(separator)
	if all_passed:
		print("All client tests passed!")
	else:
		print("Some client tests failed!")
	print(separator)
