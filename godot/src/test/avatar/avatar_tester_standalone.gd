extends Control

const GLIDER_PROP_SCENE: PackedScene = preload(
	"res://src/decentraland_components/avatar/glider_prop.tscn"
)

var avatar_list: Array = []

# Emote batch testing
var emote_batch_test_running: bool = false
var emote_batch_test_index: int = 0
var emote_batch_test_list: Array = []
var emote_batch_test_delay: float = 3.0  # seconds between each emote
var emote_batch_auto_mode: bool = false  # Auto-run and quit mode (CLI)

# Emote batch test UI (will be created dynamically)
var batch_test_button: Button = null
var batch_test_label: Label = null
var batch_test_timer: Timer = null

# Animation tester state
var _anim_library_keys: Array = []
var _current_glider_prop: Node = null

@onready var sub_viewport_container = $SubViewportContainer
@onready var avatar: Avatar = sub_viewport_container.avatar
@onready var emote_wheel = $TabContainer/Emotes/EmoteWheel

@onready var text_edit_expr = $TabContainer/Expression/VBoxContainer/TextEdit_Expr
@onready var text_edit_result = $TabContainer/Expression/VBoxContainer/TextEdit_Result
@onready var line_edit_custom = $TabContainer/Emotes/LineEdit_Custom
@onready var option_button_avatar_list = $TabContainer/Avatars/OptionButton_AvatarList

@onready var spinner = $Spinner
@onready var line_edit_profile_entity = $TabContainer/Avatars/LineEdit_ProfileEntity
@onready var outline_checkbox = %OutlineCheckBox

@onready var anim_root = $TabContainer/Animations/VBoxContainer
@onready
var option_button_library: OptionButton = anim_root.get_node("HBox_Library/OptionButton_Library")
@onready var option_button_animation: OptionButton = anim_root.get_node(
	"HBox_Animation/OptionButton_Animation"
)
@onready var label_now_playing: Label = anim_root.get_node("Label_NowPlaying")
@onready var check_walk: CheckBox = anim_root.get_node("Grid_Flags/CheckBox_Walk")
@onready var check_jog: CheckBox = anim_root.get_node("Grid_Flags/CheckBox_Jog")
@onready var check_run: CheckBox = anim_root.get_node("Grid_Flags/CheckBox_Run")
@onready var check_rise: CheckBox = anim_root.get_node("Grid_Flags/CheckBox_Rise")
@onready var check_fall: CheckBox = anim_root.get_node("Grid_Flags/CheckBox_Fall")
@onready var check_land: CheckBox = anim_root.get_node("Grid_Flags/CheckBox_Land")
@onready var check_show_glider: CheckBox = anim_root.get_node("CheckBox_ShowGlider")
@onready var option_button_glider_anim: OptionButton = anim_root.get_node(
	"HBox_GliderClip/OptionButton_GliderAnim"
)


func _ready():
	spinner.hide()
	load_avatar_list()
	avatar.avatar_loaded.connect(self._on_avatar_loaded)
	option_button_avatar_list.selected = -1
	option_button_avatar_list.text = "Select an avatar"

	# Connect outline checkbox
	if outline_checkbox:
		outline_checkbox.toggled.connect(_on_outline_toggled)

	# Visual enhance
	var viewport: Viewport = sub_viewport_container.subviewport.get_viewport()
	viewport.use_debanding = true
	viewport.scaling_3d_scale = 2.0
	RenderingServer.screen_space_roughness_limiter_set_active(true, 4.0, 1.0)

	# Setup emote batch tester UI
	_setup_emote_batch_tester()

	# Populate animation tester dropdowns once the avatar is ready
	avatar.ready.connect(_refresh_animation_lists)
	_refresh_animation_lists()
	_populate_glider_anim_list()

	# Check for CLI auto-run mode: --emote-test
	if Global.cli.emote_test_mode:
		emote_batch_auto_mode = true
		print("\n[AUTO MODE] Emote batch test will start automatically after avatar loads")
		print("[AUTO MODE] App will close when test completes\n")


