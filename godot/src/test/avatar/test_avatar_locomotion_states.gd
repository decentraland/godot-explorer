extends SceneTree

# Behavioural regression for avatar.tscn's locomotion state machine (#2603,
# #2699). Instead of asserting the shape of the graph, this drives the real tree
# with the condition vectors avatar.gd publishes and reads the live state back:
# a state the world can never pull out of is the bug those issues reported — the
# avatar frozen mid-air, or standing while gliding.
#
# Each case seeds one state, publishes one world, and asserts the state the
# machine moves to. Reachability across several states is covered by composing
# cases, so no case has to wait for a chain.
#
# The tree is advanced by hand from _initialize: headless idles as fast as the
# machine allows, so waiting for frames advances no animation time at all, and
# every case gets its own tree so one case cannot leak state into the next. The
# two nodes do have to be parented: the mixer resolves its animations through
# the player's path, and without it every transition stops firing.
#
# Run headless:
#   Godot --headless --path godot \
#     --script res://src/test/avatar/test_avatar_locomotion_states.gd

const AVATAR_SCENE := "res://src/decentraland_components/avatar/avatar.tscn"
const MACHINE := "Locomotion"
const TICK := 1.0 / 60.0
# Two seconds of animation time: past a cross-fade and past the end of the
# longest clip the machine can be waiting on (the at-end edges).
const STEP_TICKS := 120
# Let the tree run its own first update before playback.start(), or that update
# lands on top of it and the machine restarts from Start.
const SETTLE_TICKS := 2

# The tree parameters avatar.gd publishes (its `conditions/run` block).
const CONDITION_NAMES: Array[String] = [
	"emote",
	"nemote",
	"emix",
	"nemix",
	"rise_walk",
	"rise_run",
	"rise_bounce",
	"rise",
	"fall",
	"land",
	"fall_fast",
	"apex",
	"nfall",
	"long_fall",
	"fall_short",
	"stunned",
	"nstunned",
	"land_soft",
	"double_jump",
	"gliding",
	"ngliding",
]

