class_name SocialHelper


static func social_data_from_avatar(avatar_param: DclAvatar) -> SocialItemData:
	var social_data := SocialItemData.new()
	social_data.name = avatar_param.get_avatar_name()
	social_data.address = avatar_param.avatar_id
	social_data.profile_picture_url = avatar_param.get_avatar_data().get_snapshots_face_url()
	social_data.has_claimed_name = false
	return social_data


static func social_data_from_user_profile(user_profile: DclUserProfile) -> SocialItemData:
	var social_data := SocialItemData.new()
	social_data.name = user_profile.get_name()
	social_data.address = user_profile.get_ethereum_address()
	social_data.profile_picture_url = user_profile.get_avatar().get_snapshots_face_url()
	social_data.has_claimed_name = user_profile.has_claimed_name()
	return social_data
