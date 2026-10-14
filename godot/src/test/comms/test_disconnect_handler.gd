extends SceneTree

# Loads the real handler with its Global dependency replaced by test services.
const HANDLER_PATH := "res://src/ui/components/organisms/disconnect_handler/disconnect_handler.gd"
var failures := 0
var services: TestServices
var handler


class TestComms:
	extends Node
	signal disconnected(reason: int)
	signal pulse_recovered
	signal on_adapter_changed(voice: bool, adapter: String)
	var retries := 0
	var adapter_changes := 0

	func get_current_adapter_conn_str() -> String:
		return "archipelago:test"

	func change_adapter(_adapter: String) -> void:
		adapter_changes += 1

	func retry_pulse() -> void:
		retries += 1


class TestModals:
	extends Node
	signal session_ended_sign_in
	signal session_ended_retry
	signal session_ended_exit
	var shown: String = ""
	var closes := 0

	func async_show_session_ended_modal(_is_current: Callable = Callable()) -> void:
		shown = "session"

	func async_show_disconnected_modal(is_current: Callable = Callable()) -> void:
		if not is_current.is_valid() or is_current.call():
			shown = "outage"

	func async_show_room_closed_modal(_is_current: Callable = Callable()) -> void:
		shown = "closed"

	func async_show_ban_kicked_modal() -> void:
		shown = "ban"

	func close_current_modal() -> void:
		shown = ""
		closes += 1

	func close_recovered(_manager) -> void:
		if shown == "outage":
			close_current_modal()

	func clear_suppress_ban_kicked() -> void:
		pass


class TestServices:
	extends Node
	signal loading_started
	signal loading_finished
	signal on_menu_close
	var comms := TestComms.new()
	var modal_manager := TestModals.new()
	var modal_helpers = modal_manager
	var signed_out := false

	func sign_out() -> void:
		signed_out = true


func _init() -> void:
	_async_run.call_deferred()


func _expect(value: bool, label: String) -> void:
	if not value:
		failures += 1
		printerr(label)


func _new_handler() -> void:
	if is_instance_valid(handler):
		handler.free()
		services.comms.free()
		services.modal_manager.free()
		services.free()
	services = TestServices.new()
	root.add_child(services)
	var source := FileAccess.get_file_as_string(HANDLER_PATH)
	source = source.replace("class_name DisconnectHandler\n", "")
	source = source.replace("extends Node", "extends Node\nvar services")
	source = source.replace("Global.", "services.")
	source = source.replace("CommsDisconnectModal.", "services.modal_helpers.")
	var script := GDScript.new()
	script.source_code = source
	_expect(script.reload() == OK, "handler compiles")
	handler = script.new()
	handler.services = services
	root.add_child(handler)


func _async_run() -> void:
	_new_handler()
	services.comms.disconnected.emit(4)
	_expect(services.modal_manager.shown == "outage", "Pulse outage shows Retry modal")
	services.comms.on_adapter_changed.emit(false, "archipelago:test")
	_expect(handler._pulse_outage, "adapter creation does not clear Pulse outage")
	services.comms.pulse_recovered.emit()
	_expect(services.modal_manager.shown.is_empty(), "Pulse recovery closes outage")

	services.comms.disconnected.emit(4)
	services.modal_manager.session_ended_retry.emit()
	await process_frame
	_expect(services.comms.retries == 1, "Retry targets Pulse")
	_expect(services.comms.adapter_changes == 0, "Pulse retry does not race adapter retry")

	_new_handler()
	services.comms.disconnected.emit(3)
	services.comms.disconnected.emit(0)
	services.comms.disconnected.emit(4)
	services.comms.on_adapter_changed.emit(false, "archipelago:test")
	services.comms.pulse_recovered.emit()
	await create_timer(1.1).timeout
	_expect(services.comms.adapter_changes == 0, "eviction cancels pending retry")
	_expect(services.modal_manager.shown == "session", "eviction wins over stale recovery")
	services.modal_manager.session_ended_retry.emit()
	_expect(services.comms.retries == 0, "Session Ended cannot retry Pulse")
	services.loading_started.emit()
	services.loading_finished.emit()
	_expect(services.modal_manager.shown == "session", "loading preserves Session Ended")
	services.modal_manager.session_ended_sign_in.emit()
	_expect(services.signed_out, "Sign in logs out for reauthentication")

	_new_handler()
	services.loading_started.emit()
	services.comms.disconnected.emit(2)
	_expect(services.modal_manager.shown.is_empty(), "ban deferred during loading")
	services.loading_finished.emit()
	_expect(services.modal_manager.shown == "ban", "ban remains separate from Session Ended")

	_new_handler()
	services.loading_started.emit()
	services.comms.disconnected.emit(4)
	services.comms.pulse_recovered.emit()
	services.loading_finished.emit()
	_expect(services.modal_manager.shown.is_empty(), "recovery during loading cancels stale outage")

	handler.free()
	services.comms.free()
	services.modal_manager.free()
	services.free()
	print("disconnect handler tests: %d failures" % failures)
	quit(failures)
