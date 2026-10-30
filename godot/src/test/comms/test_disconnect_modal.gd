extends SceneTree

const HELPER_PATH := "res://src/ui/components/organisms/disconnect_handler/comms_disconnect_modal.gd"
var failures := 0
var accept := true


class TestModal:
	extends Control
	var blocker := false
	var button_primary := Button.new()
	var button_secondary := Button.new()
	var title: String = ""

	func _on_button_primary_pressed() -> void:
		pass

	func set_title(value) -> void:
		title = value

	func set_body(_value) -> void:
		pass

	func set_primary_button_text(_value) -> void:
		pass

	func set_secondary_button_text(_value) -> void:
		pass

	func show_icon(_value) -> void:
		pass

	func hide_url() -> void:
		pass


class TestManager:
	extends Node
	var current_modal := TestModal.new()
	var session_ended_secondary := "Exit"
	var ban_kicked_title := "Banned"
	var ban_kicked_body := "Contact support"
	var session_ended_sign_in_primary := "Sign in"
	var closed := false

	func _async_ensure_modal(_site: String, urgent := false) -> bool:
		assert(urgent, "Disconnect modals must take priority")
		if not is_instance_valid(current_modal):
			current_modal = TestModal.new()
			current_modal.button_primary.pressed.connect(current_modal._on_button_primary_pressed)
		await get_tree().process_frame
		await get_tree().process_frame
		return true

	func _disconnect_button_signals() -> void:
		for button in [current_modal.button_primary, current_modal.button_secondary]:
			for connection in button.pressed.get_connections():
				button.pressed.disconnect(connection.callable)

	func _on_session_ended_retry() -> void:
		pass

	func _on_session_ended_secondary() -> void:
		pass

	func _on_session_ended_sign_in() -> void:
		pass

	func close_current_modal() -> void:
		closed = true


func _init() -> void:
	_async_run.call_deferred()


func _expect(value: bool, label: String) -> void:
	if not value:
		failures += 1
		printerr(label)


func _async_run() -> void:
	var source := FileAccess.get_file_as_string(HELPER_PATH)
	source = source.replace("class_name CommsDisconnectModal\n", "")
	source = source.replace("Modal.MODAL_CONNECTION_ICON", "0")
	source = source.replace(": TranslationKey", ": String")
	var helper := GDScript.new()
	helper.source_code = source
	_expect(helper.reload() == OK, "modal helper compiles")
	var manager := TestManager.new()
	root.add_child(manager)
	var is_current := func(): return accept
	helper.async_show_modal(
		manager, "Outage", "", "Retry", manager._on_session_ended_retry, is_current, true
	)
	accept = false
	manager.current_modal.title = "Session Ended"
	await process_frame
	await process_frame
	await process_frame
	_expect(
		manager.current_modal.title == "Session Ended", "stale outage cannot replace eviction modal"
	)

	accept = true
	await helper.async_show_modal(
		manager, "Session Ended", "", "Sign in", manager._on_session_ended_sign_in, is_current
	)
	helper.close_recovered(manager)
	_expect(not manager.closed, "recovery cannot close Session Ended")
	await helper.async_show_platform_banned(manager, is_current)
	helper.close_recovered(manager)
	_expect(manager.current_modal.title == "Banned", "platform ban has ban copy")
	_expect(
		manager.current_modal.button_primary.pressed.is_connected(
			manager._on_session_ended_sign_in
		),
		"platform ban offers Sign in"
	)
	_expect(not manager.closed, "recovery cannot close platform ban")
	await helper.async_show_modal(
		manager, "Outage", "", "Retry", manager._on_session_ended_retry, is_current, true
	)
	helper.close_recovered(manager)
	_expect(manager.closed, "recovery closes only a Retry modal")
	manager.closed = false
	await helper.async_show_modal(
		manager, "Room Closed", "", "Retry", manager._on_session_ended_retry, is_current
	)
	helper.close_recovered(manager)
	_expect(not manager.closed, "Pulse recovery cannot close Room Closed")
	manager.current_modal.button_primary.free()
	manager.current_modal.button_secondary.free()
	manager.current_modal.free()
	manager.closed = false
	accept = true
	helper.async_show_modal(
		manager, "Outage", "", "Retry", manager._on_session_ended_retry, is_current, true
	)
	accept = false
	await process_frame
	await process_frame
	await process_frame
	_expect(manager.closed, "cancelled creation closes its unused default modal")
	manager.current_modal.button_primary.free()
	manager.current_modal.button_secondary.free()
	manager.current_modal.free()
	manager.free()
	print("disconnect modal tests: %d failures" % failures)
	if failures == 0:
		print("[test_disconnect_modal] PASS")
	quit(failures)
