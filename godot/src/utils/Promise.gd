extends RefCounted
class_name Promise

signal _on_resolved

var _resolved: bool = false
var _data: Variant = null


func resolve():
	_on_resolved.emit()
	_resolved = true


func resolve_with_data(data):
	_data = data
	resolve()


func get_data():
	return _data


func reject(reason: String):
	_data = PromiseError.create(reason)
	printerr("Promise rejected, reason: ", reason)
	resolve()


func is_resolved() -> bool:
	return _resolved


func awaiter() -> Variant:
	if !_resolved:
		await _on_resolved
	return _data
