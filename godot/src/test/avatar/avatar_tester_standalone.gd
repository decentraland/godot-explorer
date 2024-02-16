extends Control

@onready var sub_viewport_container = $SubViewportContainer
@onready var avatar: Avatar = sub_viewport_container.avatar
@onready var emote_wheel = $TabContainer/Emotes/EmoteWheel

@onready var text_edit_expr = $TabContainer/Expression/VBoxContainer/TextEdit_Expr
@onready var text_edit_result = $TabContainer/Expression/VBoxContainer/TextEdit_Result
@onready var line_edit_custom = $TabContainer/Emotes/LineEdit_Custom


# Called when the node enters the scene tree for the first time.
func _ready():
	var profile: DclUserProfile = DclUserProfile.new()
	var avatar_wf: DclAvatarWireFormat = profile.get_avatar()

	# Some emotes to test
	# urn:decentraland:matic:collections-v2:0x0b472c2c04325a545a43370b54e93c87f3d5badf:0
	# urn:decentraland:matic:collections-v2:0x54bf16bed39a02d5f8bda33664c72c59d367caf7:0
	# urn:decentraland:matic:collections-v2:0x70eb032d4621a51945b913c3f9488d50fc1fca38:0
	# urn:decentraland:matic:collections-v2:0x875146d1d26e91c80f25f5966a84b098d3db1fc8:1
	# urn:decentraland:matic:collections-v2:0xa25c20f58ac447621a5f854067b857709cbd60eb:7
	# urn:decentraland:matic:collections-v2:0xbada8a315e84e4d78e3b6914003647226d9b4001:10
	# urn:decentraland:matic:collections-v2:0xbada8a315e84e4d78e3b6914003647226d9b4001:11
	# urn:decentraland:matic:collections-v2:0x0c956c74518ed34afb7b137d9ddfdaea7ca13751:0

	avatar.avatar_loaded.connect(self._on_avatar_loaded)
	avatar.async_update_avatar(avatar_wf)


func _on_avatar_loaded():
	pass


func _on_button_open_wheel_pressed():
	emote_wheel.show()


func _on_text_edit_expr_text_changed():
	var expression = Expression.new()
	var err = expression.parse(text_edit_expr.text, ["Global"])

	if err != OK:
		text_edit_result.text = "Parse failed: " + expression.get_error_text()
		return

	var result = expression.execute([Global], self)
	if expression.has_execute_failed():
		text_edit_result.text = "Execution failed: " + expression.get_error_text()
		return

	text_edit_result.text = "Ok: " + str(result)


func _on_button_play_custom_pressed():
	avatar.emote_controller.async_play_emote(line_edit_custom.text)


func _on_button_clear_pressed():
	avatar.emote_controller.clean_unused_emotes()