func load_avatar_list():
	var file = FileAccess.open("res://src/test/avatar/avatar_list.json", FileAccess.READ)
	if file == null:
		printerr("the file does not exist")
		return

	var json_value = JSON.parse_string(file.get_as_text())
	if json_value == null or not json_value is Dictionary:
		printerr("the file has to be a valid json dictionary")
		return

	avatar_list = json_value.get("avatars", [])
	for avatar_i in avatar_list.size():
		option_button_avatar_list.add_item(avatar_list[avatar_i].get("ref", "no_ref"), avatar_i)


func download_wearable(id: String):
	var wearable = Global.content_provider.get_wearable(id)
	var dir_name = "user://downloaded/" + wearable.get_display_name().validate_filename()
	var content_mapping := wearable.get_content_mapping()

	DirAccess.make_dir_recursive_absolute(dir_name)

	for file_name in content_mapping.get_files():
		var file_hash = content_mapping.get_hash(file_name)
		var file_path = dir_name + "/" + file_name.validate_filename()
		if FileAccess.file_exists("user://content/" + file_hash):
			DirAccess.copy_absolute("user://content/" + file_hash, file_path)


func download_wearable_json(id: String):
	var wearable = Global.content_provider.get_wearable(id)
	return JSON.parse_string(wearable.to_json_string())


func download_wearables_avatar_json():
	var items = []
	items.push_back(download_wearable_json(avatar.avatar_data.get_body_shape()))
	for wearable_id in avatar.avatar_data.get_wearables():
		items.push_back(download_wearable_json(wearable_id))

	DisplayServer.clipboard_set(JSON.stringify({"wearables": items}, "\t"))


func download_avatar():
	download_wearable(avatar.avatar_data.get_body_shape())
	for wearable_id in avatar.avatar_data.get_wearables():
		download_wearable(wearable_id)


# gdlint:ignore = async-function-name
func _on_avatar_loaded():
	# Auto-start batch test in CLI mode
	if emote_batch_auto_mode and not emote_batch_test_running:
		# Small delay to ensure everything is ready
		await get_tree().create_timer(1.0).timeout
		_start_emote_batch_test()


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


# gdlint:ignore = async-function-name
func _on_option_button_avatar_list_item_selected(index):
	var avatar_i = option_button_avatar_list.get_item_id(index)

	_async_render_avatar(avatar_list[avatar_i])


func _async_render_avatar(avatar_dict: Dictionary) -> void:
	var profile: DclUserProfile = DclUserProfile.new()
	var avatar_wf: DclAvatarWireFormat = profile.get_avatar()

	avatar_wf.set_wearables(PackedStringArray(avatar_dict.wearables))
	avatar_wf.set_force_render(avatar_dict.get("forceRender", []))
	avatar_wf.set_body_shape(avatar_dict.bodyShape)

	var skin_color = avatar_dict.get("skin", {}).get("color", {})
	var eyes_color = avatar_dict.get("eye", {}).get("color", {})
	var hair_color = avatar_dict.get("hair", {}).get("color", {})

	skin_color = Color(skin_color.get("r", 0.8), skin_color.get("g", 0.8), skin_color.get("b", 0.8))
	eyes_color = Color(eyes_color.get("r", 0.8), eyes_color.get("g", 0.8), eyes_color.get("b", 0.8))
	hair_color = Color(hair_color.get("r", 0.8), hair_color.get("g", 0.8), hair_color.get("b", 0.8))

	avatar_wf.set_eyes_color(eyes_color)
	avatar_wf.set_hair_color(hair_color)
	avatar_wf.set_skin_color(skin_color)

	spinner.show()
	await avatar.async_update_avatar(avatar_wf, "")
	spinner.hide()


func _on_button_download_wearables_pressed():
	download_avatar()


func _on_button_copy_wearable_data_pressed():
	download_wearables_avatar_json()


func _on_button_refresh_pressed():
	_on_option_button_avatar_list_item_selected(option_button_avatar_list.selected)


