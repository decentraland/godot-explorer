extends AvatarScene
class_name CustomAvatarScene  #workaround


func _temp_get_custom_callable_on_avatar_changed(avatar_entity_id):
	return self.on_avatar_changed_scene.bind(avatar_entity_id)
