class_name UpgradeBadge
extends PanelContainer

## Red dot indicator shown when the current account is a thirdweb guest
## that hasn't linked an email yet. Manages its own visibility.
## Re-evaluates every time it enters the tree or becomes visible, using the
## cached upgraded flag (no network call) so it reacts instantly after upgrade.


func _ready() -> void:
	visible = false
	visibility_changed.connect(_on_visibility_changed)
	Global.guest_upgrade_state_refreshed.connect(_on_guest_upgrade_state_refreshed)
	Global.orientation_changed.connect(_on_orientation_changed)


func _on_orientation_changed(_is_portrait: bool) -> void:
	visible = _should_show()


func refresh_visibility() -> void:
	visible = _should_show()


func _on_visibility_changed() -> void:
	if visible and not _should_show():
		visible = false


func _on_guest_upgrade_state_refreshed(is_upgraded: bool) -> void:
	if is_upgraded:
		visible = false
	else:
		visible = _should_show()


func _should_show() -> bool:
	if not Global.is_orientation_portrait():
		return false
	if Global.player_identity == null:
		return false
	if not Global.player_identity.is_thirdweb_guest():
		return false
	return not Global.player_identity.is_thirdweb_guest_upgraded()
