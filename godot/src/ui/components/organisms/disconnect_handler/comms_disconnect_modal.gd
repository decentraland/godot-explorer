class_name CommsDisconnectModal
extends RefCounted


static func async_show_modal(
	manager, title, body, primary_label, primary_handler: Callable, is_current: Callable
) -> void:
	var had_modal := is_instance_valid(manager.current_modal)
	if not await manager._async_ensure_modal("CommsDisconnectModal.async_show_modal"):
		return
	if is_current.is_valid() and not is_current.call():
		# A cancelled first request can leave the newly instantiated default modal behind.
		var modal = manager.current_modal
		if (
			not had_modal
			and modal.button_primary.pressed.is_connected(
				Callable(modal, "_on_button_primary_pressed")
			)
		):
			manager.close_current_modal()
		return
	var modal = manager.current_modal
	modal.blocker = true
	modal.set_title(title)
	modal.set_body(body)
	modal.set_primary_button_text(primary_label)
	modal.set_secondary_button_text(manager.session_ended_secondary)
	modal.show_icon(Modal.MODAL_CONNECTION_ICON)
	modal.hide_url()
	modal.show()
	manager._disconnect_button_signals()
	modal.button_primary.pressed.connect(primary_handler)
	modal.button_secondary.pressed.connect(manager._on_session_ended_secondary)


static func close_recovered(manager) -> void:
	var modal = manager.current_modal
	if (
		is_instance_valid(modal)
		and modal.button_primary.pressed.is_connected(manager._on_session_ended_retry)
	):
		manager.close_current_modal()
