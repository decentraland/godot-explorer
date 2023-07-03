extends Node

func start():
	var rust_test_runner = TestRunnerSuite.new()
	var success: bool = rust_test_runner.run_all_tests([], 0, true, self)

	var exit_code: int = 0 if success else 1
	get_tree().quit(exit_code)
