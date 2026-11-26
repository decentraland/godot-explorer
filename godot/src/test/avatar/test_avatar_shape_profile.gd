extends SceneTree

# Regression test for the scene AvatarShape id (#3338, PR #3351).
#
# Since every AvatarShape.id reaches avatar_id, a plain scene id like "npc-1" is
# not a profile: tapping that NPC must not try to open a passport. An NPC whose
# id is a real address still has one. And when the scene clears the id, the
# avatar must drop it too, or AvatarModifierArea.excludeIds keeps matching it.
#
# Run headless:
#   Godot --headless --path godot \
#     --script res://src/test/avatar/test_avatar_shape_profile.gd

# Loaded at run time, untyped: avatar.gd needs the Global autoload, which does not
# exist yet while this script compiles.
const AVATAR_SCENE := "res://src/decentraland_components/avatar/avatar.tscn"
const MATCHER := preload("res://src/decentraland_components/avatar/avatar_exclude_id_matcher.gd")
const ADDRESS := "0xabcdef0123456789abcdef0123456789abcdef01"


func _initialize() -> void:
	# After the first frame: the autoloads are in the tree by then.
	process_frame.connect(_run, CONNECT_ONE_SHOT)


func _run() -> void:
	var failures: Array[String] = []
	# Left out of the tree, so avatar.gd's _ready() (and its loading) never runs:
	# wire the two @onready nodes the code under test touches by hand.
	var avatar = load(AVATAR_SCENE).instantiate()
	avatar.click_area = avatar.get_node("ClickArea")
	avatar.avatar_modifier_area_detector = avatar.get_node("avatar_modifier_area_detector")

	# Who has a passport.
	_expect_profile(failures, avatar, false, ADDRESS, true, "player")
	_expect_profile(failures, avatar, false, "", false, "player without an id yet")
	_expect_profile(failures, avatar, true, "npc-1", false, "NPC with a plain scene id")
	_expect_profile(failures, avatar, true, ADDRESS, true, "NPC wearing a real address")
	_expect_profile(failures, avatar, true, "", false, "NPC without an id")

	# The scene sets an id, then clears it.
	avatar.avatar_id = ""
	avatar.is_avatar_shape = true
	avatar._apply_shape_id("npc-1")
	if avatar.avatar_id != "npc-1":
		failures.append("setting id 'npc-1' left avatar_id = '%s'" % avatar.avatar_id)
	avatar._apply_shape_id("")
	if avatar.avatar_id != "":
		failures.append("clearing the id left avatar_id = '%s'" % avatar.avatar_id)
	if MATCHER.is_excluded(avatar.avatar_id, ["npc-1"]):
		failures.append("after clearing the id, excludeIds ['npc-1'] still matches")
	if avatar.click_area.get_meta("avatar_id", "") != "":
		failures.append(
			"clearing the id left click-area meta '%s'" % avatar.click_area.get_meta("avatar_id")
		)

	avatar.free()

	if failures.is_empty():
		print("[test_avatar_shape_profile] PASS")
		quit(0)
		return
	for f in failures:
		printerr(f)
	printerr("[test_avatar_shape_profile] FAIL: %d case(s) failing" % failures.size())
	quit(1)


func _expect_profile(
	failures: Array[String],
	avatar: Node,
	is_shape: bool,
	id: String,
	expected: bool,
	case_name: String
) -> void:
	avatar.is_avatar_shape = is_shape
	avatar.avatar_id = id
	if avatar.has_profile() != expected:
		failures.append("[%s] has_profile() should be %s" % [case_name, expected])
