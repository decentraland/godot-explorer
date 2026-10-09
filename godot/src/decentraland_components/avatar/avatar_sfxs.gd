extends Node3D

const WALK_SOUNDS = [
	preload("res://assets/sfx/avatar/avatar_footstep_walk01.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_walk02.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_walk03.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_walk04.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_walk05.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_walk06.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_walk07.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_walk08.wav"),
]

const JOG_SOUNDS = [
	preload("res://assets/sfx/avatar/avatar_footstep_light01.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_light02.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_light03.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_light04.wav"),
]

const RUN_SOUNDS = [
	preload("res://assets/sfx/avatar/avatar_footstep_run01.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_run02.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_run03.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_run04.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_run05.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_run06.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_run07.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_run08.wav"),
]

const JUMP_SOUNDS = [
	preload("res://assets/sfx/avatar/avatar_footstep_jump01.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_jump02.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_jump03.wav"),
]

const LAND_SOUNDS = [
	preload("res://assets/sfx/avatar/avatar_footstep_land01.wav"),
	preload("res://assets/sfx/avatar/avatar_footstep_land02.wav"),
]

const WALK_INTERVAL = 500
const JOG_INTERVAL = 350
const RUN_INTERVAL = 300
# Past max_distance (+ margin for movement during one recheck) nothing is audible, so polling
# sleeps and only rechecks the camera (the listener) distance.
const AUDIBLE_MARGIN := 10.0
const RANGE_RECHECK_MS := 500

var next_tick = 0

var walk_index: int = 0
var jog_index: int = 0
var run_index: int = 0

var last_walk: bool = false
var last_run: bool = false
var last_jog: bool = false
var last_rise: bool = false
var last_fall: bool = false
var last_land: bool = false
var _next_range_check: int = 0

# Both players keep `max_distance = 50` where `audio_source.gd` sets it to 0.
# Deliberate, not an oversight: Godot's `max_distance` adds a linear ramp on
# top of the 1/d rolloff (audio_stream_player_3d.cpp:446), so this is not
# Unity-exact — about -1.9 dB at 10 m and -6 dB at 25 m against the reference —
# but it stops the player past 50 m instead of keeping one voice per avatar in
# the mix forever at a level nobody can hear. SDK sources are single and
# scene-authored, so they take the exact curve; avatars are many.
@onready var audio_player_steps: AudioStreamPlayer3D = $AudioPlayer_Steps
@onready var audio_player_effects: AudioStreamPlayer3D = $AudioPlayer_Effects
@onready var avatar: Avatar = get_parent()


func _ready():
	last_land = avatar.land


func _process(_delta):
	if avatar == null or not is_instance_valid(avatar) or avatar.blocked:
		return
	var current_time = Time.get_ticks_msec()
	if current_time >= _next_range_check:
		_next_range_check = current_time + RANGE_RECHECK_MS
		if not _in_audible_range():
			set_process(false)
			_schedule_range_recheck()
			return
	var land: bool = avatar.land
	if !audio_player_steps.is_playing() and land and current_time > next_tick:
		if avatar.run:
			audio_player_steps.stream = RUN_SOUNDS[run_index]
			audio_player_steps.play()
			run_index = run_index + 1 if run_index < RUN_SOUNDS.size() - 1 else 0
			next_tick = current_time + RUN_INTERVAL
		elif avatar.jog:
			audio_player_steps.stream = JOG_SOUNDS[jog_index]
			audio_player_steps.play()
			jog_index = jog_index + 1 if jog_index < JOG_SOUNDS.size() - 1 else 0
			next_tick = current_time + JOG_INTERVAL
		elif avatar.walk:
			audio_player_steps.stream = WALK_SOUNDS[walk_index]
			audio_player_steps.play()
			walk_index = walk_index + 1 if walk_index < WALK_SOUNDS.size() - 1 else 0
			next_tick = current_time + WALK_INTERVAL

	if last_land != land:
		# Start/stop land
		last_land = land
		if last_rise == false:  # This sould be trigger on jumps
			audio_player_effects.stream = JUMP_SOUNDS.pick_random()
			audio_player_effects.play()
		else:  # This sould be trigger when it landed...
			audio_player_effects.stream = LAND_SOUNDS.pick_random()
			audio_player_effects.play()


func _in_audible_range() -> bool:
	var camera := get_viewport().get_camera_3d()
	if camera == null:
		return true
	var max_dist: float = audio_player_steps.max_distance + AUDIBLE_MARGIN
	return (
		camera.global_position.distance_squared_to(audio_player_steps.global_position)
		< max_dist * max_dist
	)


func _schedule_range_recheck() -> void:
	get_tree().create_timer(RANGE_RECHECK_MS / 1000.0).timeout.connect(_on_range_recheck)


func _on_range_recheck() -> void:
	if not is_inside_tree():
		set_process(true)
		return
	if not _in_audible_range():
		_schedule_range_recheck()
		return
	# Landing changes while asleep were inaudible; don't replay them on wake.
	if is_instance_valid(avatar):
		last_land = avatar.land
	set_process(true)
