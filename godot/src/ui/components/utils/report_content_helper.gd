class_name ReportContentHelper
extends RefCounted

## Opens the external "report content" Google Form, pre-filled with the current scene and wallet.
## Shared by Settings' Help & Support button and Discover's actions menu so both stay identical.

const _FORM_ID := "1FAIpQLSdD31D0GKROyxmrvM-KVStqdhyqF430crjaTtpemEiAqCHQbg"


static func open_form() -> void:
	var base_url := "https://docs.google.com/forms/d/e/" + _FORM_ID + "/viewform"

	var scene_name := ""
	if Global.scene_runner != null:
		var current_scene_id := Global.scene_runner.get_current_parcel_scene_id()
		if current_scene_id >= 0:
			scene_name = Global.scene_runner.get_scene_title(current_scene_id)

	var current_position: Vector2i = Global.get_config().last_parcel_position
	var scene_info := "%s (%d, %d)" % [scene_name, current_position.x, current_position.y]
	var wallet_id := Global.player_identity.get_address_str()

	var params := [
		"entry.60289947=" + scene_info.uri_encode(),
		"entry.927432836=" + wallet_id.uri_encode(),
	]

	Global.open_url(base_url + "?" + "&".join(params))
