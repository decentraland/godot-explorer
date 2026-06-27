extends VoiceChatRecorder

var is_recording = false

var is_enabled = false


func _ready():
	if ProjectSettings.get_setting_with_override("audio/driver/enable_input"):
		Services.comms.on_adapter_changed.connect(self._on_adapter_changed)
		audio.connect(Services.comms.broadcast_voice)
		setup_audio_server()


func set_recording_enabled_with_sfx(enabled: bool):
	Services.ui_sounds.play_sound("voice_chat_mic_on" if enabled else "voice_chat_mic_off")
	set_recording_enabled(enabled)


func _on_adapter_changed(voice_chat_enabled, _adapter_str):
	is_enabled = voice_chat_enabled
	set_recording_enabled(false)


func _physics_process(_delta):
	if is_enabled:
		if is_recording != Input.is_action_pressed("ia_record_mic"):
			is_recording = Input.is_action_pressed("ia_record_mic")
			set_recording_enabled_with_sfx(is_recording)