# [seed state, world primitives (see _vector), states the world may settle in].
# A set rather than one state: the machine may take one transition or two before
# it settles, and what has to hold is that the world pulls it into the right
# family instead of leaving it in the state it was already playing.
const CASES := [
	# Gliding has to win from every airborne state: Unity starts it through an
	# Any State transition on StartGliding, and a state without the edge keeps
	# the avatar frozen in the pre-glide pose (#2699).
	["Grounded", {"gliding": true, "fall": true}, ["Gliding_Start", "Gliding_Idle"]],
	["Jump_Start", {"gliding": true, "fall": true}, ["Gliding_Start", "Gliding_Idle", "Jump_Rise"]],
	[
		"Run_Jump_Start",
		{"gliding": true, "fall": true},
		["Gliding_Start", "Gliding_Idle", "Run_Jump_Rise"]
	],
	["Jump_Rise", {"gliding": true, "fall": true}, ["Gliding_Start", "Gliding_Idle"]],
	["Run_Jump_Rise", {"gliding": true, "fall": true}, ["Gliding_Start", "Gliding_Idle"]],
	[
		"Double_Jump_Rise",
		{"gliding": true, "fall": true},
		["Gliding_Start", "Gliding_Idle", "Jump_Fall"]
	],
	["Jump_Mid", {"gliding": true, "fall": true}, ["Gliding_Start", "Gliding_Idle"]],
	["Run_Jump_Mid", {"gliding": true, "fall": true}, ["Gliding_Start", "Gliding_Idle"]],
	["Jump_Fall", {"gliding": true, "fall": true}, ["Gliding_Start", "Gliding_Idle"]],
	["Run_Jump_Fall", {"gliding": true, "fall": true}, ["Gliding_Start", "Gliding_Idle"]],
	["Long_Fall", {"gliding": true, "fall": true}, ["Gliding_Start", "Gliding_Idle"]],
	["Jump_End", {"gliding": true, "fall": true}, ["Gliding_Start", "Gliding_Idle"]],
	["Run_Jump_End", {"gliding": true, "fall": true}, ["Gliding_Start", "Gliding_Idle"]],
	[
		"Hard_Landing",
		{"gliding": true, "fall": true, "stunned": true},
		["Gliding_Start", "Gliding_Idle"]
	],
	[
		"Gliding_End",
		{"gliding": true, "fall": true},
		["Gliding_Start", "Gliding_Idle", "Jump_Fall"]
	],
	# Falling: no state may hold a grounded pose while the avatar is going down.
	# Grounded tests `fall`, not `fall_fast`, so a ledge step-off changes pose on
	# the first falling frame instead of waiting for vy < -3.
	["Grounded", {"fall": true}, ["Jump_Fall"]],
	["Grounded", {"walk": true, "fall": true}, ["Jump_Fall"]],
	["Grounded", {"fall": true, "fall_fast": true}, ["Jump_Fall"]],
	["Jump_Mid", {"fall": true, "fall_fast": true}, ["Jump_Fall"]],
	["Run_Jump_Mid", {"fall": true, "fall_fast": true}, ["Run_Jump_Fall"]],
	["Long_Fall", {"fall": true, "fall_fast": true}, ["Jump_Fall"]],
	[
		"Gliding_Idle",
		{"fall": true, "fall_fast": true},
		["Gliding_End", "Jump_Fall", "Run_Jump_Fall", "Long_Fall"]
	],
	# Rising, and the jump chain out of the grounded blend.
	["Grounded", {"rise": true, "gait": 3}, ["Run_Jump_Start", "Run_Jump_Rise"]],
	["Grounded", {"rise": true}, ["Jump_Start", "Jump_Rise"]],
	["Jump_Rise", {}, ["Jump_Mid"]],
	["Run_Jump_Rise", {}, ["Run_Jump_Mid"]],
	["Jump_Rise", {"double_jump": true}, ["Double_Jump_Rise"]],
	["Jump_End", {"rise": true}, ["Jump_Rise", "Jump_Start"]],
	["Long_Fall", {"rise": true}, ["Jump_Rise", "Jump_Start"]],
	# An air jump rises out of the long fall whatever the gait is; the edge used
	# to need a bounce pad (jump_count == 0).
	["Long_Fall", {"rise": true, "gait": 3, "jump_count": 1}, ["Jump_Rise", "Jump_Start"]],
	# The double jump opens the flipper from the long fall, as it does from
	# Jump_Fall — both edges match, and the double jump wins the tie.
	[
		"Long_Fall",
		{"double_jump": true, "rise": true, "gait": 3, "jump_count": 2},
		["Double_Jump_Rise"]
	],
	# Landings, including the long fall that stuns.
	["Jump_Fall", {"grounded": true}, ["Jump_End", "Grounded"]],
	["Run_Jump_Fall", {"grounded": true}, ["Run_Jump_End", "Grounded"]],
	["Long_Fall", {"grounded": true, "stunned": true}, ["Hard_Landing"]],
	["Long_Fall", {"grounded": true}, ["Jump_End", "Grounded"]],
	["Jump_End", {"grounded": true}, ["Grounded"]],
	["Run_Jump_End", {"grounded": true}, ["Grounded"]],
	["Hard_Landing", {"grounded": true}, ["Grounded"]],
	# Gliding out and back in.
	["Gliding_Idle", {}, ["Gliding_End", "Grounded"]],
	["Gliding_End", {"fall": true}, ["Jump_Fall"]],
	# Emotes only from a standstill, and back out of them.
	["Grounded", {"grounded": true, "emote": true}, ["Emote"]],
	["Grounded", {"grounded": true, "emix": true}, ["Emote_Mix"]],
	["Emote", {"grounded": true}, ["Grounded"]],
	["Emote_Mix", {"grounded": true}, ["Grounded"]],
	# A grounded avatar with nothing else going on stays grounded.
	["Grounded", {"grounded": true}, ["Grounded"]],
]

