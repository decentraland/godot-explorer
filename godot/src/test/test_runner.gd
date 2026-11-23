extends Node

const GroundedGraceTest = preload("res://src/test/player/grounded_grace_test.gd")
const DebugCollectorCountsTest = preload(
	"res://src/test/debug_server/test_debug_collector_counts.gd"
)
const BeforeSendFreedInstanceTest = preload(
	"res://src/test/sentry/before_send_freed_instance_test.gd"
)
const BeforeSendScrubTest = preload("res://src/test/sentry/before_send_scrub_test.gd")
const ForceBackfaceCullingTest = preload("res://src/test/gltf/test_force_backface_culling.gd")


func start():
	var rust_test_runner = TestRunnerSuite.new()
	var gdscript_tests: Array = [
		GroundedGraceTest.new(),
		DebugCollectorCountsTest.new(),
		BeforeSendFreedInstanceTest.new(),
		BeforeSendScrubTest.new(),
		ForceBackfaceCullingTest.new(),
	]
	var success: bool = rust_test_runner.run_all_tests(gdscript_tests, 0, true, self)

	var exit_code: int = 0 if success else 1
	print("test-exiting with code ", exit_code)

	var testing_tools := TestingTools.new()
	testing_tools.exit_gracefully(exit_code)
