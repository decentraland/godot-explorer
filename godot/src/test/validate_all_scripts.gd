# validate_all_scripts.gd
extends Node


func _ready():
	print("Starting comprehensive script validation...")
	var error_count = 0
	var checked_count = 0

	var scripts = find_all_scripts("res://")

	for script_path in scripts:
		checked_count += 1

		# Load without forcing recompilation to avoid crashes
		var script = ResourceLoader.load(script_path, "GDScript")

		if script == null or not script is Script:
			print("❌ ERROR: Failed to load: ", script_path)
			error_count += 1
		else:
			# Check if script has valid source code
			if not script.has_source_code() or script.get_source_code() == "":
				print("❌ ERROR: Empty or invalid script: ", script_path)
				error_count += 1
			else:
				# Check if script can be instantiated (will fail for scripts with parse errors)
				# Use can_instantiate() which doesn't trigger crashes
				if script.can_instantiate():
					print("✓ OK: ", script_path)
				else:
					print("❌ ERROR: Cannot instantiate (likely parse error): ", script_path)
					error_count += 1

	print("\n--- Validation Summary ---")
	print("Scripts checked: ", checked_count)
	print("Errors found: ", error_count)

	if error_count > 0:
		get_tree().quit(1)
	else:
		print("All scripts validated successfully!")
		get_tree().quit(0)


func find_all_scripts(path: String) -> Array:
	var scripts = []
	var dir = DirAccess.open(path)

	if dir:
		dir.list_dir_begin()
		var file_name = dir.get_next()

		while file_name != "":
			if dir.current_is_dir():
				if not file_name.begins_with("."):
					scripts.append_array(find_all_scripts(path + "/" + file_name))
			elif file_name.ends_with(".gd"):
				scripts.append(path + "/" + file_name)

			file_name = dir.get_next()
		dir.list_dir_end()

	return scripts
