extends Node
class_name Wearables


class DefaultWearables:
	const by_body_shapes: Dictionary = {
		BodyShapes.MALE:
		{
			Categories.EYES: "urn:decentraland:off-chain:base-avatars:eyes_00",
			Categories.EYEBROWS: "urn:decentraland:off-chain:base-avatars:eyebrows_00",
			Categories.MOUTH: "urn:decentraland:off-chain:base-avatars:mouth_00",
			Categories.HAIR: "urn:decentraland:off-chain:base-avatars:casual_hair_01",
			Categories.FACIAL: "urn:decentraland:off-chain:base-avatars:beard",
			Categories.UPPER_BODY: "urn:decentraland:off-chain:base-avatars:green_hoodie",
			Categories.LOWER_BODY: "urn:decentraland:off-chain:base-avatars:brown_pants",
			Categories.FEET: "urn:decentraland:off-chain:base-avatars:sneakers"
		},
		BodyShapes.FEMALE:
		{
			Categories.EYES: "urn:decentraland:off-chain:base-avatars:f_eyes_00",
			Categories.EYEBROWS: "urn:decentraland:off-chain:base-avatars:f_eyebrows_00",
			Categories.MOUTH: "urn:decentraland:off-chain:base-avatars:f_mouth_00",
			Categories.HAIR: "urn:decentraland:off-chain:base-avatars:standard_hair",
			Categories.UPPER_BODY: "urn:decentraland:off-chain:base-avatars:f_sweater",
			Categories.LOWER_BODY: "urn:decentraland:off-chain:base-avatars:f_jeans",
			Categories.FEET: "urn:decentraland:off-chain:base-avatars:bun_shoes"
		}
	}


class BodyShapes:
	const FEMALE: String = "urn:decentraland:off-chain:base-avatars:BaseFemale"
	const MALE: String = "urn:decentraland:off-chain:base-avatars:BaseMale"
	const ALL_LIST: PackedStringArray = [FEMALE, MALE]


class ItemRarity:
	const RARE: String = "rare"
	const EPIC: String = "epic"
	const LEGENDARY: String = "legendary"
	const MYTHIC: String = "mythic"
	const UNIQUE: String = "unique"
	const ALL_LIST: PackedStringArray = [RARE, EPIC, LEGENDARY, MYTHIC, UNIQUE]


class Categories:
	const EYES: String = "eyes"
	const EYEBROWS: String = "eyebrows"
	const MOUTH: String = "mouth"
	const FACIAL_HAIR: String = "facial_hair"
	const HAIR: String = "hair"
	const HEAD: String = "head"
	const BODY_SHAPE: String = "body_shape"
	const UPPER_BODY: String = "upper_body"
	const LOWER_BODY: String = "lower_body"
	const FEET: String = "feet"
	const EARRING: String = "earring"
	const EYEWEAR: String = "eyewear"
	const HAT: String = "hat"
	const HELMET: String = "helmet"
	const MASK: String = "mask"
	const TIARA: String = "tiara"
	const TOP_HEAD: String = "top_head"
	const SKIN: String = "skin"
	const FACIAL: String = "facial"
	const HANDS: String = "hands"
	const HANDS_WEAR: String = "hands_wear"

	# Missing: HEAD, FACIAL, HANDS
	const HIDING_PRIORITY = [
		SKIN,
		UPPER_BODY,
		HANDS_WEAR,
		LOWER_BODY,
		FEET,
		HELMET,
		HAT,
		TOP_HEAD,
		MASK,
		EYEWEAR,
		EARRING,
		TIARA,
		HAIR,
		EYEBROWS,
		EYES,
		MOUTH,
		FACIAL_HAIR,
		BODY_SHAPE
	]

	const SKIN_IMPLICIT_CATEGORIES: PackedStringArray = [
		EYES,
		MOUTH,
		EYEBROWS,
		HAIR,
		UPPER_BODY,
		LOWER_BODY,
		FEET,
		HANDS,
		HANDS_WEAR,
		HEAD,
		FACIAL_HAIR
	]

	const UPPER_BODY_DEFAULT_HIDES: PackedStringArray = [HANDS]
	const REQUIRED_CATEGORIES: PackedStringArray = [EYES, MOUTH]

	const ALL_CATEGORIES: PackedStringArray = [
		EYES,
		EYEBROWS,
		MOUTH,
		FACIAL_HAIR,
		HAIR,
		HEAD,
		BODY_SHAPE,
		UPPER_BODY,
		LOWER_BODY,
		FEET,
		EARRING,
		EYEWEAR,
		HAT,
		HELMET,
		MASK,
		TIARA,
		TOP_HEAD,
		SKIN,
		FACIAL,
		HANDS,
		HANDS_WEAR
	]


