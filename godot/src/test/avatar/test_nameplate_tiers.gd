extends Node

# Tests for NameplateLayer distance tiers + crowd cap (#2938).
# Pins: tier band boundaries (8/12m), focused override, crowd cap degrading the
# farthest close plates to the badge tier (distant plates don't consume slots,
# ties break by instance id, incumbents keep one slot of stickiness), and the
# per-tier alpha targets.
#
# Runs as a scene, not --script: NameplateLayer reaches Global, and --script
# compiles before the autoloads exist.

## Guards against a silent early return: every check below must have run.
const EXPECTED_CHECKS := 19

var _failures: Array[String] = []
var _checks: int = 0


func _ready() -> void:
	_test_tier_bands()
	_test_focused_override()
	_test_crowd_cap()
	_test_crowd_tiebreak_and_stickiness()
	_test_tier_alpha()
	_finish()


func _test_tier_bands() -> void:
	var ui := Control.new()
	_expect(
		"dist 5 -> FULL", NameplateLayer.Tier.FULL, NameplateLayer._content_tier(ui, 5.0, false)
	)
	_expect(
		"dist 8 boundary -> FULL",
		NameplateLayer.Tier.FULL,
		NameplateLayer._content_tier(ui, 8.0, false)
	)
	_expect(
		"dist 10 -> BADGE", NameplateLayer.Tier.BADGE, NameplateLayer._content_tier(ui, 10.0, false)
	)
	_expect(
		"dist 12 boundary -> BADGE",
		NameplateLayer.Tier.BADGE,
		NameplateLayer._content_tier(ui, 12.0, false)
	)
	_expect(
		"dist 15 -> NAME", NameplateLayer.Tier.NAME, NameplateLayer._content_tier(ui, 15.0, false)
	)
	_expect(
		"dist 22 -> NAME", NameplateLayer.Tier.NAME, NameplateLayer._content_tier(ui, 22.0, false)
	)
	ui.free()


func _test_focused_override() -> void:
	var ui := Control.new()
	_expect(
		"focused at 15m -> FULL",
		NameplateLayer.Tier.FULL,
		NameplateLayer._content_tier(ui, 15.0, true)
	)
	_expect(
		"focused at 24m -> FULL",
		NameplateLayer.Tier.FULL,
		NameplateLayer._content_tier(ui, 24.0, true)
	)
	ui.free()


func _test_crowd_cap() -> void:
	# Fill the cap with CROWD_MAX_FULL nearer plates: the farthest one degrades.
	var nearer: Array[Control] = []
	for i in NameplateLayer.CROWD_MAX_FULL:
		var c := Control.new()
		nearer.append(c)
		NameplateLayer._plate_dists[c.get_instance_id()] = {"dist": 1.0, "focused": false}
	var far := Control.new()
	_expect(
		"cap full -> farthest degrades to BADGE",
		NameplateLayer.Tier.BADGE,
		NameplateLayer._content_tier(far, 5.0, false)
	)
	# A focused plate in the crowd takes a cap slot from the unfocused ones.
	var focused := Control.new()
	NameplateLayer._plate_dists[focused.get_instance_id()] = {"dist": 1.0, "focused": true}
	var another := Control.new()
	_expect(
		"focused plate consumes a cap slot",
		NameplateLayer.Tier.BADGE,
		NameplateLayer._content_tier(another, 5.0, false)
	)
	# Plates beyond the full band don't consume cap slots — fill with 7 of them;
	# if they counted, the cap would be full and `far` would degrade.
	NameplateLayer._plate_dists.clear()
	var distant: Array[Control] = []
	for i in NameplateLayer.CROWD_MAX_FULL:
		var c := Control.new()
		distant.append(c)
		NameplateLayer._plate_dists[c.get_instance_id()] = {"dist": 15.0, "focused": false}
	_expect(
		"distant plates don't consume slots",
		NameplateLayer.Tier.FULL,
		NameplateLayer._content_tier(far, 5.0, false)
	)
	NameplateLayer._plate_dists.clear()
	for c in nearer + [far, focused, another] + distant:
		c.free()


func _test_crowd_tiebreak_and_stickiness() -> void:
	# Equal distances break by instance id. `first` is created before the 7
	# plates that fill the cap, so it wins every tie; `last` loses them all.
	NameplateLayer._plate_dists.clear()
	var first := Control.new()
	var others: Array[Control] = []
	for i in NameplateLayer.CROWD_MAX_FULL:
		var c := Control.new()
		others.append(c)
		NameplateLayer._plate_dists[c.get_instance_id()] = {"dist": 5.0, "focused": false}
	var last := Control.new()
	_expect(
		"tie as oldest plate -> stays FULL",
		NameplateLayer.Tier.FULL,
		NameplateLayer._content_tier(first, 5.0, false)
	)
	_expect(
		"tie against older plates -> degrades",
		NameplateLayer.Tier.BADGE,
		NameplateLayer._content_tier(last, 5.0, false)
	)
	# Stickiness: a plate already rendered FULL keeps its slot at one over cap.
	NameplateLayer._plate_tiers[last.get_instance_id()] = [NameplateLayer.Tier.FULL, true]
	_expect(
		"incumbent FULL keeps slot at cap+0",
		NameplateLayer.Tier.FULL,
		NameplateLayer._content_tier(last, 5.0, false)
	)
	NameplateLayer._plate_tiers.erase(last.get_instance_id())
	NameplateLayer._plate_dists.clear()
	for c in others + [first, last]:
		c.free()


func _test_tier_alpha() -> void:
	_expect(
		"FULL at 5m -> 1.0", 1.0, NameplateLayer._tier_alpha(NameplateLayer.Tier.FULL, 5.0, false)
	)
	_expect(
		"NAME at 15m -> dimmed",
		NameplateLayer.TIER_NAME_ALPHA,
		NameplateLayer._tier_alpha(NameplateLayer.Tier.NAME, 15.0, false)
	)
	_expect(
		"fade midpoint 22.5m",
		0.5,
		NameplateLayer._tier_alpha(NameplateLayer.Tier.FULL, 22.5, false)
	)
	_expect(
		"NAME fade keeps dim ceiling",
		minf(NameplateLayer.TIER_NAME_ALPHA, 0.5),
		NameplateLayer._tier_alpha(NameplateLayer.Tier.NAME, 22.5, false)
	)
	_expect(
		"focused at 24m -> 1.0",
		1.0,
		NameplateLayer._tier_alpha(NameplateLayer.Tier.FULL, 24.0, true)
	)


func _expect(ctx: String, expected: Variant, actual: Variant) -> void:
	_checks += 1
	if expected != actual:
		_failures.append("%s: expected %s, got %s" % [ctx, expected, actual])


func _finish() -> void:
	if _checks != EXPECTED_CHECKS:
		_failures.append("ran %d checks, expected %d" % [_checks, EXPECTED_CHECKS])
	if _failures.is_empty():
		print("[test_nameplate_tiers] PASS")
		get_tree().quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_nameplate_tiers] FAIL: %d case(s)" % _failures.size())
	get_tree().quit(1)
