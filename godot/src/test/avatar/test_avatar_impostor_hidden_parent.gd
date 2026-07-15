extends SceneTree

# Regression test for the phantom-impostor / podium bug.
#
# Scene AvatarShapes (e.g. the Tower of Madness podium winners) are hidden by
# their parent ENTITY via a VisibilityComponent: the entity node gets
# visible=false (and scale ~0) while the Avatar node itself keeps visible=true.
# The LOD code used to gate the impostor on the Avatar node's own `visible`
# flag, which stayed true, so the avatar entered the impostor system and the
# AvatarScene MultiMesh drew a full-size impostor at the avatar's position even
# though the avatar was invisible in the tree — three identical phantom
# silhouettes on the podium.
#
# The fix gates on is_visible_in_tree() instead, which folds in every ancestor's
# visibility. This test pins the invariant the fix relies on: an avatar whose
# parent entity is hidden reports is_visible_in_tree() == false (so the guard
# skips it), and is visible again only when the parent is shown.
#
# NOTE: Node3D visibility propagates DEFERRED — is_visible_in_tree() only
# reflects a parent's visible change on the next processed frame, so every case
# awaits process_frame before asserting. This matches production: _update_lod
# runs in _process, long after the scene hid the podium entity.
#
# Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     --script res://src/test/avatar/test_avatar_impostor_hidden_parent.gd

var _failures: Array[String] = []


# gdlint:ignore = async-function-name
func _initialize() -> void:
	await _test_parent_hidden_blocks()
	await _test_parent_shown_allows()
	await _test_self_hidden_blocks()
	_finish()


# Mirrors the podium layout: entity node (carries the VisibilityComponent) with
# the Avatar node as its child. Returns [entity, avatar].
func _make_entity_with_avatar() -> Array:
	var entity := Node3D.new()
	var avatar := Node3D.new()
	entity.add_child(avatar)
	root.add_child(entity)
	return [entity, avatar]


# gdlint:ignore = async-function-name
func _test_parent_hidden_blocks() -> void:
	var pair := _make_entity_with_avatar()
	var entity: Node3D = pair[0]
	var avatar: Node3D = pair[1]
	entity.visible = false  # VisibilityComponent hid the entity
	await process_frame
	# Avatar keeps its own flag true, exactly like the live podium avatars.
	_expect("avatar.visible own flag stays true", true, avatar.visible)
	_expect("parent hidden -> not visible in tree", false, avatar.is_visible_in_tree())
	entity.free()


# gdlint:ignore = async-function-name
func _test_parent_shown_allows() -> void:
	var pair := _make_entity_with_avatar()
	var entity: Node3D = pair[0]
	var avatar: Node3D = pair[1]
	entity.visible = true
	await process_frame
	_expect("parent shown -> visible in tree", true, avatar.is_visible_in_tree())
	entity.free()


# gdlint:ignore = async-function-name
func _test_self_hidden_blocks() -> void:
	var pair := _make_entity_with_avatar()
	var entity: Node3D = pair[0]
	var avatar: Node3D = pair[1]
	entity.visible = true
	avatar.visible = false  # e.g. set_hidden / modifier area
	await process_frame
	_expect("self hidden -> not visible in tree", false, avatar.is_visible_in_tree())
	entity.free()


func _expect(ctx: String, expected: bool, actual: bool) -> void:
	if expected != actual:
		_failures.append("%s: expected %s, got %s" % [ctx, expected, actual])


func _finish() -> void:
	if _failures.is_empty():
		print("[test_avatar_impostor_hidden_parent] PASS")
		quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_avatar_impostor_hidden_parent] FAIL: %d case(s)" % _failures.size())
	quit(1)
