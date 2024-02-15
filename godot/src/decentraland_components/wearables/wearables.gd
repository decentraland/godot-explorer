class_name Wearables
extends Node

const BASE_WEARABLES: PackedStringArray = [
	"BaseFemale",
	"BaseMale",
	"blue_star_earring",
	"green_feather_earring",
	"square_earring",
	"pink_gem_earring",
	"f_skull_earring",
	"thunder_02_earring",
	"punk_piercing",
	"golden_earring",
	"Thunder_earring",
	"toruspiercing",
	"triple_ring",
	"pearls_earring",
	"f_eyebrows_00",
	"f_eyebrows_01",
	"f_eyebrows_02",
	"f_eyebrows_03",
	"f_eyebrows_04",
	"f_eyebrows_05",
	"f_eyebrows_06",
	"f_eyebrows_07",
	"eyebrows_00",
	"eyebrows_01",
	"eyebrows_02",
	"eyebrows_03",
	"eyebrows_04",
	"eyebrows_05",
	"eyebrows_06",
	"eyebrows_07",
	"f_eyes_00",
	"f_eyes_01",
	"f_eyes_02",
	"f_eyes_03",
	"f_eyes_04",
	"f_eyes_05",
	"f_eyes_06",
	"f_eyes_07",
	"f_eyes_08",
	"f_eyes_09",
	"f_eyes_10",
	"f_eyes_11",
	"eyes_00",
	"eyes_01",
	"eyes_02",
	"eyes_03",
	"eyes_04",
	"eyes_05",
	"eyes_06",
	"eyes_07",
	"eyes_08",
	"eyes_09",
	"eyes_10",
	"eyes_11",
	"black_sun_glasses",
	"cyclope",
	"f_glasses_cat_style",
	"f_glasses_city",
	"f_glasses_fashion",
	"f_glasses",
	"heart_glasses",
	"italian_director",
	"aviatorstyle",
	"matrix_sunglasses",
	"piratepatch",
	"retro_sunglasses",
	"rounded_sun_glasses",
	"thug_life",
	"balbo_beard",
	"lincoln_beard",
	"beard",
	"chin_beard",
	"french_beard",
	"full_beard",
	"goatee_beard",
	"granpa_beard",
	"horseshoe_beard",
	"handlebar",
	"Mustache_Short_Beard",
	"short_boxed_beard",
	"old_mustache_beard",
	"bear_slippers",
	"citycomfortableshoes",
	"classic_shoes",
	"crocs",
	"crocsocks",
	"Espadrilles",
	"bun_shoes",
	"comfy_green_sandals",
	"pink_sleepers",
	"ruby_blue_loafer",
	"ruby_red_loafer",
	"SchoolShoes",
	"sport_black_shoes",
	"sport_colored_shoes",
	"pink_blue_socks",
	"red_sandals",
	"comfy_sport_sandals",
	"m_greenflipflops",
	"m_mountainshoes.glb",
	"m_feet_soccershoes",
	"moccasin",
	"f_m_sandals",
	"sneakers",
	"sport_blue_shoes",
	"hair_anime_01",
	"hair_undere",
	"hair_bun",
	"hair_coolshortstyle",
	"cornrows",
	"double_bun",
	"modern_hair",
	"hair_f_oldie",
	"hair_f_oldie_02",
	"pompous",
	"pony_tail",
	"hair_punk",
	"shoulder_bob_hair",
	"curly_hair",
	"shoulder_hair",
	"standard_hair",
	"hair_stylish_hair",
	"two_tails",
	"moptop",
	"curtained_hair",
	"cool_hair",
	"keanu_hair",
	"slicked_hair",
	"hair_oldie",
	"punk",
	"rasta",
	"semi_afro",
	"semi_bold",
	"short_hair",
	"casual_hair_01",
	"casual_hair_02",
	"casual_hair_03",
	"tall_front_01",
	"f_african_leggins",
	"f_capris",
	"f_brown_skirt",
	"f_brown_trousers",
	"f_country_pants",
	"f_diamond_leggings",
	"distressed_black_Jeans",
	"elegant_blue_trousers",
	"f_jeans",
	"f_red_comfy_pants",
	"f_red_modern_pants",
	"f_roller_leggings",
	"f_school_skirt",
	"f_short_blue_jeans",
	"f_short_colored_leggins",
	"f_sport_shorts",
	"f_stripe_long_skirt",
	"f_stripe_white_pants",
	"f_yoga_trousers",
	"basketball_shorts",
	"brown_pants_02",
	"cargo_shorts",
	"comfortablepants",
	"grey_joggers",
	"hip_hop_joggers",
	"kilt",
	"brown_pants",
	"oxford_pants",
	"safari_pants",
	"jean_shorts",
	"soccer_pants",
	"pijama_pants",
	"striped_swim_suit",
	"swim_short",
	"trash_jean",
	"f_mouth_00",
	"f_mouth_01",
	"f_mouth_02",
	"f_mouth_03",
	"f_mouth_04",
	"f_mouth_05",
	"f_mouth_06",
	"f_mouth_07",
	"f_mouth_08",
	"mouth_00",
	"mouth_01",
	"mouth_02",
	"mouth_03",
	"mouth_04",
	"mouth_05",
	"mouth_06",
	"mouth_07",
	"blue_bandana",
	"diamond_colored_tiara",
	"green_stone_tiara",
	"laurel_wreath",
	"red_bandana",
	"bee_t_shirt",
	"black_top",
	"simple_blue_tshirt",
	"f_blue_elegant_shirt",
	"f_blue_jacket",
	"brown_sleveless_dress",
	"croupier_shirt",
	"colored_sweater",
	"elegant_striped_shirt",
	"simple_green_tshirt",
	"light_green_shirt",
	"f_pink_simple_tshirt",
	"f_pride_t_shirt",
	"f_red_simple_tshirt",
	"f_red_elegant_jacket",
	"Red_topcoat",
	"roller_outfit",
	"school_shirt",
	"baggy_pullover",
	"f_sport_purple_tshirt",
	"striped_top",
	"f_sweater",
	"f_body_swimsuit",
	"f_white_shirt",
	"white_top",
	"f_simple_yellow_tshirt",
	"lovely_yellow_shirt",
	"black_jacket",
	"blue_tshirt",
	"elegant_sweater",
	"green_square_shirt",
	"green_tshirt",
	"green_hoodie",
	"pride_tshirt",
	"puffer_jacket_hoodie",
	"puffer_jacket",
	"red_square_shirt",
	"red_tshirt",
	"safari_shirt",
	"sleeveless_punk_shirt",
	"soccer_shirt",
	"sport_jacket",
	"striped_pijama",
	"striped_shirt_01",
	"m_sweater",
	"m_sweater_02",
	"turtle_neck_sweater",
	# New wearables 2021-10-29
	"yellow_tshirt",
	"eyebrows_8",
	"eyebrows_09",
	"eyebrows_10",
	"eyebrows_11",
	"eyebrows_12",
	"eyebrows_13",
	"eyebrows_14",
	"eyebrows_15",
	"eyebrows_16",
	"eyebrows_17",
	"eyes_12",
	"eyes_13",
	"eyes_14",
	"eyes_15",
	"eyes_16",
	"eyes_17",
	"eyes_18",
	"eyes_19",
	"eyes_20",
	"eyes_21",
	"eyes_22",
	"corduroygreenpants",
	"corduroypurplepants",
	"corduroysandypants",
	"mouth_09",
	"mouth_10",
	"mouth_11",
	"skatercoloredlongsleeve",
	"skaterquadlongsleeve",
	"skatertriangleslongsleeve",
	"denimdungareesblue",
	"denimdungareesred",
	"poloblacktshirt",
	"polobluetshirt",
	"polocoloredtshirt",
	"black_glove",
	"cord_bracelet",
	"dcl_watch",
	"emerald_ring"
]


