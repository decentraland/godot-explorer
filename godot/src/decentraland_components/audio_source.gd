extends DclAudioSource

var last_loaded_audio_clip := ""
var audio_clip_file_hash := ""
var valid := false


func apply_audio_props(action_on_playing: bool):
	if not valid:
		return

	self.pitch_scale = dcl_pitch

	if not dcl_enable:
		self.volume_db = -80
	else:
		# TODO: Check if it should be 10 instead 20 to talk in terms of power
		self.volume_db = 20 * log(dcl_volume)
		# -80 = 20 log 0.00001, so muted is when (volume <= 0.00001)

	if action_on_playing:
		if self.playing and not dcl_playing:
			self.stop()
		elif not self.playing and dcl_playing:
			self.play()


func _refresh_data():
	dcl_audio_clip_url = dcl_audio_clip_url.to_lower()

	if last_loaded_audio_clip == dcl_audio_clip_url:
		apply_audio_props(true)
	else:
		var content_mapping = Global.scene_runner.get_scene_content_mapping(dcl_scene_id)

		last_loaded_audio_clip = dcl_audio_clip_url
		valid = false

		audio_clip_file_hash = content_mapping.get("content", {}).get(last_loaded_audio_clip, "")
		if audio_clip_file_hash.is_empty():
			# TODO: log file not found
			return

		var fetching_resource = Global.content_manager.fetch_audio(
			last_loaded_audio_clip, content_mapping
		)
		if not fetching_resource:
			self._on_audio_loaded.call_deferred(audio_clip_file_hash)
		else:
			Global.content_manager.content_loading_finished.connect(
				self._content_manager_resource_loaded
			)


func _content_manager_resource_loaded(resource_hash: String):
#	Global.content_manager.content_loading_finished.disconnect(
#		self._content_manager_resource_loaded
#	)
	_on_audio_loaded(resource_hash)


func _on_audio_loaded(file_hash: String):
	if file_hash != audio_clip_file_hash:
		return

	var audio_stream = Global.content_manager.get_resource_from_hash(file_hash)
	if audio_stream == null:
		self.stop()
		self.stream = null
		return

	self.stream = audio_stream
	valid = true

	apply_audio_props(true)