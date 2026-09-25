class_name WorldSecretPrompt
extends RefCounted

## Collects the password of a shared-secret world and keeps asking until it is accepted,
## the player backs out, or the world stops answering (#2651).
##
## The retry lives inside one modal rather than reopening a new one per attempt: the
## input modal already keeps itself open on a recoverable error, and a fresh modal each
## time would lose the typed value and flash the screen between tries.

var _result: Destination
var _closed := Promise.new()

static var title := TranslationKey.new("MODAL_WORLD_PASSWORD_TITLE")
static var subtitle := TranslationKey.new("MODAL_WORLD_PASSWORD_SUBTITLE")
static var placeholder := TranslationKey.new("MODAL_WORLD_PASSWORD_PLACEHOLDER")
static var confirm := TranslationKey.new("MODAL_WORLD_PASSWORD_CONFIRM")
static var cancel := TranslationKey.new("COMMON_CANCEL")
static var wrong_password := TranslationKey.new("MODAL_WORLD_PASSWORD_WRONG")


func _init(dest: Destination) -> void:
	_result = dest


## Runs the prompt to its end. Returns the destination as it finished: READY once a
## password was accepted, RATE_LIMITED when the world cut us off, or the NEEDS_PASSWORD
## one it started from when the player cancelled.
func async_run() -> Destination:
	var modal: InputModal = await Global.modal_manager.async_show_input_modal(
		title, subtitle, placeholder, confirm, cancel, _is_plausible
	)
	if not is_instance_valid(modal):
		return _result

	modal.dismissable = false
	modal.set_submit_handler(_async_submit)
	modal.confirmed.connect(_on_closed.unbind(1))
	modal.failed.connect(_on_closed.unbind(1))
	modal.cancelled.connect(_on_closed)

	await PromiseUtils.async_awaiter(_closed)
	return _result


## Local gate on the confirm button only: the world is the authority on what its secret
## is, so anything non-empty is worth one attempt.
func _is_plausible(value: String) -> bool:
	return not value.strip_edges().is_empty()


func _async_submit(secret: String) -> Dictionary:
	_result = await DestinationResolver.async_retry_with_credential(_result, secret)
	match _result.state:
		Destination.State.READY:
			return {"status": InputModal.SUBMIT_OK}
		Destination.State.NEEDS_PASSWORD:
			return {"status": InputModal.SUBMIT_INVALID, "message": wrong_password.text()}
		_:
			# Rate limited, or the world stopped answering. Retrying here would only
			# spend more of the same budget, so the navigation ends and Navigator
			# surfaces the failure.
			return {"status": InputModal.SUBMIT_ERROR, "message": _result.failure_reason()}


func _on_closed() -> void:
	if not _closed.is_resolved():
		_closed.resolve()
