extends DclVideoPlayer

var file_hash: String = ""

func stream_buffer(data: PackedVector2Array):
	if not self.playing:
		self.play()

	self.get_stream_playback().push_buffer(data)


func set_mute(value: bool):
	if value:
		self.volume_db = 0
	else:
		self.volume_db = -80

func request_video(_file_hash):
	var content_mapping = Global.scene_runner.get_scene_content_mapping(dcl_scene_id)
	self.file_hash = _file_hash

	var fetching_resource = Global.content_manager.fetch_video(file_hash, content_mapping)
	if not fetching_resource:
		self._on_video_loaded(self.file_hash)
	else:
		Global.content_manager.content_loading_finished.connect(
			self._content_manager_resource_loaded
		)

func _content_manager_resource_loaded(resource_hash: String):
	_on_video_loaded(resource_hash, true)

func _on_video_loaded(resource_hash: String, from_signal: bool = false):
	if resource_hash != file_hash:
		return

	if from_signal:
		Global.content_manager.content_loading_finished.disconnect(
			self._content_manager_resource_loaded
		)
		
	var local_video_path = "user://content/" + file_hash
	var absolute_file_path = local_video_path.replace("user:/", OS.get_user_data_dir())
	self.resolve_resource(absolute_file_path)

func init_audio(frame_rate, frames, length, format, bit_rate, frame_size, channels):
	print(
		"audio_stream debug init ",
		frame_rate,
		" - ",
		frames,
		" - ",
		length,
		" - ",
		format,
		" - ",
		bit_rate,
		" - ",
		frame_size,
		" - ",
		channels
	)
