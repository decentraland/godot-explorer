class_name Snapshot
extends RefCounted

signal snapshot_generated(face_image: Image)

const AVATAR_PREVIEW_SCENE = preload("res://src/ui/components/backpack/avatar_preview.tscn")


# ADR-290: Generate snapshots locally for immediate display in the UI.
# These are NOT uploaded to the server - they're only stored locally.
# The profile-images service generates snapshots on-demand for other users.
# gdlint:ignore = async-function-name
func async_generate_for_avatar(
	avatar_wire_format: DclAvatarWireFormat, profile: DclUserProfile
) -> void:
	var avatar_preview: AvatarPreview = AVATAR_PREVIEW_SCENE.instantiate()
	avatar_preview.show_platform = false
	avatar_preview.hide_name = true
	avatar_preview.can_move = false

	# Add to scene tree temporarily (off-screen)
	var root = Global.get_tree().root
	root.add_child(avatar_preview)
	avatar_preview.set_position(root.get_visible_rect().size)

	# Wait for avatar to be ready and load the profile
	await avatar_preview.avatar.async_update_avatar_from_profile(profile)

	# Generate face snapshot
	var face: Image = await avatar_preview.async_get_viewport_image(true, Vector2i(256, 256), 25)

	# Store face snapshot
	var face_data: PackedByteArray = face.save_png_to_buffer()
	var face_hash: String = DclHashing.hash_v1(face_data)
	await PromiseUtils.async_awaiter(Global.content_provider.store_file(face_hash, face_data))

	# Store local snapshot hash for UI display (not uploaded to server)
	avatar_wire_format.set_snapshots(face_hash, "")

	# Clean up
	avatar_preview.queue_free()

	# Notify subscribers that new snapshot is available
	snapshot_generated.emit(face)
