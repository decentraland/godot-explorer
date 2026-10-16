class_name EmoteAvatarPreview
extends Control

## The player's avatar looping an emote, framed on every pose of the emote: the standing-body
## framing of AvatarPreview leaves a sitting or crouching emote small at the bottom.
##
## Stays transparent until it is framed and playing, so whatever sits behind it shows when the
## avatar or the emote cannot be loaded.

# Poses measured across the emote; their union is what the camera frames.
const FIT_SAMPLES := 8
const FADE_SECONDS := 0.3

## Faded out (its own drawing only, not its children) once the avatar shows: the art this
## preview replaces, kept as the fallback.
@export var fallback: CanvasItem = null
## Camera angle around the emote, in degrees: to the viewer's right, and up.
@export var camera_yaw_degrees := 25.0
@export var camera_pitch_degrees := 20.0

@onready var _preview: AvatarPreview = %AvatarPreview


func async_play(emote_urn: String) -> void:
	var profile: DclUserProfile = Global.player_identity.get_profile_or_null()
	if profile == null:
		return
	var avatar: Avatar = _preview.avatar
	await avatar.async_update_avatar_from_profile(profile)
	if not NodeGuard.is_alive(self, "EmoteAvatarPreview.async_play"):
		return
	var emotes: AvatarEmoteController = avatar.emote_controller
	# Raised and turned to the viewer's right, looking at the emote's centre.
	var view := Basis.from_euler(
		Vector3(deg_to_rad(camera_pitch_degrees), deg_to_rad(camera_yaw_degrees), 0.0)
	)
	var frame := AABB()
	for i in FIT_SAMPLES:
		var posed: bool = await emotes.async_freeze_on_emote(emote_urn, float(i) / FIT_SAMPLES)
		if not NodeGuard.is_alive(self, "EmoteAvatarPreview.async_play") or not posed:
			return
		await get_tree().process_frame
		if not NodeGuard.is_alive(self, "EmoteAvatarPreview.async_play"):
			return
		var pose := _preview.compute_current_fit_aabb(view)
		frame = pose if i == 0 else frame.merge(pose)
	# Posing disables the animation tree; playing only sets it active.
	avatar.animation_tree.process_mode = Node.PROCESS_MODE_INHERIT
	_preview.fit_to_aabb(frame, view)
	emotes.play_emote(emote_urn)
	var tween := create_tween().set_parallel()
	tween.tween_property(_preview, "modulate:a", 1.0, FADE_SECONDS)
	if is_instance_valid(fallback):
		tween.tween_property(fallback, "self_modulate:a", 0.0, FADE_SECONDS)