static func is_texture(category: String) -> bool:
	if (
		category == Categories.EYES
		or category == Categories.EYEBROWS
		or category == Categories.MOUTH
	):
		return true
	return false


static func get_replaces_list(wearable: Dictionary, body_shape_id: String) -> PackedStringArray:
	var representation = get_representation(wearable, body_shape_id)
	if representation.is_empty() or representation.get("overrideHides", []).is_empty():
		return wearable.get("hides", [])
	else:
		return representation.get("overrideHides", [])


static func get_hides_list(wearable: Dictionary, body_shape_id: String) -> PackedStringArray:
	var result: PackedStringArray = []
	var representation = get_representation(wearable, body_shape_id)

	var hides: PackedStringArray = []

	if representation.is_empty() or representation.get("overrideHides", []).is_empty():
		hides.append_array(wearable.get("hides", []))
	else:
		hides.append_array(representation.get("overrideHides", []))

	# we apply this rule to hide the hands by default if the wearable is an upper body or hides the upper body
	var is_or_hides_upper_body: bool = (
		hides.has(Categories.UPPER_BODY) or get_category(wearable) == Categories.UPPER_BODY
	)
	# the rule is ignored if the wearable contains the removal of this default rule (newer upper bodies since the release of hands)
	var removes_hand_default: bool = wearable.get("removesDefaultHiding", []).has(Categories.HANDS)
	# why we do this? because old upper bodies contains the base hand mesh, and they might clip with the new handwear items
	if is_or_hides_upper_body and not removes_hand_default:
		hides.append_array(Categories.UPPER_BODY_DEFAULT_HIDES)

	hides.append_array(get_replaces_list(wearable, body_shape_id))

	# Safeguard the wearable can not hide itself
	var index := hides.find(wearable.get("category", ""))
	if index != -1:
		hides.remove_at(index)

	return result


# @returns Empty if there is no representation
static func get_representation(wearable: Dictionary, body_shape_id: String) -> Dictionary:
	var representation_array = wearable.get("metadata", {}).get("data", {}).get(
		"representations", []
	)
	for representation in representation_array:
		var index = representation.get("bodyShapes", []).find(body_shape_id)
		if index != -1:
			return representation

	return {}


static func get_category(wearable: Dictionary) -> String:
	return wearable.get("metadata", {}).get("data", {}).get("category", "unknown-category")


static func compose_hidden_categories(
	body_shape_id: String, force_render: PackedStringArray, wearables_by_category: Dictionary
) -> PackedStringArray:
	var result: PackedStringArray = []
	var previously_hidden: Dictionary

	for priority_category in Categories.HIDING_PRIORITY:
		previously_hidden[priority_category] = []

		var wearable = wearables_by_category.get(priority_category)

		if wearable == null:
			continue

		var current_hides_list = get_hides_list(wearable, body_shape_id)
		if current_hides_list.is_empty():
			continue

		for category_to_hide in current_hides_list:
			var hidden_categories = previously_hidden.get(category_to_hide)
			if hidden_categories != null and hidden_categories.has(priority_category):
				continue

			previously_hidden[priority_category].push_back(category_to_hide)

			if force_render.has(category_to_hide):
				continue

			result.push_back(category_to_hide)

	return result


static func get_skeleton_from_content(content_hash: String) -> Skeleton3D:
	var content = Global.content_manager.get_resource_from_hash(content_hash)
	if content == null:
		return null

	if not content is Node:
		return null

	var skeleton = content.find_node("Skeleton3D")
	if skeleton == null:
		return null

	return skeleton


