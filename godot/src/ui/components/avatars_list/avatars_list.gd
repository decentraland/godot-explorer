extends Control

signal size_changed

const NEARBY_PLAYER_ITEM = preload(
	"res://src/ui/components/nearby_player_item/nearby_player_item.tscn"
)

var list_size: int = 0

@onready var v_box_container_nearby_players: VBoxContainer = %VBoxContainer_NearbyPlayers


func async_update_nearby_users(remote_avatars: Array) -> void:
	list_size = remote_avatars.size()
	size_changed.emit()
	var children_avatars = []
	for child in v_box_container_nearby_players.get_children():
		if child.avatar != null and is_instance_valid(child.avatar):
			children_avatars.append(child.avatar)
	var avatars_to_remove = []
	for child_avatar in children_avatars:
		if not is_instance_valid(child_avatar):
			continue
		var found = false
		for remote_avatar in remote_avatars:
			if not is_instance_valid(remote_avatar):
				continue
			if child_avatar.get_unique_id() == remote_avatar.get_unique_id():
				found = true
				break
		if not found:
			avatars_to_remove.append(child_avatar)

	var avatars_to_add = []
	for remote_avatar in remote_avatars:
		if not is_instance_valid(remote_avatar):
			continue

		var found = false
		for child_avatar in children_avatars:
			if not is_instance_valid(child_avatar):
				continue
			if remote_avatar.get_unique_id() == child_avatar.get_unique_id():
				found = true
				break
		if not found:
			avatars_to_add.append(remote_avatar)

	for child in v_box_container_nearby_players.get_children():
		if child.avatar != null and is_instance_valid(child.avatar):
			for avatar_to_remove in avatars_to_remove:
				if not is_instance_valid(avatar_to_remove):
					continue
				if child.avatar.get_unique_id() == avatar_to_remove.get_unique_id():
					if (
						child.avatar is Avatar
						and child.avatar.avatar_loaded.is_connected(child.async_set_data)
					):
						child.avatar.avatar_loaded.disconnect(child.async_set_data)

					if child.profile_picture_clicked.is_connected(_on_player_profile_clicked):
						child.profile_picture_clicked.disconnect(_on_player_profile_clicked)

					child.queue_free()
					break

	for avatar in avatars_to_add:
		var avatar_item = NEARBY_PLAYER_ITEM.instantiate()
		v_box_container_nearby_players.add_child(avatar_item)

		avatar_item.profile_picture_clicked.connect(_on_player_profile_clicked)

		if avatar is Avatar:
			if not avatar.avatar_loaded.is_connected(avatar_item.async_set_data):
				avatar.avatar_loaded.connect(avatar_item.async_set_data)
		await avatar_item.async_set_data(avatar)

	var children = v_box_container_nearby_players.get_children()
	var valid_children = []
	for child in children:
		if child.avatar != null and is_instance_valid(child.avatar):
			valid_children.append(child)

	valid_children.sort_custom(self._compare_avatar_names)

	for child in valid_children:
		v_box_container_nearby_players.move_child(child, -1)


func _compare_avatar_names(a, b):
	if not is_instance_valid(a.avatar) or not is_instance_valid(b.avatar):
		return false
	return a.avatar.get_avatar_name() < b.avatar.get_avatar_name()
