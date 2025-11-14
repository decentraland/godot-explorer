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
	#Global.get_explorer().hud_button_friends.friends_clicked.connect(self.async_update_list)


func async_update_list(_remote_avatars: Array = []) -> void:
	match player_list_type:
		SocialType.NEARBY:
			_reload_nearby_list()
		SocialType.BLOCKED:
			await _reload_blocked_list()
		SocialType.ONLINE:
			await _reload_blocked_list()
		SocialType.OFFLINE:
			await _reload_blocked_list()
		SocialType.REQUEST:
			await _reload_blocked_list()
		_:
			pass

func _reload_blocked_list() -> void:
	var blocked_social_items = []
	var blocked_addresses = Global.social_blacklist.get_blocked_list()
	
	for address in blocked_addresses:
		var social_item_data = SocialItemData.new()
		social_item_data.name = address
		social_item_data.address = address
		social_item_data.has_claimed_name = false
		social_item_data.profile_picture_url = address
		blocked_social_items.append(social_item_data)
		
	remove_items()
	add_items_by_social_item_data(blocked_social_items)
	

func _reload_nearby_list() -> void:
	remove_items()
	var all_avatars = Global.avatars.get_avatars()
	var avatars = []
	var seen_addresses = {}  # Diccionario para rastrear direcciones ya agregadas
	
	for avatar in all_avatars:
		if avatar != null and avatar is Avatar:
			var avatar_address = avatar.avatar_id
			if not avatar_address.is_empty() and not Global.social_blacklist.is_blocked(avatar_address):
				# Verificar si ya agregamos este avatar_id para evitar duplicados
				if not seen_addresses.has(avatar_address):
					seen_addresses[avatar_address] = true
					avatars.append(avatar)
	
	list_size = avatars.size()
	size_changed.emit()
	add_items_by_avatar(avatars)


func _compare_names(a, b):
	return a.social_data.name < b.social_data.name


func remove_items() -> void:
	for child in self.get_children():
		child.queue_free()


func add_items_by_avatar(avatar_list) -> void:
	for avatar in avatar_list:
		var social_item = Global.preload_assets.SOCIAL_ITEM.instantiate()
		self.add_child(social_item)
		social_item.set_type(player_list_type)
		social_item.set_data_from_avatar(avatar as Avatar)

func add_items_by_social_item_data(item_list) -> void:
	for item in item_list:
		var social_item = Global.preload_assets.SOCIAL_ITEM.instantiate()
		self.add_child(social_item)
		social_item.set_type(player_list_type)
		social_item.set_data(item as SocialItemData)
