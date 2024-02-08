extends VoiceChatRecorder

var is_recording = false


func _ready():
	audio.connect(Global.comms.broadcast_voice)
	setup_audio_server()


func _physics_process(_delta):
	if is_recording != Input.is_action_pressed("ia_record_mic"):
		is_recording = Input.is_action_pressed("ia_record_mic")
		set_recording_enabled(is_recording)
