extends Control

@onready var sub_viewport_container = $SubViewportContainer
@onready var avatar: Avatar = sub_viewport_container.avatar
@onready var emote_wheel = $TabContainer/Emotes/EmoteWheel

@onready var text_edit_expr = $TabContainer/Expression/VBoxContainer/TextEdit_Expr
@onready var text_edit_result = $TabContainer/Expression/VBoxContainer/TextEdit_Result

# Called when the node enters the scene tree for the first time.
func _ready():
	var profile: DclUserProfile = DclUserProfile.new()
	var avatar_wf: DclAvatarWireFormat = profile.get_avatar()
	
	var emotes = PackedStringArray([                       
		"handsair",
		"wave",
		"urn:decentraland:matic:collections-v2:0x0b472c2c04325a545a43370b54e93c87f3d5badf:0",
		"urn:decentraland:matic:collections-v2:0x54bf16bed39a02d5f8bda33664c72c59d367caf7:0",
		"urn:decentraland:matic:collections-v2:0x70eb032d4621a51945b913c3f9488d50fc1fca38:0",
		"urn:decentraland:matic:collections-v2:0x875146d1d26e91c80f25f5966a84b098d3db1fc8:1:105312291668557186697918027683670432318895095400549111254310981119",
		"urn:decentraland:matic:collections-v2:0xa25c20f58ac447621a5f854067b857709cbd60eb:7:737186041679900306885426193785693026232265667803843778780176846151",
		"urn:decentraland:matic:collections-v2:0xbada8a315e84e4d78e3b6914003647226d9b4001:10:1053122916685571866979180276836704323188950954005491112543109777455",
		"urn:decentraland:matic:collections-v2:0xbada8a315e84e4d78e3b6914003647226d9b4001:11:1158435208354129053677098304520374755507846049406040223797420753072",
		"shrug"
	])
	avatar_wf.set_emotes(emotes)
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
