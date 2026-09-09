extends DclAudioSource

## Linear volume units per second, matching Unity's `AudioSourcesPlugin.FadeSpeed`.
const FADE_SPEED := 1.0
const MIN_VOLUME_DB := -80.0
## Distance where attenuation starts. Unity leaves `minDistance` at its default of
## 1 m; Godot's default `unit_size` of 10 is +20 dB louder past a metre.
const UNIT_SIZE := 1.0
## Top of Godot's `attenuation_filter_cutoff_hz` range: the high-shelf ends up
## above the audible band, which is how the filter is switched off.
const FILTER_DISABLED_HZ := 20500.0

# Clip states, matching the Rust CLIP_STATE_* constants.
const CLIP_STATE_NONE = 0
const CLIP_STATE_LOADING = 1
const CLIP_STATE_READY = 2
const CLIP_STATE_ERROR = 3

var last_loaded_audio_clip := ""
var valid := false
var _time_specified := false
## Faded gain, ramping toward `dcl_volume` inside the scene and toward 0 outside.
var _faded_volume := 0.0
var _fade_initialized := false
var _last_sdk_volume := -1.0


func apply_audio_props(action_on_playing: bool):
	_sync_volume_from_sdk()

	if not valid:
		return

	self.pitch_scale = dcl_pitch

	if dcl_global:
		attenuation_model = AudioStreamPlayer3D.ATTENUATION_DISABLED
	else:
		attenuation_model = AudioStreamPlayer3D.ATTENUATION_INVERSE_DISTANCE

	self.unit_size = UNIT_SIZE
	# Godot's `max_distance` hard-mutes past the limit and adds a linear ramp
	# across the whole range, so it can't stand in for Unity's `maxDistance`.
	self.max_distance = 0.0
	# Godot ties a high-shelf to the attenuation multiplier, so a source at 5 m
	# would lose 19 dB above 5 kHz on top of the gain. Unity's rolloff is
	# gain-only, so push the filter above hearing.
	self.attenuation_filter_cutoff_hz = FILTER_DISABLED_HZ

	if action_on_playing:
		if self.playing and not dcl_playing:
			self.stop()
		elif dcl_playing and (not self.playing or _time_specified):
			self.play(dcl_current_time)


## An SDK `volume` change snaps; only entering/leaving the scene crossfades.
func _sync_volume_from_sdk() -> void:
	if not _fade_initialized:
		_fade_initialized = true
		# Spawned outside the scene means silent, so entering fades in instead of
		# blasting a neighbouring scene's first frames (unity-explorer#4790).
		_faded_volume = dcl_volume if dcl_enable else 0.0
	elif dcl_enable and not is_equal_approx(dcl_volume, _last_sdk_volume):
		_faded_volume = dcl_volume

	_last_sdk_volume = dcl_volume
	_apply_volume()


func _process(delta: float) -> void:
	var target: float = dcl_volume if dcl_enable else 0.0
	if is_equal_approx(_faded_volume, target):
		return

	if _faded_volume < target:
		_faded_volume = minf(target, _faded_volume + delta * FADE_SPEED)
	else:
		_faded_volume = maxf(target, _faded_volume - delta * FADE_SPEED)

	_apply_volume()


func _apply_volume() -> void:
	var gain_db: float = (
		MIN_VOLUME_DB if _faded_volume <= 0.0 else maxf(MIN_VOLUME_DB, linear_to_db(_faded_volume))
	)
	self.volume_db = gain_db
	# Godot clamps `attenuation + volume_db` to `max_db`. Clamping at 0 would drop
	# the SDK volume wherever attenuation is positive; clamping at `volume_db`
	# gives `volume * min(1, unit_size / distance)`, which is Unity's rolloff.
	self.max_db = gain_db


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
			dcl_clip_state = (
				CLIP_STATE_NONE if last_loaded_audio_clip.is_empty() else CLIP_STATE_ERROR
			)
			return

		dcl_clip_state = CLIP_STATE_LOADING
		var promise: Promise = Global.content_provider.fetch_audio(
			last_loaded_audio_clip, content_mapping
		)
		var res = await PromiseUtils.async_awaiter(promise)
		if res is PromiseError:
			self.stop()
			self.stream = null
			dcl_clip_state = CLIP_STATE_ERROR
			printerr("Error on fetch audio: ", res.get_error())
		else:
			_on_audio_loaded(res)


func _on_audio_loaded(audio_stream):
	# A resolved promise carrying no stream is still a failure: it used to pass
	# as READY and leave a silent player behind (#2741).
	if not audio_stream is AudioStream:
		self.stop()
		self.stream = null
		valid = false
		dcl_clip_state = CLIP_STATE_ERROR
		printerr("Audio clip resolved with no stream: ", last_loaded_audio_clip)
		return

	self.stream = audio_stream
	valid = true
	dcl_clip_state = CLIP_STATE_READY

	apply_audio_props(true)


func _on_finished():
	if dcl_loop_activated:
		play()
