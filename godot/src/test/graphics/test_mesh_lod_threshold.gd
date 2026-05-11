extends Node

## Per-profile mesh_lod_threshold fixtures.
##
## Default Godot viewport.mesh_lod_threshold is 1.0 (very conservative). These
## tests pin the per-profile values so a future tweak to the LOD chain or to
## profile balancing surfaces in CI rather than as a silent regression on
## low-end hardware (where higher thresholds buy real fps by skipping
## sub-pixel mesh detail).

const _TEST_METHODS: Array[String] = [
	"test_profile_definitions_include_mesh_lod_threshold",
	"test_apply_very_low_profile_sets_viewport_threshold_to_8",
	"test_apply_low_profile_sets_viewport_threshold_to_6",
	"test_apply_medium_profile_sets_viewport_threshold_to_3",
	"test_apply_high_profile_sets_viewport_threshold_to_2",
]

var suite_name: String = "test_mesh_lod_threshold"
var method_name: String = ""
var errors: Array[String] = []
var execution_time_seconds: float = 0.0


func run() -> bool:
	errors.clear()
	var start_usec: int = Time.get_ticks_usec()
	if method_name.is_empty():
		for m in _TEST_METHODS:
			call(m)
	else:
		call(method_name)
	execution_time_seconds = float(Time.get_ticks_usec() - start_usec) / 1_000_000.0
	return errors.is_empty()


func _viewport() -> Viewport:
	return Global.get_tree().root.get_viewport()


func _fail(msg: String) -> void:
	errors.append("[%s] %s" % [method_name, msg])


func test_profile_definitions_include_mesh_lod_threshold() -> void:
	method_name = "test_profile_definitions_include_mesh_lod_threshold"
	for i in range(GraphicSettings.PROFILE_DEFINITIONS.size()):
		var entry: Dictionary = GraphicSettings.PROFILE_DEFINITIONS[i]
		if not entry.has("mesh_lod_threshold"):
			_fail("PROFILE_DEFINITIONS[%d] missing mesh_lod_threshold key" % i)


func test_apply_very_low_profile_sets_viewport_threshold_to_8() -> void:
	method_name = "test_apply_very_low_profile_sets_viewport_threshold_to_8"
	GraphicSettings.apply_graphic_profile(0)
	var got: float = _viewport().mesh_lod_threshold
	if not is_equal_approx(got, 8.0):
		_fail("expected viewport.mesh_lod_threshold == 8.0, got %f" % got)


func test_apply_low_profile_sets_viewport_threshold_to_6() -> void:
	method_name = "test_apply_low_profile_sets_viewport_threshold_to_6"
	GraphicSettings.apply_graphic_profile(1)
	var got: float = _viewport().mesh_lod_threshold
	if not is_equal_approx(got, 6.0):
		_fail("expected viewport.mesh_lod_threshold == 6.0, got %f" % got)


func test_apply_medium_profile_sets_viewport_threshold_to_3() -> void:
	method_name = "test_apply_medium_profile_sets_viewport_threshold_to_3"
	GraphicSettings.apply_graphic_profile(2)
	var got: float = _viewport().mesh_lod_threshold
	if not is_equal_approx(got, 3.0):
		_fail("expected viewport.mesh_lod_threshold == 3.0, got %f" % got)


func test_apply_high_profile_sets_viewport_threshold_to_2() -> void:
	method_name = "test_apply_high_profile_sets_viewport_threshold_to_2"
	GraphicSettings.apply_graphic_profile(3)
	var got: float = _viewport().mesh_lod_threshold
	if not is_equal_approx(got, 2.0):
		_fail("expected viewport.mesh_lod_threshold == 2.0, got %f" % got)
