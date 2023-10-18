extends DclAudioStream


func stream_buffer(data: PackedVector2Array):
	if not self.playing:
		self.play()

	self.get_stream_playback().push_buffer(data)


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