# gdlint:ignore = async-function-name
func _on_button_fetch_pressed():
	var avatars_fetched = null
	spinner.show()

	if line_edit_profile_entity.text.begins_with("0x"):
		var address = line_edit_profile_entity.text
		var url = "https://peer.decentraland.org/lambdas/profiles/" + address
		var promise: Promise = Global.http_requester.request_json(
			url, HTTPClient.METHOD_GET, "", {}
		)
		var response = await PromiseUtils.async_awaiter(promise)

		if response is PromiseError:
			printerr("Error while fetching profile " + url, " reason: ", response.get_error())
			spinner.hide()
			return

		var json: Dictionary = response.get_string_response_as_json()
		avatars_fetched = json.get("avatars", [])
	elif line_edit_profile_entity.text.begins_with("bafk"):
		var url = "https://peer.decentraland.org/content/contents/" + line_edit_profile_entity.text
		var promise: Promise = Global.http_requester.request_json(
			url, HTTPClient.METHOD_GET, "", {}
		)
		var response = await PromiseUtils.async_awaiter(promise)

		if response is PromiseError:
			printerr("Error while fetching entity " + url, " reason: ", response.get_error())
			spinner.hide()
			return

		var json: Dictionary = response.get_string_response_as_json()
		avatars_fetched = json.get("metadata", {}).get("avatars", [])

	if avatars_fetched == null or avatars_fetched.is_empty():
		printerr("no avatars found")
		spinner.hide()
		return

	spinner.hide()
	_async_render_avatar(avatars_fetched[0].get("avatar", {}))


func _on_outline_toggled(button_pressed: bool):
	if sub_viewport_container:
		if button_pressed:
			sub_viewport_container.enable_outline()
		else:
			sub_viewport_container.disable_outline()


# ============================================================================
# EMOTE BATCH TESTER
# ============================================================================


func _setup_emote_batch_tester():
	var emotes_panel = $TabContainer/Emotes

	# Create batch test button
	batch_test_button = Button.new()
	batch_test_button.text = "Start Emote Batch Test"
	batch_test_button.set_anchors_preset(Control.PRESET_TOP_WIDE)
	batch_test_button.offset_top = 60
	batch_test_button.offset_bottom = 98
	batch_test_button.offset_left = 50
	batch_test_button.offset_right = -50
	batch_test_button.pressed.connect(_on_batch_test_button_pressed)
	emotes_panel.add_child(batch_test_button)

	# Create status label
	batch_test_label = Label.new()
	batch_test_label.text = "Batch test: Ready"
	batch_test_label.set_anchors_preset(Control.PRESET_TOP_WIDE)
	batch_test_label.offset_top = 105
	batch_test_label.offset_bottom = 135
	batch_test_label.offset_left = 50
	batch_test_label.offset_right = -50
	batch_test_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	emotes_panel.add_child(batch_test_label)

	# Create timer
	batch_test_timer = Timer.new()
	batch_test_timer.one_shot = true
	batch_test_timer.timeout.connect(_on_batch_test_timer_timeout)
	add_child(batch_test_timer)

	# Build emote list for testing
	emote_batch_test_list.clear()

	# Priority default emotes (test these first - common ones)
	var priority_emotes = ["raiseHand", "wave", "clap", "dance", "kiss", "handsair"]
	for emote_id in priority_emotes:
		emote_batch_test_list.append(Emotes.get_base_emote_urn(emote_id))

	# Rest of default emotes (embedded)
	for emote_id in Emotes.DEFAULT_EMOTE_NAMES.keys():
		var urn = Emotes.get_base_emote_urn(emote_id)
		if not emote_batch_test_list.has(urn):
			emote_batch_test_list.append(urn)

	# Utility emotes (embedded)
	for emote_id in Emotes.UTILITY_EMOTE_NAMES.keys():
		emote_batch_test_list.append(Emotes.get_base_emote_urn(emote_id))

	# Custom emotes from collections (downloaded - test these for foot issues!)
	# Festival 23 emotes (known to have foot issues)
	var custom_emotes = [
		"urn:decentraland:matic:collections-v2:0x8bfa4ffb139049f953fea3409bcc846decbef4b1:0",
		"urn:decentraland:matic:collections-v2:0x8bfa4ffb139049f953fea3409bcc846decbef4b1:1",
		"urn:decentraland:matic:collections-v2:0x8bfa4ffb139049f953fea3409bcc846decbef4b1:2",
		"urn:decentraland:matic:collections-v2:0x8bfa4ffb139049f953fea3409bcc846decbef4b1:3",
		"urn:decentraland:matic:collections-v2:0x8bfa4ffb139049f953fea3409bcc846decbef4b1:4",
	]
	emote_batch_test_list.append_array(custom_emotes)


