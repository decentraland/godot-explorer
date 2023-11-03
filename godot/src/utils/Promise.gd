extends RefCounted
class_name Promise

signal _on_resolved

var _resolved = false
var _error: String = ""
var _rejected = false
var _data = null

func resolve():
	_on_resolved.emit()
	_resolved = true

func resolve_with_data(data):
	_data = data
	resolve()

func get_data():
	return _data

func reject(reason: String):
	_rejected = true
	_error = reason
	printerr("Promise rejected, reason: ", reason)
	resolve()


func is_resolved() -> bool:
	return _resolved


func awaiter() -> bool:
	if !_resolved:
		await _on_resolved
	return !_rejected


func get_error() -> String:
	return _error
