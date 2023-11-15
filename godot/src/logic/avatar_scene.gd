extends AvatarScene
class_name CustomAvatarScene  #workaround


# TODO: when 4.2 is used, this could be removed
# It's not possible to bind a custom callable to a signal in Rust before 4.2
func _temp_get_custom_callable_on_avatar_changed(avatar_entity_id):
	return self.on_avatar_changed_scene.bind(avatar_entity_id)
