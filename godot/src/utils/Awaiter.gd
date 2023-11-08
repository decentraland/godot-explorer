# util.gd
class_name Awaiter


class AllAwaiter:
	var _mask: int
	var _promise: Promise = Promise.new()

	func _init(funcs: Array) -> void:
		var size := funcs.size()
		if size == 0:  # inmediate resolve, no funcs to await...
			_promise.resolve()
			return

		assert(size < 64)
		_mask = (1 << size) - 1
		for i in size:
			_call_func(i, funcs[i])

	func _call_func(i: int, f) -> void:
		@warning_ignore("redundant_await")
		if f is Promise:
			await f.co_awaiter()
		elif f is Callable:
			var res: Promise = f.call()
			if res != null:
				await res.co_awaiter()
		_mask &= ~(1 << i)

		if not _mask and not _promise.is_resolved():
			_promise.resolve()


static func co_all(funcs: Array) -> void:
	await AllAwaiter.new(funcs)._promise.co_awaiter()
