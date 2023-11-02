extends DclVideoPlayer

func stream_buffer(data: PackedVector2Array):
	if not self.playing:
		self.play()

	self.get_stream_playback().push_buffer(data)


func request_video(file_hash):
	var content_mapping = Global.scene_runner.get_scene_content_mapping(dcl_scene_id)

	var request_state = Global.content_manager.fetch_video(file_hash, content_mapping)
	if request_state != null:
		await request_state.on_finish

	_on_video_loaded(file_hash)

func _on_video_loaded(file_hash: String):
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
