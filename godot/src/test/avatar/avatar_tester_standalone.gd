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

	# Test simple without mouth/eyes
	# var wearables: PackedStringArray = [
	# 	"urn:decentraland:off-chain:base-avatars:yellow_tshirt",
	# 	"urn:decentraland:off-chain:base-avatars:soccer_pants",
	# 	"urn:decentraland:off-chain:base-avatars:comfy_sport_sandals",
	# 	"urn:decentraland:off-chain:base-avatars:keanu_hair",
	# 	"urn:decentraland:off-chain:base-avatars:granpa_beard"
	# ]

	# With force_render and skin
	#var wearables: PackedStringArray = [
	#"urn:decentraland:matic:collections-v2:0xa83c8951dd73843bf5f7e9936e72a345a3e79874:8:842498333348457493583344221469363458551160763204392890034487820295",
	#"urn:decentraland:matic:collections-v2:0x89dd5ee70e4fa4400b02bac1145f5260bb827a24:0:1",
	#"urn:decentraland:matic:collections-v2:0x83a600dfb82a4806f60f5ee5bf02c306639fe385:0:24",
	#"urn:decentraland:matic:collections-v2:0xaf26e33ccea26e697e71b005499f820b95821c04:0:7",
	#"urn:decentraland:matic:collections-v2:0xa83c8951dd73843bf5f7e9936e72a345a3e79874:7:737186041679900306885426193785693026232265667803843778780176842765",
	#"urn:decentraland:matic:collections-v2:0xfb1d9d5dbb92f2dccc841bd3085081bb1bbeb04d:13:1369059791691243427072934359887715620145636240207138446306042707992",
	#"urn:decentraland:matic:collections-v2:0xd62cb20c1fc76962aae30e7067babdf66463ffe3:0:6",
	#"urn:decentraland:matic:collections-v2:0x844a933934fba88434dfade0b04b1d211e92d7c4:0:57",
	#"urn:decentraland:matic:collections-v2:0x7d65d7ca3d44814c697aea3a1db45da330546e7b:0:55",
	#"urn:decentraland:matic:collections-v2:0x3da9e56ce30dc83f6415ce35acdcc71c236e1829:2:210624583337114373395836055367340864637790190801098222508621955117",
	#"urn:decentraland:matic:collections-v2:0xb055cc2916bf8857ad1ae19b0c8a4d128180c4a9:0:115",
	#"urn:decentraland:matic:collections-v2:0x2929bbb4f18b40ac52a7f0b91629c695e3f96504:1:105312291668557186697918027683670432318895095400549111254310977567",
	#"urn:decentraland:matic:collections-v2:0x34f266ed68b877dd98ee2697f09bc0481be828bd:0:90",
	#"urn:decentraland:matic:collections-v2:0xf3df68b5748f1955f68b4fefda3f65b2e0250325:0:100",
	#"urn:decentraland:off-chain:base-avatars:f_eyebrows_00",
	#"urn:decentraland:matic:collections-v2:0xac3b666704ec025b2e59f22249830a07b6fb9573:0:30"
	#]
	#avatar_wf.set_force_render([
	#"helmet",
	#"lower_body",
	#"tiara",
	#"hands_wear",
	#"feet",
	#"upper_body"
	#])

	# Test Mask
	#var wearables: PackedStringArray = [
	#"urn:decentraland:off-chain:base-avatars:Thunder_earring",
	#"urn:decentraland:off-chain:base-avatars:eyebrows_06",
	#"urn:decentraland:off-chain:base-avatars:eyes_22",
	#"urn:decentraland:off-chain:base-avatars:horseshoe_beard",
	#"urn:decentraland:off-chain:base-avatars:modern_hair",
	#"urn:decentraland:off-chain:base-avatars:mouth_09",
	#"urn:decentraland:matic:collections-v2:0x0dc28547b88100eb6b3f3890f0501607aa5dd6be:0:3202",
	#"urn:decentraland:matic:collections-v2:0xbf83965191065487db0644812649d5238435c723:1:105312291668557186697918027683670432318895095400549111254310978934",
	#]
	#
	#var wearables: PackedStringArray = [
	#"urn:decentraland:off-chain:base-avatars:mouth_03", "urn:decentraland:off-chain:base-avatars:eyes_08", "urn:decentraland:off-chain:base-avatars:eyebrows_00", "urn:decentraland:off-chain:base-avatars:chin_beard", "urn:decentraland:off-chain:base-avatars:cool_hair", "urn:decentraland:ethereum:collections-v1:cybermike_cybersoldier_set:cybersoldier_boots_feet:25", "urn:decentraland:ethereum:collections-v1:cybermike_cybersoldier_set:cybersoldier_helmet:29", "urn:decentraland:ethereum:collections-v1:cybermike_cybersoldier_set:cybersoldier_leggings_lower_body:34", "urn:decentraland:ethereum:collections-v1:cybermike_cybersoldier_set:cybersoldier_torso_upper_body:35"
	#]

	var wearables: PackedStringArray = [
		"urn:decentraland:matic:collections-v2:0xded1e53d7a43ac1844b66c0ca0f02627eb42e16d:9:947810625017014680281262249153033890870055858604942001288798798198",
		"urn:decentraland:matic:collections-v2:0xb0ddfa06521df41e0c4f9738734943e91741c2b0:2:210624583337114373395836055367340864637790190801098222508621955104",
		"urn:decentraland:matic:collections-v2:0xca520eea5aadff51b48d9e9b3038001a751139ca:0:46",
		"urn:decentraland:matic:collections-v2:0x7cf465d90bcb0c7da69757e6ba029123cf9b64db:0:1",
		"urn:decentraland:off-chain:base-avatars:mouth_07",
		"urn:decentraland:off-chain:base-avatars:f_eyebrows_02",
		"urn:decentraland:off-chain:base-avatars:eyes_08",
		"urn:decentraland:matic:collections-v2:0xf73841bd6ee00efd3036a54bffc5f914ea1ef469:1:105312291668557186697918027683670432318895095400549111254310977559",
		"urn:decentraland:matic:collections-v2:0xfb26d0b332d8954b3a049276a0865c8f7d106c31:0:41",
		"urn:decentraland:matic:collections-v2:0x37a21dcf21120af16da54542720abbe351670af8:1:105312291668557186697918027683670432318895095400549111254310977581",
		"urn:decentraland:ethereum:collections-v1:halloween_2020:hwn_2020_ghostblaster_tiara:3855",
		"urn:decentraland:matic:collections-v2:0xded1e53d7a43ac1844b66c0ca0f02627eb42e16d:3:315936875005671560093754083051011296956685286201647333762932932647"
	]

	avatar_wf.set_body_shape("urn:decentraland:off-chain:base-avatars:BaseMale")
	avatar_wf.set_wearables(wearables)

	# FEMALE
	#var wearables: PackedStringArray = ["urn:decentraland:off-chain:base-avatars:colored_sweater", "urn:decentraland:off-chain:base-avatars:f_african_leggins", "urn:decentraland:off-chain:base-avatars:citycomfortableshoes", "urn:decentraland:off-chain:base-avatars:hair_undere", "urn:decentraland:off-chain:base-avatars:black_sun_glasses", "urn:decentraland:off-chain:base-avatars:f_mouth_05"]
	#avatar_wf.set_body_shape("urn:decentraland:off-chain:base-avatars:BaseFemale")
	#avatar_wf.set_wearables(wearables)

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
	await avatar.async_update_avatar(avatar_wf, "")
	#download_avatar()


func download_wearable(id: String, body_shape_id: String):
	var wearable = Global.content_provider.get_wearable(id)
	var dir_name = "user://downloaded/" + wearable.get_display_name().validate_filename()
	var content_mapping := wearable.get_content_mapping()

	DirAccess.make_dir_recursive_absolute(dir_name)

	for file_name in content_mapping.get_files():
		var file_hash = content_mapping.get_hash(file_name)
		var file_path = dir_name + "/" + file_name.validate_filename()
		if FileAccess.file_exists("user://content/" + file_hash):
			DirAccess.copy_absolute("user://content/" + file_hash, file_path)


func download_avatar():
	var body_shape = Global.content_provider.get_wearable(avatar.avatar_data.get_body_shape())
	download_wearable(avatar.avatar_data.get_body_shape(), avatar.avatar_data.get_body_shape())
	for wearable_id in avatar.avatar_data.get_wearables():
		download_wearable(wearable_id, avatar.avatar_data.get_body_shape())


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
