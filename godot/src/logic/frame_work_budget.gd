class_name FrameWorkBudget
extends RefCounted

## Shared per-frame budget for heavy main-thread steps (avatar assembly, emote merges,
## GLTF realize, video surface init). `await FrameWorkBudget.async_acquire(label)` right
## before the step: it returns at once while this frame has room, otherwise in a later frame.
##
## Room: the first step of a frame always runs; more run while fewer than MAX_STEPS ran and
## less than BUDGET_USEC passed since the frame's process step began. Priority callers (local
## player, UI previews) skip the step cap and are served first. Waiters alternate between
## "oldest avatar build first" (avatars finish one after another) and plain arrival order
## (nothing starves). A waiter whose owner was freed is dropped without resuming.

const BUDGET_USEC := 2_000
const MAX_STEPS := 1
## While the loading screen is up, throughput matters more than frame time.
const LOADING_BUDGET_USEC := 12_000
const LOADING_MAX_STEPS := 16

## True between Global.loading_started and Global.loading_finished.
static var loading := false

static var _queue: Array[Ticket] = []
static var _frame := -1
static var _frame_start_usec := 0
static var _steps := 0
static var _seq := 0
# owner instance id -> order of its current build (see begin_build)
static var _build_order: Dictionary = {}
static var _driver: Driver = null


class Ticket:
	extends RefCounted
	signal granted
	var label := ""
	var owner_id := 0
	var order := 0
	var arrival := 0
	var priority := false


class Driver:
	extends RefCounted

	func on_process_frame() -> void:
		# A load cancelled back to Discover never emits loading_finished.
		if FrameWorkBudget.loading:
			var tree := Engine.get_main_loop() as SceneTree
			var global = tree.root.get_node_or_null("Global") if tree != null else null
			if global != null and global.get_explorer() == null:
				FrameWorkBudget.loading = false
		FrameWorkBudget._drain()

	func on_loading_started() -> void:
		FrameWorkBudget.loading = true

	func on_loading_finished() -> void:
		FrameWorkBudget.loading = false


## Hooks the per-frame drain and the loading-screen signals. Idempotent.
static func ensure_driver() -> void:
	if _driver != null:
		return
	var tree := Engine.get_main_loop() as SceneTree
	if tree == null:
		return
	_driver = Driver.new()
	tree.process_frame.connect(_driver.on_process_frame)
	var global := tree.root.get_node_or_null("Global")
	if global != null:
		global.loading_started.connect(_driver.on_loading_started)
		global.loading_finished.connect(_driver.on_loading_finished)


static func async_acquire(label: String, owner: Object = null, priority := false) -> void:
	ensure_driver()
	if _driver == null:
		return
	if not _has_waiting(priority) and _has_room(priority):
		_steps += 1
		return
	_seq += 1
	var ticket := Ticket.new()
	ticket.label = label
	ticket.owner_id = owner.get_instance_id() if owner != null else 0
	ticket.arrival = _seq
	ticket.order = _build_order.get(ticket.owner_id, _seq)
	ticket.priority = priority
	_queue.append(ticket)
	await ticket.granted


## Remote avatars (children of Global.avatars, the AvatarScene) wait in line; every other
## avatar (local player, previews, AvatarShape NPCs) has priority.
static func async_acquire_for_avatar(label: String, avatar: Node) -> void:
	await async_acquire(label, avatar, not is_background_avatar(avatar))


static func is_background_avatar(avatar: Node) -> bool:
	return avatar.get_parent() is AvatarScene


## Steps of `owner` queue by when its build began, until end_build.
static func begin_build(owner: Object) -> void:
	var id := owner.get_instance_id()
	if not _build_order.has(id):
		_seq += 1
		_build_order[id] = _seq


static func end_build(owner: Object) -> void:
	_build_order.erase(owner.get_instance_id())


## Whether this frame still has time left (used by callers that batch inside one step).
static func has_time() -> bool:
	_roll_frame()
	var budget := LOADING_BUDGET_USEC if loading else BUDGET_USEC
	return Time.get_ticks_usec() - _frame_start_usec < budget


static func _roll_frame() -> void:
	var frame := Engine.get_process_frames()
	if frame != _frame:
		_frame = frame
		_steps = 0
		_frame_start_usec = Time.get_ticks_usec()


static func _has_room(priority: bool) -> bool:
	_roll_frame()
	if _steps == 0:
		return true
	if not has_time():
		return false
	return priority or _steps < (LOADING_MAX_STEPS if loading else MAX_STEPS)


static func _has_waiting(priority: bool) -> bool:
	if not priority:
		return not _queue.is_empty()
	for ticket in _queue:
		if ticket.priority:
			return true
	return false


static func _drain() -> void:
	_roll_frame()
	while not _queue.is_empty():
		var ticket := _pick()
		if ticket == null or not _has_room(ticket.priority):
			return
		_queue.erase(ticket)
		_steps += 1
		ticket.granted.emit()


# Priority first; otherwise odd steps take the oldest arrival, even steps the oldest build.
static func _pick() -> Ticket:
	var by_arrival := _steps % 2 == 1
	var best: Ticket = null
	for ticket in _queue.duplicate():
		if ticket.owner_id != 0 and not is_instance_id_valid(ticket.owner_id):
			_queue.erase(ticket)
			_build_order.erase(ticket.owner_id)
			continue
		if best == null or _before(ticket, best, by_arrival):
			best = ticket
	return best


static func _before(a: Ticket, b: Ticket, by_arrival: bool) -> bool:
	if a.priority != b.priority:
		return a.priority
	if a.priority or by_arrival or a.order == b.order:
		return a.arrival < b.arrival
	return a.order < b.order