var _failures: Array[String] = []


func _initialize() -> void:
	for case in CASES:
		_run_case(case[0], case[1], case[2])
	if _failures.is_empty():
		print("[test_avatar_locomotion_states] PASS (%d cases)" % CASES.size())
		quit(0)
		return
	for failure in _failures:
		printerr(failure)
	printerr("[test_avatar_locomotion_states] FAIL: %d case(s)" % _failures.size())
	quit(1)


func _run_case(seed: String, primitives: Dictionary, expected: Array) -> void:
	var inst: Node = load(AVATAR_SCENE).instantiate()
	var player: AnimationPlayer = inst.get_node("AnimationPlayer")
	var tree: AnimationTree = inst.get_node("AnimationTree")
	# Only the animation half of the avatar is needed, and leaving the scene root
	# unparented keeps avatar.gd's _ready() out of the test.
	inst.remove_child(player)
	inst.remove_child(tree)
	inst.free()
	root.add_child(player)
	root.add_child(tree)
	tree.active = true
	var machine: AnimationNodeStateMachine = (tree.tree_root as AnimationNodeBlendTree).get_node(
		MACHINE
	)
	if not machine.has_node(seed):
		_fail("seed '%s' is not a state of the machine" % seed)
		return

	for name in CONDITION_NAMES:
		tree.set("parameters/%s/conditions/%s" % [MACHINE, name], false)
	for _tick in SETTLE_TICKS:
		tree.advance(TICK)

	var vector := _vector(primitives)
	for name in CONDITION_NAMES:
		tree.set("parameters/%s/conditions/%s" % [MACHINE, name], vector[name])
	var playback: AnimationNodeStateMachinePlayback = tree.get("parameters/%s/playback" % MACHINE)
	playback.start(seed)
	for _tick in STEP_TICKS:
		tree.advance(TICK)

	var current := String(playback.get_current_node())
	if not expected.has(current):
		_fail(
			(
				"seed '%s' with world %s: settled in '%s', expected one of %s"
				% [seed, str(primitives), current, str(expected)]
			)
		)


# The condition vector avatar.gd publishes for a set of world primitives (its
# `conditions/run` block). Everything not named is derived here the way the
# runtime derives it, so every vector is a world the runtime can publish.
func _vector(primitives: Dictionary) -> Dictionary:
	var grounded: bool = primitives.get("grounded", false)
	var rise: bool = primitives.get("rise", false)
	var gait: int = primitives.get("gait", 0)
	var stunned: bool = primitives.get("stunned", false)
	var gliding: bool = primitives.get("gliding", false)
	var emote: bool = primitives.get("emote", false)
	var emix: bool = primitives.get("emix", false)
	var fall_fast: bool = primitives.get("fall_fast", false)
	var long_fall: bool = primitives.get("long_fall", false)
	# Bounce pads launch without a jump; an air jump increments it.
	var jump_count: int = primitives.get("jump_count", 0)

	var vector := {}
	vector["nfall"] = grounded
	vector["rise"] = rise
	vector["rise_walk"] = rise and gait < 3
	vector["rise_run"] = rise and gait == 3
	vector["rise_bounce"] = rise and jump_count == 0
	vector["fall"] = primitives.get("fall", false)
	vector["fall_fast"] = fall_fast
	vector["fall_short"] = fall_fast and not long_fall
	vector["long_fall"] = long_fall
	vector["apex"] = not grounded and not rise and not fall_fast
	vector["land"] = primitives.get("land", false)
	vector["land_soft"] = grounded and not stunned
	vector["stunned"] = stunned
	vector["gliding"] = gliding
	vector["double_jump"] = primitives.get("double_jump", false)
	vector["emote"] = emote
	vector["emix"] = emix
	vector["nemote"] = not emote
	vector["nemix"] = not emix
	vector["nstunned"] = not stunned
	vector["ngliding"] = not gliding
	return vector


func _fail(message: String) -> void:
	_failures.append(message)
