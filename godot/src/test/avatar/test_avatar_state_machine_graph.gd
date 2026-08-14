extends SceneTree

# Regression test for the avatar.tscn state-machine graph itself (#2603).
#
# Caveat: unconditional (at-end) edges are treated as always traversable, but
# an at-end edge out of a LOOPING animation never fires at runtime. This check
# is necessary but not sufficient — a state whose only escape is at-end on a
# looping anim would pass here while still trapping in-game.
#
# Several bugs shared one shape: the AnimationTree lands in a state that has
# no outgoing transition for the current world condition, so the avatar stays
# frozen in a stale pose — walking mid-air, standing while gliding, etc.
# Instead of pinning one transition at a time, this test parses the real
# transition graph out of avatar.tscn and asserts every state can escape:
#
#   - on `fall`:    every state must reach a fall state, except the gliding
#                   family (they exit through `ngliding` -> Gliding_End first)
#   - on `gliding`: every state must reach Gliding_Start, except Gliding_Idle
#                   (already gliding; Gliding_Start re-entry covers the rest)
#
# Run headless:
#   Godot --headless --path godot \
#     --script res://src/test/avatar/test_avatar_state_machine_graph.gd

const TSCN := "res://src/decentraland_components/avatar/avatar.tscn"
const GLIDE_FAMILY := ["Gliding_Start", "Gliding_Idle", "Gliding_End"]

var _failures: Array[String] = []


func _initialize() -> void:
	var edges := _parse_edges()
	if edges.is_empty():
		_fail("no transitions parsed from avatar.tscn — parser drifted from file format?")
	else:
		_test_escapes(edges, "fall", ["Gliding_Start", "Gliding_Idle"])
		_test_escapes(edges, "gliding", ["Gliding_Idle"])
	_finish()


# Returns Array of [from, to, condition_or_empty]. Only the main state machine
# (the one whose transition list mentions Gliding_Start) is considered.
func _parse_edges() -> Array:
	var f := FileAccess.open(TSCN, FileAccess.READ)
	if f == null:
		_fail("cannot open " + TSCN)
		return []
	var txt := f.get_as_text()

	# sub_resource id -> advance_condition ("")
	var conds := {}
	var re_block := RegEx.new()
	re_block.compile(
		'\\[sub_resource type="AnimationNodeStateMachineTransition" id="([^"]+)"\\]([^\\[]*)'
	)
	var re_cond := RegEx.new()
	re_cond.compile('advance_condition = &"(\\w+)"')
	for m in re_block.search_all(txt):
		var cond := ""
		var cm := re_cond.search(m.get_string(2))
		if cm:
			cond = cm.get_string(1)
		conds[m.get_string(1)] = cond

	# main machine transition list
	var re_list := RegEx.new()
	re_list.compile('transitions = \\[("Start".*?)\\]')
	var lm := re_list.search(txt)
	if lm == null:
		return []
	var re_tok := RegEx.new()
	re_tok.compile('"([^"]+)"|SubResource\\("([^"]+)"\\)')
	var edges := []
	var raw: Array = []
	for tm in re_tok.search_all(lm.get_string(1)):
		raw.append(tm.get_string(1) if tm.get_string(1) != "" else tm.get_string(2))
	for i in range(0, raw.size() - 2, 3):
		edges.append([raw[i], raw[i + 1], conds.get(raw[i + 2], "")])
	return edges


func _test_escapes(edges: Array, cond: String, exempt: Array) -> void:
	var states := {}
	for e in edges:
		states[e[0]] = true
		states[e[1]] = true
	states.erase("Start")
	states.erase("End")

	# States reachable by following only unconditional/at-end or `cond` edges,
	# starting from states entered via a `cond` edge.
	var ok := {}
	for e in edges:
		if e[2] == cond:
			ok[e[1]] = true
	var changed := true
	while changed:
		changed = false
		for e in edges:
			if ok.has(e[1]) and not ok.has(e[0]) and (e[2] == "" or e[2] == cond):
				ok[e[0]] = true
				changed = true

	for s in states:
		if s in exempt:
			continue
		if not ok.has(s):
			_fail("state '%s' has no escape path on condition '%s'" % [s, cond])


func _fail(msg: String) -> void:
	_failures.append(msg)


func _finish() -> void:
	if _failures.is_empty():
		print("[test_avatar_state_machine_graph] PASS")
		quit(0)
		return
	for fail in _failures:
		printerr(fail)
	printerr("[test_avatar_state_machine_graph] FAIL: %d case(s)" % _failures.size())
	quit(1)
