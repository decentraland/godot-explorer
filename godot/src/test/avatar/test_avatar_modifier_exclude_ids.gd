extends SceneTree

# Regression test for issue #2166.
#
# AvatarModifierArea.excludeIds must keep remote avatars visible even when the
# address casing differs (checksummed vs lowercase) and when no `0x` prefix is
# used. Comms / wire profile sometimes deliver mixed-case addresses, while
# scenes commonly push lowercase IDs in excludeIds.
#
# Run with the project's godot binary, e.g.:
#   .bin/godot/godot4_bin --headless --path godot \
#     --script res://src/test/avatar/test_avatar_modifier_exclude_ids.gd

const MATCHER := preload("res://src/decentraland_components/avatar/avatar_exclude_id_matcher.gd")

const ADDR_LOWER := "0xabcdef0123456789abcdef0123456789abcdef01"
const ADDR_CHECKSUM := "0xAbCdEf0123456789AbCdEf0123456789aBcDeF01"
const ADDR_NO_PREFIX := "abcdef0123456789abcdef0123456789abcdef01"
const OTHER_ADDR := "0x1111111111111111111111111111111111111111"


func _init() -> void:
	var failures: Array[String] = []

	# Sanity: identical casing matches.
	_expect("exact lowercase match", failures, ADDR_LOWER, [ADDR_LOWER], true)

	# Sanity: no overlap.
	_expect("no overlap", failures, ADDR_LOWER, [OTHER_ADDR], false)

	# Bug #2166: scene sent lowercase, profile delivered checksummed.
	_expect(
		"checksum avatar_id vs lowercase exclude (bug #2166)",
		failures,
		ADDR_CHECKSUM,
		[ADDR_LOWER],
		true
	)

	# Bug #2166 inverse: profile lowercase, scene listed checksummed.
	_expect(
		"lowercase avatar_id vs checksum exclude (bug #2166)",
		failures,
		ADDR_LOWER,
		[ADDR_CHECKSUM],
		true
	)

	# Bug #2166: missing 0x prefix on the scene-side id.
	_expect(
		"lowercase avatar_id vs no-prefix exclude (bug #2166)",
		failures,
		ADDR_LOWER,
		[ADDR_NO_PREFIX],
		true
	)

	# Race-condition guard: avatar_id not yet populated when area triggers.
	# Empty id must never silently match a non-empty exclude entry.
	_expect("empty avatar_id never matches", failures, "", [ADDR_LOWER], false)

	# Multi-entry list: one of several entries matches with mixed casing.
	_expect(
		"mixed-casing entry among many is found",
		failures,
		ADDR_LOWER,
		[OTHER_ADDR, ADDR_CHECKSUM, "0x0000000000000000000000000000000000000000"],
		true
	)

	if failures.is_empty():
		print("[test_avatar_modifier_exclude_ids] PASS (all 7 cases)")
		quit(0)
		return

	for f in failures:
		printerr(f)
	printerr("[test_avatar_modifier_exclude_ids] FAIL: %d case(s) failing" % failures.size())
	quit(1)


func _expect(
	case_name: String,
	failures: Array[String],
	avatar_id: String,
	exclude_ids: Array,
	expected: bool
) -> void:
	var actual: bool = MATCHER.is_excluded(avatar_id, exclude_ids)
	if actual != expected:
		failures.append(
			(
				"[%s] expected is_excluded=%s, got %s (avatar_id=%s, exclude_ids=%s)"
				% [case_name, expected, actual, avatar_id, str(exclude_ids)]
			)
		)
