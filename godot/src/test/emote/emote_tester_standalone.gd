extends Control

# Emote batch testing
var emote_test_running: bool = false
var emote_test_index: int = 0
var emote_test_list: Array = []
var emote_test_delay: float = 3  # seconds between each emote
var auto_mode: bool = false

@onready var avatar_preview: AvatarPreview = $AvatarPreview
@onready var avatar: Avatar = $AvatarPreview.avatar
@onready var status_label: Label = $StatusLabel
@onready var emote_name_label: Label = $EmoteNameLabel
@onready var start_button: Button = $StartButton
@onready var timer: Timer = $Timer


func _ready():
	# Build emote list
	_build_emote_list()

	# Setup timer
	timer.one_shot = true
	timer.timeout.connect(_on_timer_timeout)

	# Setup button
	start_button.pressed.connect(_on_start_button_pressed)

	# Wait for avatar to load
	avatar.avatar_loaded.connect(_on_avatar_loaded)

	# Check for auto mode
	if Global.cli.emote_test_mode:
		auto_mode = true
		status_label.text = "[AUTO] Waiting for avatar..."
		print("\n========== EMOTE TESTER - AUTO MODE ==========")
		print("Will cycle through %d emotes and exit\n" % emote_test_list.size())

	# Load default avatar
	_load_default_avatar()


func _load_default_avatar():
	status_label.text = "Loading avatar..."

	# Set default profile and load avatar
	Global.player_identity.set_default_profile()
	var profile: DclUserProfile = Global.player_identity.get_profile_or_null()
	if profile != null:
		avatar.async_update_avatar_from_profile(profile)
	else:
		printerr("Failed to get default profile")
		status_label.text = "ERROR: No profile"


func _build_emote_list():
	emote_test_list.clear()

	# Priority default emotes (test these first)
	var priority_emotes = ["raiseHand", "wave", "clap", "dance", "kiss", "handsair"]
	for emote_id in priority_emotes:
		emote_test_list.append(Emotes.get_base_emote_urn(emote_id))

	# Rest of default emotes (NOT utility/action emotes)
	for emote_id in Emotes.DEFAULT_EMOTE_NAMES.keys():
		var urn = Emotes.get_base_emote_urn(emote_id)
		if not emote_test_list.has(urn):
			emote_test_list.append(urn)

	# Custom/remote emotes (these are known to have foot issues)
	var custom_emotes = [
		"urn:decentraland:matic:collections-v2:0x0b472c2c04325a545a43370b54e93c87f3d5badf:1:105312291668557186697918027683670432318895095400549111254310978331",
		"urn:decentraland:matic:collections-v2:0xb13c91d7288e2d4328dd153fc0c6d29fad6159f7:0:147",
		"urn:decentraland:matic:collections-v2:0xfbc9b2cff58dcc29dab28e2af7eac80c9012fe02:0:39",
	]
	emote_test_list.append_array(custom_emotes)


# gdlint:ignore = async-function-name
func _on_avatar_loaded():
	status_label.text = "Avatar loaded. Ready to test."
	if auto_mode:
		# Small delay then start
		await get_tree().create_timer(1.0).timeout
		_start_test()


func _on_start_button_pressed():
	if emote_test_running:
		_stop_test()
	else:
		_start_test()


func _start_test():
	emote_test_running = true
	emote_test_index = 0
	start_button.text = "Stop"
	print("\n========== EMOTE TEST STARTED ==========")
	print("Testing %d emotes (%.1fs each)\n" % [emote_test_list.size(), emote_test_delay])
	_play_next_emote()


func _stop_test():
	emote_test_running = false
	timer.stop()
	start_button.text = "Start Test"
	status_label.text = "Test stopped"
	emote_name_label.text = ""
	print("\n========== EMOTE TEST STOPPED ==========\n")


func _play_next_emote():
	if not emote_test_running:
		return

	if emote_test_index >= emote_test_list.size():
		_on_test_complete()
		return

	var emote_urn = emote_test_list[emote_test_index]
	var emote_id = Emotes.get_base_emote_id_from_urn(emote_urn)
	var emote_name = Emotes.get_emote_name(emote_id)

	# For custom emotes, show URN
	if emote_name == emote_id and not Emotes.is_emote_embedded(emote_id):
		emote_name = "Custom: " + emote_urn.get_slice(":", -1)

	status_label.text = "[%d/%d]" % [emote_test_index + 1, emote_test_list.size()]
	emote_name_label.text = emote_name

	print("[%d/%d] %s" % [emote_test_index + 1, emote_test_list.size(), emote_name])

	avatar.emote_controller.async_play_emote(emote_urn)

	emote_test_index += 1
	timer.start(emote_test_delay)


func _on_timer_timeout():
	_play_next_emote()


# gdlint:ignore = async-function-name
func _on_test_complete():
	emote_test_running = false
	start_button.text = "Start Test"
	status_label.text = "TEST COMPLETE!"
	emote_name_label.text = ""
	print("\n========== EMOTE TEST COMPLETE ==========\n")

	if auto_mode:
		print("[AUTO] Exiting...")
		await get_tree().create_timer(1.0).timeout
		get_tree().quit(0)
