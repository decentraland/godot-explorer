extends Control

signal size_changed

enum SocialType { ONLINE, OFFLINE, REQUEST, NEARBY, BLOCKED }
@export var player_list_type: SocialType

var list_size: int = 0
# Aliases para avatares bloqueados (rango 20000-29999)
const BLOCKED_AVATAR_ALIAS_BASE: int = 20000

func _ready():
	async_update_list()
	# Connect to avatar scene changed signal instead of using timer
	Global.avatars.avatar_scene_changed.connect(self.async_update_list)
	Global.social_blacklist.blacklist_changed.connect(self.async_update_list)

func _get_alias_for_address(address: String) -> int:
	# Genera un alias único basado en el hash de la dirección
	var hash = address.hash()
	# Asegura que el alias esté en el rango 20000-29999
	return BLOCKED_AVATAR_ALIAS_BASE + (hash % 10000)

func async_update_list(_remote_avatars: Array = []) -> void:
	match player_list_type:
		SocialType.NEARBY:
			_update_nearby_list()
		SocialType.BLOCKED:
			await async_update_blocked_list()
		SocialType.ONLINE:
			await async_update_blocked_list()
		SocialType.OFFLINE:
			await async_update_blocked_list()
		SocialType.REQUEST:
			await async_update_blocked_list()
		_:
			pass

func async_update_blocked_list() -> void:
	var blocked_avatars = []
	var blocked_addresses = Global.social_blacklist.get_blocked_list()
	
	for address in blocked_addresses:
		var avatar = Global.avatars.get_avatar_by_address(address)
		
		# Si el avatar no existe, crearlo y fetchear su perfil
		if avatar == null or not is_instance_valid(avatar):
			var alias = _get_alias_for_address(address)
			
			# Crear el avatar
			Global.avatars.add_avatar(alias, address)
			
			# Esperar un frame para que el avatar se cree
			await get_tree().process_frame
			
			# Obtener el avatar recién creado
			avatar = Global.avatars.get_avatar_by_address(address)
			
			if avatar != null and is_instance_valid(avatar):
				# Fetchear el perfil del usuario (los perfiles ya existen, solo los obtenemos)
				var profile_promise = Global.content_provider.fetch_profile(address)
				var profile_result = await PromiseUtils.async_awaiter(profile_promise)
				
				if not profile_result is PromiseError:
					var profile = profile_result
					if profile != null and profile is DclUserProfile:
						# Actualizar el avatar con el perfil fetcheado
						Global.avatars.update_dcl_avatar_by_alias(alias, profile)
						
						# Esperar a que el avatar se actualice
						await get_tree().process_frame
		
		# Agregar el avatar a la lista si es válido
		if avatar != null and is_instance_valid(avatar) and avatar is Avatar:
			blocked_avatars.append(avatar)
	
	list_size = blocked_avatars.size()
	size_changed.emit()
	remove_avatars(blocked_avatars)
	add_avatars(blocked_avatars)
	

func _update_nearby_list() -> void:
	var all_avatars = Global.avatars.get_avatars()
	var avatars = []
	
	for avatar in all_avatars:
		if avatar != null and avatar is Avatar:
			var avatar_address = avatar.avatar_id
			if not avatar_address.is_empty() and not Global.social_blacklist.is_blocked(avatar_address):
				avatars.append(avatar)
	
	list_size = avatars.size()
	size_changed.emit()
	remove_avatars(avatars)
	add_avatars(avatars)

func _compare_avatar_names(a, b):
	if not is_instance_valid(a.avatar) or not is_instance_valid(b.avatar):
		return false
	return a.avatar.get_avatar_name() < b.avatar.get_avatar_name()

		
		
func get_avatar_children():
	var children_avatars = []
	for child in self.get_children():
		if child.avatar != null and is_instance_valid(child.avatar):
			children_avatars.append(child.avatar)
	return children_avatars


func get_avatars_to_remove(avatars_list) -> Array:
	var avatars_to_remove = []
	for child_avatar in get_avatar_children():
		if not is_instance_valid(child_avatar):
			continue
		var found = false
		for avatar in avatars_list:
			if not is_instance_valid(avatar):
				continue
			if child_avatar.unique_id == avatar.unique_id:
				found = true
				break
		if not found:
			avatars_to_remove.append(child_avatar)
	return avatars_to_remove


func get_avatars_to_add(avatars_list) -> Array:
	var avatars_to_add = []
	for avatar in avatars_list:
		if not is_instance_valid(avatar):
			continue

		var found = false
		for child_avatar in get_avatar_children():
			if not is_instance_valid(child_avatar):
				continue
			if avatar.unique_id == child_avatar.unique_id:
				found = true
				break
		if not found:
			avatars_to_add.append(avatar)
	return avatars_to_add


func remove_avatars(avatars_list) -> void:
	for child in self.get_children():
		if child.avatar == null or not is_instance_valid(child.avatar):
			continue
		for avatar_to_remove in get_avatars_to_remove(avatars_list):
			if not is_instance_valid(avatar_to_remove):
				continue
			if child.avatar.unique_id == avatar_to_remove.unique_id:
				if (
					child.avatar is Avatar
					and child.avatar.avatar_loaded.is_connected(child.async_set_data_from_avatar)
				):
					child.avatar.avatar_loaded.disconnect(child.async_set_data_from_avatar)
				child.queue_free()
				break
				

func add_avatars(avatars_list) -> void:
	for avatar in get_avatars_to_add(avatars_list):
		var social_item = Global.preload_assets.SOCIAL_ITEM.instantiate()
		self.add_child(social_item)
		
		if avatar is Avatar:
			if not avatar.avatar_loaded.is_connected(social_item.async_set_data_from_avatar):
				avatar.avatar_loaded.connect(social_item.async_set_data_from_avatar)
		await social_item.async_set_data_from_avatar(avatar)
		social_item.set_type(player_list_type)

	var children = self.get_children()
	var valid_children = []
	for child in children:
		if child.avatar != null and is_instance_valid(child.avatar):
			valid_children.append(child)

	valid_children.sort_custom(self._compare_avatar_names)

	for child in valid_children:
		self.move_child(child, -1)
