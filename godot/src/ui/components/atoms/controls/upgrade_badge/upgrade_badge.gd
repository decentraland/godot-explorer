class_name UpgradeBadge
extends PanelContainer

## Red dot indicator shown when the current account is a thirdweb guest
## that hasn't linked an email yet. Manages its own visibility.


# gdlint:ignore = async-function-name
func _ready() -> void:
	visible = false
	_async_update_visibility()


# gdlint:ignore = async-function-name
func _async_update_visibility() -> void:
	visible = false
	if Global.player_identity == null or not Global.player_identity.is_thirdweb_guest():
		return

	var anchor: String = Global.get_device_anchor_id()
	var promise: Promise = Global.player_identity.async_refresh_thirdweb_upgrade_state(anchor)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		visible = not Global.player_identity.is_thirdweb_guest_upgraded()
		return
	visible = not result