class DefaultWearables:
	const BY_BODY_SHAPES: Dictionary = {
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
	const COMMON: String = "common"
	const UNCOMMON: String = "uncommon"
	const RARE: String = "rare"
	const EPIC: String = "epic"
	const LEGENDARY: String = "legendary"
	const MYTHIC: String = "mythic"
	const UNIQUE: String = "unique"
	const ALL_LIST: PackedStringArray = [RARE, UNCOMMON, EPIC, LEGENDARY, MYTHIC, UNIQUE]


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

	const MAIN_CATEGORIES: Dictionary = {
		BODY_SHAPE: [BODY_SHAPE],
		HAIR: [EYES, EYEBROWS, MOUTH, FACIAL_HAIR, HAIR],
		UPPER_BODY: [UPPER_BODY],
		HANDS_WEAR: [HANDS_WEAR],
		LOWER_BODY: [LOWER_BODY],
		FEET: [FEET],
		HAT: [EARRING, EYEWEAR, HAT, HELMET, MASK, TIARA, TOP_HEAD],  # accesories...
		SKIN: [SKIN],
	}

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


static func get_base_avatar_urn(wearable_name: String):
	return "urn:decentraland:off-chain:base-avatars:" + wearable_name


static func can_equip(wearable: DclItemEntityDefinition, body_shape_id: String) -> bool:
	return wearable.has_representation(body_shape_id)


static func compose_hidden_categories(
	body_shape_id: String, force_render: PackedStringArray, wearables_by_category: Dictionary
) -> PackedStringArray:
	var result: PackedStringArray = []
	var previously_hidden: Dictionary = {}

	for priority_category in Categories.HIDING_PRIORITY:
		previously_hidden[priority_category] = []

		var wearable: DclItemEntityDefinition = wearables_by_category.get(priority_category)

		if wearable == null:
			continue

		var current_hides_list = wearable.get_hides_list(body_shape_id)
		if current_hides_list.is_empty():
			continue

		for category_to_hide in current_hides_list:
			var hidden_categories = previously_hidden.get(category_to_hide)
			if hidden_categories != null and hidden_categories.has(priority_category):
				continue

			previously_hidden[priority_category].push_back(category_to_hide)

			if force_render.has(category_to_hide):
				continue

			if not result.has(category_to_hide):
				result.push_back(category_to_hide)

	return result


static func get_skeleton_from_content(content_hash: String) -> Skeleton3D:
	var content = Global.content_provider.get_gltf_from_hash(content_hash)
	if content == null:
		return null

	if not content is Node:
		return null

	var skeleton = content.find_node("Skeleton3D")
	if skeleton == null:
		return null

	return skeleton


static func get_wearable_facial_hashes(
	wearable: DclItemEntityDefinition, body_shape_id: String
) -> Array[String]:
	if wearable == null:
		return []

	if not is_texture(wearable.get_category()):
		return []

	if not wearable.has_representation(body_shape_id):
		return []

	var main_file: String = wearable.get_representation_main_file(body_shape_id)
	var content_mapping: DclContentMappingAndUrl = wearable.get_content_mapping()
	var files := content_mapping.get_files()
	var main_texture_file_hash = content_mapping.get_hash(main_file)
	if main_texture_file_hash.is_empty():
		for file_name in files:
			if file_name.ends_with(".png") and not file_name.ends_with("_mask.png"):
				main_texture_file_hash = content_mapping.get_hash(file_name)
				break

	if main_texture_file_hash.is_empty():
		return []

	var mask_texture_file_hash: String
	for file_name in files:
		if file_name.ends_with("_mask.png"):
			mask_texture_file_hash = content_mapping.get_hash(file_name)
			break

	if mask_texture_file_hash.is_empty():
		return [main_texture_file_hash]

	return [main_texture_file_hash, mask_texture_file_hash]


static func get_item_main_file_hash(item: DclItemEntityDefinition, body_shape_id: String) -> String:
	if item == null:
		return ""

	if not item.has_representation(body_shape_id):
		return ""

	var main_file: String = item.get_representation_main_file(body_shape_id)
	var content_mapping: DclContentMappingAndUrl = item.get_content_mapping()
	var file_hash = content_mapping.get_hash(main_file)
	return file_hash


static func is_valid_wearable(
	wearable: DclItemEntityDefinition, body_shape_id: String, skip_content_integrity: bool = false
) -> bool:
	if wearable == null:
		return false

	if not wearable.has_representation(body_shape_id):
		return false

	var main_file: String = wearable.get_representation_main_file(body_shape_id)
	var content_mapping: DclContentMappingAndUrl = wearable.get_content_mapping()
	var file_hash = content_mapping.get_hash(main_file)
	if file_hash.is_empty():
		return false

	if not skip_content_integrity:
		var obj = Global.content_provider.get_gltf_from_hash(file_hash)
		if obj == null:
			obj = Global.content_provider.get_texture_from_hash(file_hash)
		if obj == null:
			# printerr("wearable ", wearable_key, " doesn't have resource from hash")
			return false

		if obj is Image or obj is ImageTexture:
			if not is_texture(wearable.get_category()):
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

	var body_shape = Global.content_provider.get_wearable(body_shape_id)
	if not is_valid_wearable(body_shape, body_shape_id):
		return []

	wearables_by_category[Categories.BODY_SHAPE] = body_shape

	for wearable_id in wearables:
		var wearable: DclItemEntityDefinition = Global.content_provider.get_wearable(wearable_id)
		if is_valid_wearable(wearable, body_shape_id):
			var category = wearable.get_category()
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

		var fallback_wearable_id = DefaultWearables.BY_BODY_SHAPES.get(body_shape_id, {}).get(
			needed_catagory
		)
		if fallback_wearable_id != null:
			var fallback_wearable = Global.content_provider.get_wearable(fallback_wearable_id)
			if is_valid_wearable(fallback_wearable, body_shape_id):
				wearables_by_category[needed_catagory] = Global.content_provider.get_wearable(
					fallback_wearable_id
				)

	return wearables_by_category
