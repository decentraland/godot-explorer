extends Control
const PROFILE_PICTURE = "res://src/ui/components/profile_picture/profile_picture.tscn"

@onready var h_box_container_pictures: HBoxContainer = %HBoxContainer_Pictures
@onready var label: Label = %Label


func _ready() -> void:
	hide()
	label.text = ""
	_clear_profile_pictures()


func async_set_mutual_friends(address):
	hide()  # Hide at the start to prevent flickering
	_clear_profile_pictures()
	var promise = Global.social_service.get_mutual_friends(address, 1000, 0)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		hide()
		return

	var mutual_friends = promise.get_data()
	print(mutual_friends)

	if mutual_friends.size() == 0:
		hide()
	else:
		label.text = str(mutual_friends.size()) + " Mutual"
		_clear_profile_pictures()
		await _add_profile_pictures(mutual_friends)


func _clear_profile_pictures():
	if not h_box_container_pictures:
		return
	for child in h_box_container_pictures.get_children():
		child.queue_free()


func _add_profile_pictures(mutual_friends):
	if not h_box_container_pictures:
		return

	var max_pictures = min(mutual_friends.size(), 3)

	for i in range(max_pictures):
		var mutual = mutual_friends[i]
		var profile_picture_scene = load(PROFILE_PICTURE)
		var profile_picture_instance = profile_picture_scene.instantiate() as ProfilePicture

		profile_picture_instance.picture_size = ProfilePicture.Size.SMALL

		h_box_container_pictures.add_child(profile_picture_instance)
		var data = SocialItemData.new()
		data.profile_picture_url = mutual.get("profile_picture_url", "")
		data.name = mutual.get("name", "")
		data.has_claimed_name = mutual.get("has_claimed_name", false)
		data.address = mutual.get("address", "")
		profile_picture_instance.async_update_profile_picture(data)
	show()
