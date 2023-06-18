extends Node
class_name RustHttpRequesterWrapper

signal request_completed(response: RequestResponse)

var _requester = RustHttpRequester.new()


func _ready():
	_requester = RustHttpRequester.new()


var tmp


func poll():
	tmp = _requester.poll()
	while tmp != null:
		emit_signal("request_completed", tmp)
		tmp = _requester.poll()