func _on_batch_test_button_pressed():
	if emote_batch_test_running:
		_stop_emote_batch_test()
	else:
		_start_emote_batch_test()


func _start_emote_batch_test():
	emote_batch_test_running = true
	emote_batch_test_index = 0
	batch_test_button.text = "Stop Batch Test"
	print("\n========== EMOTE BATCH TEST STARTED ==========")
	print(
		(
			"Testing %d emotes with %.1fs delay between each"
			% [emote_batch_test_list.size(), emote_batch_test_delay]
		)
	)
	print("Watch for broken feet/animations!\n")
	_play_next_emote()


func _stop_emote_batch_test():
	emote_batch_test_running = false
	batch_test_timer.stop()
	batch_test_button.text = "Start Emote Batch Test"
	batch_test_label.text = "Batch test: Stopped"
	print("\n========== EMOTE BATCH TEST STOPPED ==========\n")


# gdlint:ignore = async-function-name
func _play_next_emote():
	if not emote_batch_test_running:
		return

	if emote_batch_test_index >= emote_batch_test_list.size():
		_stop_emote_batch_test()
		batch_test_label.text = "Batch test: COMPLETE!"
		print("\n========== EMOTE BATCH TEST COMPLETE ==========\n")

		# Quit app in auto mode
		if emote_batch_auto_mode:
			print("[AUTO MODE] Test complete. Closing app...")
			await get_tree().create_timer(1.0).timeout
			get_tree().quit(0)
		return

	var emote_urn = emote_batch_test_list[emote_batch_test_index]
	var emote_name = Emotes.get_emote_name(Emotes.get_base_emote_id_from_urn(emote_urn))

	batch_test_label.text = (
		"Testing [%d/%d]: %s"
		% [emote_batch_test_index + 1, emote_batch_test_list.size(), emote_name]
	)

	print(
		(
			"[%d/%d] Playing: %s (%s)"
			% [emote_batch_test_index + 1, emote_batch_test_list.size(), emote_name, emote_urn]
		)
	)

	avatar.emote_controller.async_play_emote(emote_urn)

	emote_batch_test_index += 1
	batch_test_timer.start(emote_batch_test_delay)


func _on_batch_test_timer_timeout():
	_play_next_emote()


# ============================================================================
# ANIMATION TESTER
# ============================================================================


func _refresh_animation_lists() -> void:
	option_button_library.clear()
	option_button_animation.clear()
	_anim_library_keys.clear()

	var player: AnimationPlayer = avatar.animation_player
	if player == null:
		return

	var libraries := player.get_animation_library_list()
	for lib_name in libraries:
		var display := "<default>" if String(lib_name).is_empty() else String(lib_name)
		option_button_library.add_item(display)
		_anim_library_keys.append(String(lib_name))

	if option_button_library.item_count > 0:
		option_button_library.select(0)
		_refresh_animation_options_for_library(0)


func _refresh_animation_options_for_library(lib_idx: int) -> void:
	option_button_animation.clear()
	var player: AnimationPlayer = avatar.animation_player
	if player == null or lib_idx < 0 or lib_idx >= _anim_library_keys.size():
		return
	var lib_name: String = _anim_library_keys[lib_idx]
	var lib := player.get_animation_library(lib_name)
	if lib == null:
		return
	var clips := lib.get_animation_list()
	for clip in clips:
		option_button_animation.add_item(String(clip))
	if option_button_animation.item_count > 0:
		option_button_animation.select(0)


func _selected_animation_path() -> String:
	var lib_idx := option_button_library.selected
	var clip_idx := option_button_animation.selected
	if lib_idx < 0 or clip_idx < 0:
		return ""
	var lib_name: String = _anim_library_keys[lib_idx]
	var clip_name := option_button_animation.get_item_text(clip_idx)
	if lib_name.is_empty():
		return clip_name
	return "%s/%s" % [lib_name, clip_name]


