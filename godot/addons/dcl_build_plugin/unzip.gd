class_name UnZip


static func unzip_to_dir(zip_path: String, extract_to: String) -> bool:
	var reader = ZIPReader.new()
	var err = reader.open(zip_path)
	if err != OK:
		push_error("Failed to open ZIP archive: %s" % zip_path)
		return false

	var dir = DirAccess.open(extract_to)
	if dir == null:
		DirAccess.make_dir_recursive_absolute(extract_to)
		dir = DirAccess.open(extract_to)
		if dir == null:
			push_error("Failed to create target extraction directory: %s" % extract_to)
			return false

	var files = reader.get_files()
	for file_path in files:
		var full_path = extract_to.path_join(file_path)

		if file_path.ends_with("/"):
			DirAccess.make_dir_recursive_absolute(full_path)
			continue

		# Ensure parent directories exist
		DirAccess.make_dir_recursive_absolute(full_path.get_base_dir())

		var buffer = reader.read_file(file_path)
		var file = FileAccess.open(full_path, FileAccess.WRITE)
		if file:
			file.store_buffer(buffer)
			file.close()
		else:
			push_error("Failed to write extracted file: %s" % full_path)

	reader.close()
	return true
