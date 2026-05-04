extends PlaceItem


func set_data(item_data) -> void:
	super.set_data(item_data)

	if not item_data is Dictionary:
		return

	var friend_nickname: String = item_data.get("_friend_name", "")
	var friend_address: String = item_data.get("_friend_address", "")
	var has_claimed_name: bool = item_data.get("_friend_has_claimed_name", false)
	var location_name: String = item_data.get("title", "")

	# In the carousel card, Label_Title shows the friend nickname instead of the place title,
	# and Label_Location shows the place name instead of coordinates.
	var label_title = _get_label_title()
	if label_title and not friend_nickname.is_empty():
		label_title.text = friend_nickname
	var label_location = _get_label_location()
	if label_location and not location_name.is_empty():
		label_location.text = location_name

	var friend_name_label = _get_node_safe("Label_FriendName")
	if friend_name_label:
		friend_name_label.text = friend_nickname

	var friend_tag_label = _get_node_safe("Label_FriendTag")
	if friend_tag_label:
		if has_claimed_name:
			friend_tag_label.hide()
		else:
			if not friend_address.is_empty():
				friend_tag_label.text = "#" + friend_address.substr(2, 4)
			else:
				friend_tag_label.text = ""
			friend_tag_label.show()

	var checkmark = _get_node_safe("TextureRect_ClaimedCheckmark")
	if checkmark:
		checkmark.visible = has_claimed_name

	var filler = _get_node_safe("Control_Filler")
	if filler:
		filler.visible = has_claimed_name

	var profile_pic = _get_node_safe("ProfilePicture")
	if profile_pic:
		var social_data = SocialItemData.new()
		social_data.name = friend_nickname
		social_data.address = friend_address
		social_data.profile_picture_url = item_data.get("_friend_profile_picture_url", "")
		social_data.has_claimed_name = has_claimed_name
		profile_pic.async_update_profile_picture(social_data)
