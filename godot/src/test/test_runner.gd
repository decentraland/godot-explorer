extends Node

const GDSCRIPT_TEST_FILES: Array[String] = [
	"res://src/test/graphics/test_shadow_config.gd",
]


func start():
	var gdscript_tests := _collect_gdscript_tests()
	var rust_test_runner = TestRunnerSuite.new()
	var success: bool = rust_test_runner.run_all_tests(
		gdscript_tests, GDSCRIPT_TEST_FILES.size(), true, self
	)

	var exit_code: int = 0 if success else 1
	print("test-exiting with code ", exit_code)

	var testing_tools := TestingTools.new()
	testing_tools.exit_gracefully(exit_code)


func _collect_gdscript_tests() -> Array:
	var tests: Array = []
	for path in GDSCRIPT_TEST_FILES:
		var script: GDScript = load(path) as GDScript
		if script == null:
			push_error("test_runner: could not load %s" % path)
			continue
		for method_dict in script.get_script_method_list():
			var method_name: String = method_dict["name"]
			if not method_name.begins_with("test_"):
				continue
			var instance: Object = script.new()
			instance.method_name = method_name
			tests.append(instance)
	return tests
