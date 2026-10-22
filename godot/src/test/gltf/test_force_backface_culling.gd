extends RefCounted

# `alfa-force-backface-culling` (issue #3004): with the flag on, every scene glTF material
# must land on CULL_BACK. Assets authored as an inverted-hull outline ship the shell
# `doubleSided`, which Godot honours as CULL_DISABLED — the shell then covers the model
# instead of leaving a rim. Off is the shipped default and must leave the authored cull
# mode alone, since forcing it also drops the back faces of double-sided content.

const GltfContainerScript = preload("res://src/decentraland_components/gltf_container.gd")

var suite_name := "gltf_container_material"
var method_name := "test_force_backface_culling_flag"
var errors: Array[String] = []
var execution_time_seconds := 0.0


func run() -> bool:
	var start := Time.get_ticks_usec()
	var ok := true

	# Detached stubs: _ready never runs, so there is no bff fetch to race against.
	var previous_flags := Global.feature_flags
	var flags := FeatureFlags.new()
	Global.feature_flags = flags
	var container = GltfContainerScript.new()

	# Absent flag and explicit false are the same thing: today's rendering.
	for payload in [{}, {FeatureFlags.FLAG_FORCE_BACKFACE_CULLING: false}]:
		flags._flags = payload
		var authored := _double_sided_material()
		container.fix_material(authored)
		ok = (
			_expect_cull(
				authored,
				BaseMaterial3D.CULL_DISABLED,
				"flag off keeps CULL_DISABLED (%s)" % payload
			)
			and ok
		)

	# Flag on: the shell is force-culled.
	flags._flags = {FeatureFlags.FLAG_FORCE_BACKFACE_CULLING: true}
	var shell := _double_sided_material()
	container.fix_material(shell)
	ok = _expect_cull(shell, BaseMaterial3D.CULL_BACK, "flag on forces CULL_BACK") and ok

	# The rules this function already owned must still run.
	ok = _expect(shell.has_meta("dcl_fixed"), true, "material still marked dcl_fixed") and ok
	ok = _expect(shell.vertex_color_use_as_albedo, false, "vertex color still off") and ok

	# QA opt-in: the deeplink wins over a payload that does not serve the flag.
	var previous_link: DclParseDeepLink = Global.deep_link_obj
	Global.deep_link_obj = DclParseDeepLink.parse_decentraland_link(
		"decentraland://open?force-backface-culling=true"
	)
	flags._flags = {}
	var forced := _double_sided_material()
	container.fix_material(forced)
	ok = _expect_cull(forced, BaseMaterial3D.CULL_BACK, "deeplink forces CULL_BACK") and ok
	Global.deep_link_obj = previous_link

	container.free()
	Global.feature_flags = previous_flags
	flags.free()

	execution_time_seconds = (Time.get_ticks_usec() - start) / 1_000_000.0
	return ok


func _double_sided_material() -> BaseMaterial3D:
	# StandardMaterial3D, not BaseMaterial3D: the latter is abstract (and it is what the
	# glTF importer hands to fix_material).
	var mat := StandardMaterial3D.new()
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	return mat


func _expect_cull(mat: BaseMaterial3D, expected: int, label: String) -> bool:
	if mat.cull_mode != expected:
		errors.append("%s: expected cull %d, got %d" % [label, expected, mat.cull_mode])
		return false
	return true


func _expect(actual: bool, expected: bool, label: String) -> bool:
	if actual != expected:
		errors.append("%s: expected %s, got %s" % [label, expected, actual])
		return false
	return true