static func get_wearable_facial_hashes(wearable: Variant, body_shape_id: String) -> Array[String]:
	if wearable == null:
		return []

	var category = get_category(wearable)
	if not is_texture(category):
		return []

	var representation = get_representation(wearable, body_shape_id)
	if representation.is_empty():
		return []

	var main_file: String = representation.get("mainFile", "").to_lower()
	var content = wearable.get("content", {})
	var main_texture_file_hash = content.get(main_file, "")
	if main_texture_file_hash.is_empty():
		for file_name in content:
			if file_name.ends_with(".png") and not file_name.ends_with("_mask.png"):
				main_texture_file_hash = content[file_name]
				break

	if main_texture_file_hash.is_empty():
		return []

	var mask_texture_file_hash: String
	for file_name in content:
		if file_name.ends_with("_mask.png"):
			mask_texture_file_hash = content[file_name]
			break

	if mask_texture_file_hash.is_empty():
		return [main_texture_file_hash]
	else:
		return [main_texture_file_hash, mask_texture_file_hash]


static func get_wearable_main_file_hash(wearable: Variant, body_shape_id: String) -> String:
	if wearable == null:
		return ""

	var representation = get_representation(wearable, body_shape_id)
	if representation.is_empty():
		return ""

	var main_file: String = representation.get("mainFile", "").to_lower()
	var file_hash = wearable.get("content", {}).get(main_file, "")
	return file_hash


static func is_valid_wearable(
	wearable: Variant, body_shape_id: String, skip_content_integrity: bool = false
) -> bool:
	if wearable == null:
		return false

	var representation = get_representation(wearable, body_shape_id)
	if representation.is_empty():
		return false

	var main_file: String = representation.get("mainFile", "").to_lower()
	var file_hash = wearable.get("content", {}).get(main_file, "")
	if file_hash.is_empty():
		return false

	if not skip_content_integrity:
		var obj = Global.content_manager.get_resource_from_hash(file_hash)
		if obj == null:
			# printerr("wearable ", wearable_key, " doesn't have resource from hash")
			return false

		var category: String = get_category(wearable)
		if obj is Image:
			if not is_texture(category):
				# Category and the object don't match
				return false
		elif obj is Node3D:
			var wearable_skeleton: Skeleton3D = obj.find_child("Skeleton3D")
			if wearable_skeleton == null:
				# The wearable doesn't have a skeleton
				return false
		else:
			# Invalid object
			return false

	return true


static func get_curated_wearable_list(
	body_shape_id: String, wearables: PackedStringArray, force_render: PackedStringArray
) -> Array:
	var wearables_by_category: Dictionary = {}

	var body_shape = Global.content_manager.get_wearable(body_shape_id)
	if not is_valid_wearable(body_shape, body_shape_id):
		return []

	wearables_by_category[Categories.BODY_SHAPE] = body_shape

	for wearable_id in wearables:
		var wearable = Global.content_manager.get_wearable(wearable_id)
		if is_valid_wearable(wearable, body_shape_id):
			var category = get_category(wearable)
			if not wearables_by_category.has(category):
				wearables_by_category[category] = wearable
		else:
			printerr("invalid wearable ", wearable_id)

	var hidden_categories = compose_hidden_categories(
		body_shape_id, force_render, wearables_by_category
	)
	for hide_category in hidden_categories:
		if wearables_by_category.has(hide_category):
			wearables_by_category.erase(hide_category)

	wearables_by_category = set_fallback_for_missing_needed_categories(
		body_shape_id, wearables_by_category, hidden_categories
	)
	return [wearables_by_category, hidden_categories]


static func set_fallback_for_missing_needed_categories(
	body_shape_id: String, wearables_by_category: Dictionary, hidden_categories: PackedStringArray
):
	for needed_catagory in Categories.REQUIRED_CATEGORIES:
		# If a needed category is hidden we dont need to fallback, we skipped it on purpose
		if hidden_categories.has(needed_catagory):
			continue

		# The needed category is present
		if wearables_by_category.has(hidden_categories):
			continue

		var fallback_wearable_id = DefaultWearables.by_body_shapes.get(body_shape_id, {}).get(
			needed_catagory
		)
		if fallback_wearable_id != null:
			var fallback_werable = Global.content_manager.get_wearable(fallback_wearable_id)
			if is_valid_wearable(fallback_werable, body_shape_id):
				wearables_by_category[needed_catagory] = Global.content_manager.get_wearable(
					fallback_wearable_id
				)

	return wearables_by_category
