class_name RnRequests
extends RefCounted
## Generic request/response channel: `{type:"req", id, method, params}` in, a `res` event out.
## Handlers return {ok, result|error}; anything that throws or rejects becomes {ok:false}.

var _emit: Callable
var _settings: RnReqSettings
var _social := RnReqSocial.new()
var _profile := RnReqProfile.new()
var _misc := RnReqMisc.new()
var _scene := RnReqScene.new()


func _init(emit: Callable) -> void:
	_emit = emit
	_settings = RnReqSettings.new(emit)


func connect_signals() -> void:
	_settings.connect_signals()


## Returns true when the command was a request (it is answered asynchronously).
func handle(cmd: Dictionary) -> bool:
	if cmd.get("type", "") != "req":
		return false
	_async_run(str(cmd.get("id", "")), str(cmd.get("method", "")), cmd.get("params"))
	return true


func _async_run(id: String, method: String, raw_params: Variant) -> void:
	var params: Dictionary = raw_params if raw_params is Dictionary else {}
	var reply: Dictionary = await _async_dispatch(method, params)
	reply["id"] = id
	_emit.call("res", reply)


func _async_dispatch(method: String, params: Dictionary) -> Dictionary:
	var reply: Variant
	match method.get_slice(".", 0):
		"settings":
			reply = await _settings.async_handle(method, params)
		"friends":
			reply = await _social.async_handle(method, params)
		"profile", "account":
			reply = await _profile.async_handle(method, params)
		"notifications", "fetch", "places", "clipboard", "screenshot":
			reply = await _misc.async_handle(method, params)
		"scene":
			reply = await _scene.async_handle(method, params)
		_:
			return RnReqUtil.fail("unknown method " + method)
	return reply if reply is Dictionary else RnReqUtil.fail("handler returned no reply")
