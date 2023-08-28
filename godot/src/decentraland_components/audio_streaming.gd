extends AudioStreamPlayer


func stream_buffer(data: PackedVector2Array):
	if not self.playing:
		self.play()

	self.get_stream_playback().push_buffer(data)


# 	print(data.length())


func init(frame_rate, frames, length, format, bit_rate, frame_size, channels):
	pass
	print(
		"audio_streaming debug init ",
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
