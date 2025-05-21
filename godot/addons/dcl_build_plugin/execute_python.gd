class_name ExecutePython

static func run(args: PackedStringArray):
	var output: Array = []
	prints("Executing python:", args)
	var exit_code = OS.execute("python", args, output)
	print("".join(output))
