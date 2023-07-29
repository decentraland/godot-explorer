extends VBoxContainer

@onready
var v_box_container_category = $ColorRect_Background/HBoxContainer/ScrollContainer/ColorRect_Sidebar/MarginContainer/VBoxContainer/HBoxContainer2/ScrollContainer/MarginContainer/VBoxContainer
@onready var avatar = %Avatar
@onready
var wearable_item_instanceable = preload("res://src/ui/components/wearable_item/wearable_item.tscn")
@onready
var grid_container_wearables_list = $ColorRect_Background/HBoxContainer/ScrollContainer/ColorRect_Sidebar/MarginContainer/VBoxContainer/HBoxContainer2/VBoxContainer/ScrollContainer/GridContainer_WearablesList

var filtered_data: Array
var avatar_body_shape: String = "urn:decentraland:off-chain:base-avatars:BaseFemale"
var avatar_wearables: PackedStringArray = [
	"urn:decentraland:off-chain:base-avatars:f_sweater",
	"urn:decentraland:off-chain:base-avatars:f_jeans",
	"urn:decentraland:off-chain:base-avatars:bun_shoes",
	"urn:decentraland:off-chain:base-avatars:standard_hair",
	"urn:decentraland:off-chain:base-avatars:f_eyes_01",
	"urn:decentraland:off-chain:base-avatars:f_eyebrows_00",
	"urn:decentraland:off-chain:base-avatars:f_mouth_00"
]

var avatar_eyes_color: Color = Color(0.3, 0.8, 0.5)
var avatar_hair_color: Color = Color(0.5960784554481506, 0.37254902720451355, 0.21568627655506134)
var avatar_skin_color: Color = Color(0.4901960790157318, 0.364705890417099, 0.27843138575553894)

var emotes: Array = []
var base_wearable_request_id: int = -1
var wearable_data: Dictionary = {}


func _ready():
	for child in v_box_container_category.get_children():
		# TODO: check if it's a wearable_button
		for wearable_button in child.get_children():
			wearable_button.filter_type.connect(self._on_wearable_button_filter_type)
			wearable_button.filter_type.connect(self._on_wearable_button_clear_filter)

	Global.content_manager.wearable_data_loaded.connect(self._on_wearable_data_loaded)

	for wearable_id in Wearables.BASE_WEARABLES:
		var key = "urn:decentraland:off-chain:base-avatars:" + wearable_id
		wearable_data[key] = null

	base_wearable_request_id = Global.content_manager.fetch_wearables(
		wearable_data.keys(), "https://peer.decentraland.org/content/"
	)


func _on_wearable_data_loaded(req_id: int):
	if base_wearable_request_id == -1 or req_id != base_wearable_request_id:
		return

	for wearable_id in wearable_data:
		wearable_data[wearable_id] = Global.content_manager.get_wearable(wearable_id)


func _update_avatar():
	avatar.update_avatar(
		"https://peer.decentraland.org/content",
		"",
		avatar_body_shape,
		avatar_eyes_color,
		avatar_hair_color,
		avatar_skin_color,
		avatar_wearables,
		emotes
	)


func load_filtered_data(filter: String):
	filtered_data = []
	for wearable_id in wearable_data:
		var wearable = wearable_data[wearable_id]
		if Wearables.get_category(wearable) == filter:
			print(wearable_id)
			filtered_data.push_back(wearable_id)
	show_wearables()


func show_wearables():
	for child in grid_container_wearables_list.get_children():
		child.queue_free()

	for wearable_id in filtered_data:
		var wearable_item = wearable_item_instanceable.instantiate()
		grid_container_wearables_list.add_child(wearable_item)
		wearable_item.set_wearable(wearable_data[wearable_id])
		wearable_item.toggled.connect(self._on_wearable_toggled.bind(wearable_id))


func _on_wearable_toggled(_button_toggled: bool, wearable_id: String) -> void:
	var desired_wearable = wearable_data[wearable_id]
	var category = Wearables.get_category(desired_wearable)

	if category == Wearables.Categories.BODY_SHAPE:
		avatar_body_shape = wearable_id
	else:
		var to_remove = []
		# Unequip current wearable with category
		for current_wearable_id in avatar_wearables:
			# TODO: put the fetch wearable function
			var wearable = wearable_data[current_wearable_id]
			if Wearables.get_category(wearable) == category:
				to_remove.push_back(current_wearable_id)

		for to_remove_id in to_remove:
			var index = avatar_wearables.find(to_remove_id)
			avatar_wearables.remove_at(index)

		avatar_wearables.append(wearable_id)

#	print("item ", item, " toggled ", button_toggled)
	_update_avatar()


func _on_wearable_button_filter_type(type):
	load_filtered_data(type)


func _on_wearable_button_clear_filter():
	filtered_data = []
	show_wearables()
