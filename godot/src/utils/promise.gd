# Promises for GDScript
# Every function that must be awaited has an `async_` prefix

class_name PromiseUtils


static func resolved(data = null) -> Promise:
	var promise := Promise.new()
	promise.resolve_with_data(data)
	return promise


static func rejected(reason: String) -> Promise:
	var promise := Promise.new()
	promise.reject(reason)
	return promise


static func async_awaiter(promise: Promise) -> Variant:
	if promise == null:
		printerr("try to await a null promise")
		return null

	if !promise.is_resolved():
		var thread_id := OS.get_thread_caller_id()
		await promise.on_resolved

		# This happen because emitting signal with call_deferred
		#  enqueues the call in the main thread (MessageQueue)
		if thread_id != OS.get_thread_caller_id():
			var main_thread_id := OS.get_main_thread_id()
			printerr(
				"Thread different after await in async_awaiter ",
				OS.get_thread_caller_id(),
				" != ",
				thread_id,
				" main=",
				main_thread_id
			)

	# var some = Node.new()
	# some.call_deferred_thread_group()

	var data = promise.get_data()
	if data is Promise:  # Chain promises
		return await PromiseUtils.async_awaiter(data)

	return data


# Internal helper function
class _Internal:
	static func async_call_and_get_promise(f) -> Promise:
		if f is Promise:
			return f

		if f is Callable:
			var res = await f.call()
			if res is Promise:
				return res

			printerr("Func doesn't return a Promise")
			return null

		printerr("Func is not a callable nor promise")
		return null


class AllAwaiter:
	var results: Array = []
	var _mask: int
	var _promise: Promise = Promise.new()

	func _init(funcs: Array) -> void:
		var size := funcs.size()
		if size == 0:  # inmediate resolve, no funcs to await...
			_promise.resolve()
			return

		results.resize(size)
		results.fill(null)  # by default, the return will be null
		assert(size < 64)
		_mask = (1 << size) - 1
		for i in size:
			_async_call_func(i, funcs[i])

	func _async_call_func(i: int, f) -> void:
		@warning_ignore("redundant_await")
		var promise = await PromiseUtils._Internal.async_call_and_get_promise(f)
		var data = await PromiseUtils.async_awaiter(promise)
		results[i] = data

		_mask &= ~(1 << i)

		if not _mask and not _promise.is_resolved():
			_promise.resolve_with_data(results)


class AllAwaiterEx:
	var results: Array = []
	var resolved: Array = []
	var _promise: Promise = Promise.new()

	func _init(funcs: Array) -> void:
		var size := funcs.size()
		if size == 0:  # inmediate resolve, no funcs to await...
			_promise.resolve()
			return

		resolved.resize(size)
		resolved.fill(false)
		results.resize(size)
		results.fill(null)  # by default, the return will be null
		for i in size:
			_async_call_func(i, funcs[i])

	func _async_call_func(i: int, f) -> void:
		@warning_ignore("redundant_await")
		var promise = await PromiseUtils._Internal.async_call_and_get_promise(f)
		var data = await PromiseUtils.async_awaiter(promise)
		results[i] = data
		resolved[i] = true

		if not resolved.has(false) and not _promise.is_resolved():
			_promise.resolve_with_data(results)


class AnyAwaiter:
	var _promise: Promise = Promise.new()

	func _init(funcs: Array) -> void:
		var size := funcs.size()
		if size == 0:  # inmediate resolve, no funcs to await...
			_promise.resolve()
			return
		for i in size:
			_async_call_func(i, funcs[i])

	func _async_call_func(_i: int, f) -> void:
		@warning_ignore("redundant_await")
		var promise: Promise = await PromiseUtils._Internal.async_call_and_get_promise(f)
		var res = await PromiseUtils.async_awaiter(promise)

		# Promise.async_any ignores promises with errors
		if !promise.is_rejected() and not _promise.is_resolved():
			_promise.resolve_with_data(res)


class RaceAwaiter:
	var _promise: Promise = Promise.new()

	func _init(funcs: Array) -> void:
		var size := funcs.size()
		if size == 0:  # inmediate resolve, no funcs to await...
			_promise.resolve()
			return
		for i in size:
			_async_call_func(i, funcs[i])

	func _async_call_func(_i: int, f) -> void:
		@warning_ignore("redundant_await")
		var promise: Promise = await PromiseUtils._Internal.async_call_and_get_promise(f)
		var res = await PromiseUtils.async_awaiter(promise)

		# Promise.async_race doesn't ignore on error, you get the first one, with or without an error
		if not _promise.is_resolved():
			_promise.resolve_with_data(res)


# `async_all` is a static function that takes an array of functions (`funcs`)
# and returns an array. It awaits the resolution of all the given functions.
# Each function in the array is expected to be a coroutine or a function
# that returns a promise.
static func async_all(funcs: Array) -> Array:
	if funcs.is_empty():
		return []
	if funcs.size() < 64:
		return await PromiseUtils.async_awaiter(AllAwaiter.new(funcs)._promise)

	return await PromiseUtils.async_awaiter(AllAwaiterEx.new(funcs)._promise)


# `async_any` is a static function similar to `async_all`, but it resolves as soon as any of the
# functions in the provided array resolves. It returns the result of the first function
# that resolves. It ignores the rejections (differently from async_race)
static func async_any(funcs: Array) -> Variant:
	return await PromiseUtils.async_awaiter(AnyAwaiter.new(funcs)._promise)


# `async_race` is another static function that takes an array of functions and returns
# a variant. It behaves like a race condition, returning the result of the function
# that completes first, even if it fails (differently from async_any)
static func async_race(funcs: Array) -> Variant:
	return await PromiseUtils.async_awaiter(RaceAwaiter.new(funcs)._promise)
