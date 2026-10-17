class_name RnReqUtil
extends RefCounted
## Shared helpers for the req/res handlers: reply shapes, promise unwrapping, JSON-safe data.


static func ok(result: Variant = {}) -> Dictionary:
	return {"ok": true, "result": result}


static func fail(message: String) -> Dictionary:
	return {"ok": false, "error": message}


## Awaits a Promise; resolves to ok(data) or fail(message) so handlers never throw.
static func async_unwrap(promise: Promise, wrap: Callable = Callable()) -> Dictionary:
	var data: Variant = await PromiseUtils.async_awaiter(promise)
	if promise == null or promise.is_rejected():
		return fail(PromiseUtils.get_error_message(promise))
	if data is PromiseError:
		return fail(data.get_error())
	return ok(wrap.call(data) if wrap.is_valid() else json_safe(data))


## Round-trips through JSON so Godot-only types (Vector2, Color, objects) cannot reach the bridge.
static func json_safe(value: Variant) -> Variant:
	var text := JSON.stringify(value)
	if text.is_empty():
		return null
	return JSON.parse_string(text)


static func str_param(params: Dictionary, key: String) -> String:
	return str(params.get(key, "")).strip_edges()
