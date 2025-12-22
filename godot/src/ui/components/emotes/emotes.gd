class_name Emotes
extends RefCounted

const BASE_EMOTES_URN_PREFIX = "urn:decentraland:off-chain:base-emotes:"

# Base emotes from avatar-assets repository
const DEFAULT_EMOTE_NAMES = {
	# Original 10 default slot emotes
	"handsair": "Hands Air",
	"wave": "Wave",
	"fistpump": "Fist Pump",
	"dance": "Dance",
	"raiseHand": "Raise Hand",
	"clap": "Clap",
	"money": "Money",
	"kiss": "Kiss",
	"shrug": "Shrug",
	"headexplode": "Head Explode",
	# Additional base emotes
	"cry": "Cry",
	"dab": "Dab",
	"disco": "Disco",
	"dontsee": "Don't See",
	"hammer": "Hammer",
	"hohoho": "Ho Ho Ho",
	"robot": "Robot",
	"snowfall": "Snowfall",
	"tektonik": "Tektonik",
	"tik": "Tik",
	"confettipopper": "Confetti Popper",
}

# Utility/game emotes (triggered by scenes, no thumbnails)
const UTILITY_EMOTE_NAMES = {
	"buttonDown": "Button Down",
	"buttonFront": "Button Front",
	"crafting": "Crafting",
	"getHit": "Get Hit",
	"knockOut": "Knock Out",
	"lever": "Lever",
	"openChest": "Open Chest",
	"openDoor": "Open Door",
	"punch": "Punch",
	"push": "Push",
	"sittingChair1": "Sitting Chair 1",
	"sittingChair2": "Sitting Chair 2",
	"sittingGround1": "Sitting Ground 1",
	"sittingGround2": "Sitting Ground 2",
	"swingWeaponOneHand": "Swing Weapon (One Hand)",
	"swingWeaponTwoHands": "Swing Weapon (Two Hands)",
	"throw": "Throw",
}


static func is_emote_default(urn_or_id: String) -> bool:
	return DEFAULT_EMOTE_NAMES.keys().has(urn_or_id)


static func is_emote_utility(urn_or_id: String) -> bool:
	return UTILITY_EMOTE_NAMES.keys().has(urn_or_id)


static func is_emote_embedded(urn_or_id: String) -> bool:
	return is_emote_default(urn_or_id) or is_emote_utility(urn_or_id)


static func get_emote_name(urn_or_id: String) -> String:
	if DEFAULT_EMOTE_NAMES.has(urn_or_id):
		return DEFAULT_EMOTE_NAMES[urn_or_id]
	if UTILITY_EMOTE_NAMES.has(urn_or_id):
		return UTILITY_EMOTE_NAMES[urn_or_id]
	# Check if it's a base emote URN
	if urn_or_id.begins_with(BASE_EMOTES_URN_PREFIX):
		var emote_id = get_base_emote_id_from_urn(urn_or_id)
		if DEFAULT_EMOTE_NAMES.has(emote_id):
			return DEFAULT_EMOTE_NAMES[emote_id]
	return urn_or_id


static func get_base_emote_urn(emote_id: String) -> String:
	return BASE_EMOTES_URN_PREFIX + emote_id


static func get_base_emote_id_from_urn(urn: String) -> String:
	if urn.begins_with(BASE_EMOTES_URN_PREFIX):
		return urn.substr(BASE_EMOTES_URN_PREFIX.length())
	return urn


static func is_base_emote_urn(urn: String) -> bool:
	return urn.begins_with(BASE_EMOTES_URN_PREFIX)
