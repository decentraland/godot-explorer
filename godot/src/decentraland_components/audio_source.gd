extends DclAudioSource

var last_loaded_audio_clip := ""
var valid := false
var _time_specified := false


func apply_audio_props(action_on_playing: bool):
	if not valid:
		return

	self.pitch_scale = dcl_pitch

	if dcl_global:
		attenuation_model = AudioStreamPlayer3D.ATTENUATION_DISABLED
	else:
		attenuation_model = AudioStreamPlayer3D.ATTENUATION_INVERSE_DISTANCE

	if not dcl_enable:
		self.volume_db = -80
	else:
		# TODO: Check if it should be 10 instead 20 to talk in terms of power
		self.volume_db = 20 * log(dcl_volume)
		# -80 = 20 log 0.00001, so muted is when (volume <= 0.00001)

	if action_on_playing:
		if self.playing and not dcl_playing:
			self.stop()
		elif dcl_playing and (not self.playing or _time_specified):
			self.play(dcl_current_time)


func _async_refresh_data(time_specified: bool):
	dcl_audio_clip_url = dcl_audio_clip_url.to_lower()
	_time_specified = time_specified

	if last_loaded_audio_clip == dcl_audio_clip_url:
		apply_audio_props(true)
	else:
		var content_mapping := Global.scene_runner.get_scene_content_mapping(dcl_scene_id)

		last_loaded_audio_clip = dcl_audio_clip_url
		valid = false

		var audio_clip_file_hash = content_mapping.get_hash(last_loaded_audio_clip)
		if audio_clip_file_hash.is_empty():
			# TODO: log file not found
			return

		var promise: Promise = Global.content_provider.fetch_audio(
			last_loaded_audio_clip, content_mapping
		)
		var res = await PromiseUtils.async_awaiter(promise)
		if res is PromiseError:
			self.stop()
			self.stream = null
			printerr("Error on fetch audio: ", res.get_error())
		else:
			_on_audio_loaded(res)


func _on_audio_loaded(audio_stream):
	self.stream = audio_stream
	valid = true

	apply_audio_props(true)


func _on_finished():
	if dcl_loop_activated:
		play()
