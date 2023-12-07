extends Node


func start():
	var rust_test_runner = TestRunnerSuite.new()
	var success: bool = rust_test_runner.run_all_tests([], 0, true, self)

	var exit_code: int = 0 if success else 1
	print("test-exiting with code ", exit_code)

	var testing_tools := TestingTools.new()
	testing_tools.exit_gracefully(exit_code)