func _suspend_state_machine() -> void:
	# Stop avatar.gd's _process from re-enabling the tree and overwriting conditions.
	avatar.set_process(false)
	avatar.animation_tree.active = false


# gdlint:ignore = async-function-name
func _play_selected_animation(loop: bool) -> void:
	var anim_path := _selected_animation_path()
	if anim_path.is_empty():
		label_now_playing.text = "Now playing: (no selection)"
		return
	var player: AnimationPlayer = avatar.animation_player
	if not player.has_animation(anim_path):
		label_now_playing.text = "Now playing: MISSING %s" % anim_path
		return

	_suspend_state_machine()

	var anim := player.get_animation(anim_path)
	var prev_loop_mode := anim.loop_mode
	if loop:
		anim.loop_mode = Animation.LOOP_LINEAR
	player.play(anim_path)
	label_now_playing.text = "Now playing: %s%s" % [anim_path, " (looping)" if loop else ""]
	# Restore loop mode after the clip finishes so we don't mutate the shared resource.
	if loop:
		# Defer restore to when animation actually stops.
		await player.animation_finished
		anim.loop_mode = prev_loop_mode


func _on_anim_library_selected(index: int) -> void:
	_refresh_animation_options_for_library(index)


func _on_anim_refresh_pressed() -> void:
	_refresh_animation_lists()


func _on_anim_play_pressed() -> void:
	_play_selected_animation(false)


func _on_anim_play_loop_pressed() -> void:
	_play_selected_animation(true)


func _on_anim_stop_pressed() -> void:
	avatar.animation_player.stop()
	label_now_playing.text = "Now playing: —"


func _on_anim_restore_tree_pressed() -> void:
	avatar.animation_tree.active = true
	avatar.set_process(true)
	label_now_playing.text = "Now playing: (state machine)"
	# Reset flags so the tree returns to idle.
	check_walk.button_pressed = false
	check_jog.button_pressed = false
	check_run.button_pressed = false
	check_rise.button_pressed = false
	check_fall.button_pressed = false
	check_land.button_pressed = false


func _on_flag_walk_toggled(pressed: bool) -> void:
	avatar.walk = pressed


func _on_flag_jog_toggled(pressed: bool) -> void:
	avatar.jog = pressed


func _on_flag_run_toggled(pressed: bool) -> void:
	avatar.run = pressed


func _on_flag_rise_toggled(pressed: bool) -> void:
	avatar.rise = pressed


func _on_flag_fall_toggled(pressed: bool) -> void:
	avatar.fall = pressed


func _on_flag_land_toggled(pressed: bool) -> void:
	avatar.land = pressed


# ---------------------------------------------------------------------------
# Glider prop
# ---------------------------------------------------------------------------


func _populate_glider_anim_list() -> void:
	option_button_glider_anim.clear()
	# Hardcoded — the library is known and small. Keeps the tester independent
	# of whether the prop scene has been instantiated yet.
	var clips := [
		"Glider_Idle",
		"Glider_Open",
		"Glider_Close",
		"Glider_Start",
		"Glider_Forward",
		"Glider_TurnLeft",
		"Glider_TurnRight",
		"Glider_End",
	]
	for clip in clips:
		option_button_glider_anim.add_item(clip)
	option_button_glider_anim.select(0)


func _on_show_glider_toggled(pressed: bool) -> void:
	if pressed:
		if _current_glider_prop != null:
			return
		_current_glider_prop = GLIDER_PROP_SCENE.instantiate()
		avatar.add_child(_current_glider_prop)
	else:
		if _current_glider_prop != null:
			_current_glider_prop.queue_free()
			_current_glider_prop = null


func _on_play_glider_pressed() -> void:
	if _current_glider_prop == null:
		return
	var clip := option_button_glider_anim.get_item_text(option_button_glider_anim.selected)
	var player := _current_glider_prop.get_node_or_null("AnimationPlayer") as AnimationPlayer
	if player == null:
		return
	if player.has_animation(clip):
		player.play(clip)
