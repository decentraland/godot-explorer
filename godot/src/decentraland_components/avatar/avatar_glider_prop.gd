class_name AvatarGliderProp
extends RefCounted

# The glider prop: Unity's wings — visibility, the Start/Idle/Forward/End clip
# sequence and the open/close/idle audio, driven by the avatar's glide_state
# edges. The prop is a persistent child (rotated 180° Y to compensate the
# Unity→Godot axis flip), so its audio players and AnimationPlayer stay warm
# across glide cycles.

var _avatar: Node3D
# glide_state last frame, for the edge detection.
var _last_state: int = 0
# The first call adopts whatever state came in instead of firing the edges, so a
# remote first seen mid-glide shows wings and the idle loop.
var _sync_pending: bool = true
# Set once the close clip has been started, so nothing schedules the hide twice.
var _close_initiated: bool = false


func _init(avatar: Node3D) -> void:
	_avatar = avatar


func update() -> void:
	var prop: Node3D = _avatar.glider_prop
	var curr_state: int = _avatar.glide_state
	if _avatar.is_avatar_shape:
		if prop.visible:
			prop.visible = false
		_last_state = curr_state
		_sync_pending = false
		return

	if _sync_pending:
		_sync_pending = false
		_last_state = curr_state
		if curr_state == 1 or curr_state == 2:
			# Joined into a glide: wings up, idle loop, no Start/Open sfx.
			prop.visible = true
			_close_initiated = false
			_play_clip_if_different(prop, "Glider_Idle")
			_play_audio(prop, "AudioPlayer_Idle")
		elif curr_state == 3:
			# Joined during CLOSING: show the prop and let the next tick's
			# CLOSING→CLOSED edge hide it.
			prop.visible = true
			_close_initiated = true
		return

	var prev_state: int = _last_state
	_last_state = curr_state

	if prev_state == 0 and curr_state == 1:
		prop.visible = true
		_close_initiated = false
		_play_audio(prop, "AudioPlayer_Open")
		_play_audio(prop, "AudioPlayer_Idle")
		_play_clip(prop, "Glider_Start")
	elif (prev_state == 1 or prev_state == 2) and curr_state == 3:
		# Start the close on the CLOSING edge: waiting for the FSM to reach
		# CLOSED (0.15s later) leaves the wings open through the close.
		_close_initiated = true
		_stop_audio(prop, "AudioPlayer_Idle")
		_play_audio(prop, "AudioPlayer_Close")
		_play_clip(prop, "Glider_End")
	elif prev_state == 3 and curr_state == 0 and prop.visible:
		_schedule_hide()
	elif curr_state == 0 and prop.visible and not _close_initiated:
		# Fallback: state jumped straight to CLOSED without passing CLOSING.
		_close_initiated = true
		_stop_audio(prop, "AudioPlayer_Idle")
		_play_audio(prop, "AudioPlayer_Close")
		_play_clip(prop, "Glider_End")
		_schedule_hide()

	if curr_state == 2 and prop.visible:
		var moving: bool = _avatar.walk or _avatar.jog or _avatar.run
		_play_clip_if_different(prop, "Glider_Forward" if moving else "Glider_Idle", 0.25)


func _schedule_hide() -> void:
	var tree := _avatar.get_tree()
	if tree == null:
		_hide()
		return
	tree.create_timer(0.25).timeout.connect(_hide)


func _hide() -> void:
	# Skipped when the glide was re-opened inside the fade-out window.
	if _avatar.glide_state == 0:
		_avatar.glider_prop.visible = false


# The three glider players keep `max_distance = 50` for the same reason as
# `avatar_sfxs.gd`: a cheap per-avatar cutoff, traded against Unity-exact
# rolloff. See the comment there.
func _play_audio(prop: Node3D, node_name: String) -> void:
	var player := prop.get_node_or_null(node_name) as AudioStreamPlayer3D
	if player != null:
		player.play()


func _stop_audio(prop: Node3D, node_name: String) -> void:
	var player := prop.get_node_or_null(node_name) as AudioStreamPlayer3D
	if player != null:
		player.stop()


func _play_clip(prop: Node3D, clip: String) -> void:
	var ap := prop.get_node_or_null("AnimationPlayer") as AnimationPlayer
	if ap == null or not ap.has_animation(clip):
		return
	ap.play(clip)


func _play_clip_if_different(prop: Node3D, clip: String, blend_time: float = -1.0) -> void:
	var ap := prop.get_node_or_null("AnimationPlayer") as AnimationPlayer
	if ap == null or not ap.has_animation(clip):
		return
	if ap.current_animation != clip:
		ap.play(clip, blend_time)
