extends RefCounted
class_name PromiseError

var _error_description: String = ""


static func create(description: String) -> PromiseError:
	var error = PromiseError.new()
	error._error_description = description
	return error


func get_error() -> String:
	return _error_description
