# util.gd
class_name Awaiter


class AllAwaiter:
	var _mask: int
	var _completed := false
	var promise: Promise = Promise.new()

	func _init(funcs: Array) -> void:
		var size := funcs.size()
		assert(size < 64)
		_mask = (1 << size) - 1
		for i in size:
			_call_func(i, funcs[i])

	func _call_func(i: int, f) -> void:
		@warning_ignore("redundant_await")
		if f is Promise:
			await f.awaiter()
		elif f is Callable:
			var res: Promise = f.call()
			if res != null:
				await res.awaiter()
		_mask &= ~(1 << i)

		if not _mask and not _completed:
			_completed = true
			promise.resolve()


static func all(funcs: Array) -> void:
	await AllAwaiter.new(funcs).promise.awaiter()
