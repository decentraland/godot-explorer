extends Node

# Regression test for #2685 (tiny avatars in the profile screen).
#
# AvatarPreview._fit_to_overall can commit at a transient viewport size while
# the profile layout is still settling (placeholder swap, safe-area insets).
# _on_resized used to refit only while _pending_fit_overall was set, so once
# that first fit committed, a later resize to the final size left the ortho
# camera size tuned for the smaller viewport and the avatar rendered tiny.
# The fix keeps the initial fit live on resize until the user pans/zooms.
#
# Runs as a scene (not --script) so autoloads exist and avatar_preview.gd,
# which references the Global autoload, compiles:
#   .bin/godot/godot4_bin --headless --path godot \
#     res://src/test/ui/test_avatar_preview_refit_on_resize.tscn

const AvatarPreviewScene := preload("res://src/ui/pages/backpack/avatar_preview.tscn")

# Full-body AABB of an idle avatar in local space (~1.8m tall).
const FIT_AABB := AABB(Vector3(-0.4, 0.0, -0.4), Vector3(0.8, 1.8, 0.8))
# Same margins the profile screens configure.
const MARGIN_TOP := 64
const MARGIN_BOTTOM := 46

var _failures: Array[String] = []


func _ready() -> void:
	_test_refit_when_viewport_grows_after_initial_fit()
	_test_no_refit_after_user_pan()
	_finish()


func _make_preview() -> AvatarPreview:
	var preview: AvatarPreview = AvatarPreviewScene.instantiate()
	# The scene file pins full-rect anchors, which would override manual sizes.
	preview.set_anchors_preset(Control.PRESET_TOP_LEFT)
	preview.preview_margin_top = MARGIN_TOP
	preview.preview_margin_bottom = MARGIN_BOTTOM
	add_child(preview)
	# Fake a loaded avatar: async_on_avatar_loaded would fill these from the
	# real meshes; the fit math only reads the "overall" entry.
	preview._cached_aabbs = {"overall": FIT_AABB}
	return preview


func _expected_cam_size(vp: Vector2) -> float:
	var inner_h: float = vp.y - MARGIN_TOP - MARGIN_BOTTOM
	var inner_w: float = vp.x
	return maxf(
		maxf(FIT_AABB.size.y * vp.y / inner_h, FIT_AABB.size.x * vp.x / inner_w),
		AvatarPreview.MIN_CAMERA_SIZE_OVERALL
	)


func _test_refit_when_viewport_grows_after_initial_fit() -> void:
	var preview := _make_preview()
	_assert(preview != null, "preview instantiates")
	if preview == null:
		return
	# Layout settles through a transient short viewport: the fit commits here.
	# (Height can't go below 500: stretch=true clamps to the SubViewport size.)
	preview.size = Vector2(390, 500)
	preview._fit_to_overall()
	var transient_cam: float = preview._target_camera_size
	_assert(
		is_equal_approx(transient_cam, _expected_cam_size(Vector2(390, 500))),
		"initial fit commits at the transient size"
	)
	# Final layout size arrives: the camera must refit or the avatar is tiny.
	preview.size = Vector2(390, 844)
	_assert(
		is_equal_approx(preview._target_camera_size, _expected_cam_size(Vector2(390, 844))),
		(
			"resize to the final viewport refits the camera (got %f, transient was %f)"
			% [preview._target_camera_size, transient_cam]
		)
	)
	preview.queue_free()


func _test_no_refit_after_user_pan() -> void:
	var preview := _make_preview()
	if preview == null:
		return
	preview.size = Vector2(390, 500)
	preview._fit_to_overall()
	var transient_cam: float = preview._target_camera_size
	# A user-driven pan/zoom takes ownership of the camera: resizes must not
	# snap it back to the auto fit.
	preview._user_has_panned = true
	preview.size = Vector2(390, 844)
	_assert(
		is_equal_approx(preview._target_camera_size, transient_cam),
		"resize does not refit once the user has panned"
	)
	preview.queue_free()


func _assert(condition: bool, label: String) -> void:
	if condition:
		print("PASS: ", label)
	else:
		_failures.append(label)
		printerr("FAIL: ", label)


func _finish() -> void:
	if _failures.is_empty():
		print("[test_avatar_preview_refit_on_resize] PASS")
		get_tree().quit(0)
	else:
		printerr("[test_avatar_preview_refit_on_resize] FAIL: %d case(s)" % _failures.size())
		get_tree().quit(1)
