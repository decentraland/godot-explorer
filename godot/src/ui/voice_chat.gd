extends Control

var recording: bool = false
var _effect_capture: AudioEffectCapture
var _prev_frame_recording = false

@onready var button_record = $Button
@onready var audio_stream_player = $AudioStreamPlayer


func _ready():
	var devices = AudioServer.get_input_device_list()
	print(AudioServer.get_input_device_list())
#	AudioServer.input_device = devices[1]

	var idx = AudioServer.get_bus_index("Capture")
	_effect_capture = AudioServer.get_bus_effect(idx, 0)

	audio_stream_player.stream = AudioStreamMicrophone.new()
	audio_stream_player.bus = "Capture"


func _process(delta):
	if recording:
		var stereo_data: PackedVector2Array = _effect_capture.get_buffer(
			_effect_capture.get_frames_available()
		)
		if stereo_data.size() > 0:
			Global.comms.broadcast_voice(stereo_data)

	_prev_frame_recording = recording


func _on_button_pressed():
	if recording:
		button_record.text = "Enable mic"
		recording = false
		_effect_capture.clear_buffer()
		audio_stream_player.stop()
	else:
		button_record.text = "Stop mic"
		recording = true
		audio_stream_player.play()
		_effect_capture.clear_buffer()
