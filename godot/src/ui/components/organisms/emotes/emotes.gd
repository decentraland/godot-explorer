class_name Emotes
extends RefCounted

const BASE_EMOTES_URN_PREFIX = "urn:decentraland:off-chain:base-emotes:"
# Catalyst collection holding the scene "feedback" emotes (throw, punch, sittingChair1…),
# deployed 2026-09-17. Bevy puts these URNs on the wire for `triggerEmote` since
# bevy-explorer#1290; Unity and this client send the bare id.
const BASE_SCENE_EMOTES_URN_PREFIX = "urn:decentraland:off-chain:base-scene-emotes:"
# Emote files shipped inside a scene: `{prefix}{sceneId}-{glbHash}-{loop}`.
const SCENE_EMOTE_URN_PREFIX = "urn:decentraland:off-chain:scene-emote:"

# Emote categories (@dcl/schemas EmoteCategory) — distinct from wearable categories.
const CATEGORIES: PackedStringArray = [
	"dance", "stunt", "greetings", "fun", "poses", "reactions", "horror", "miscellaneous"
]

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

# Lower-cased id -> the id as spelled above, built on first use. Bevy lower-cases the URNs
# it sends (`…:base-scene-emotes:sittingchair2`), so id lookups must ignore case.
static var _embedded_ids_by_lower: Dictionary = {}


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


static func is_scene_emote_urn(urn: String) -> bool:
	return urn.begins_with(SCENE_EMOTE_URN_PREFIX)


## Maps every spelling other clients put on the wire for a built-in emote to the one this
## client plays (#2986):
## - `…:base-scene-emotes:<id>` and `…:base-emotes:<id>` naming a utility emote -> the bare
##   utility id, played from the bundled `default_actions` library
## - `…:base-emotes:<id>` / `…:base-scene-emotes:<id>` naming a base emote -> the
##   `base-emotes` URN with canonical casing
## - a bare id in any case -> the canonical bare id ("sittingchair2" -> "sittingChair2")
## Anything else (on-chain, scene-emote, ids this client doesn't ship) is returned unchanged,
## so an unknown `base-(scene-)emotes` URN still resolves through the catalyst.
static func normalize_emote_id(urn_or_id: String) -> String:
	var lower := urn_or_id.to_lower()
	var id := urn_or_id
	var is_urn := false
	if lower.begins_with(BASE_SCENE_EMOTES_URN_PREFIX):
		id = urn_or_id.substr(BASE_SCENE_EMOTES_URN_PREFIX.length())
		is_urn = true
	elif lower.begins_with(BASE_EMOTES_URN_PREFIX):
		id = urn_or_id.substr(BASE_EMOTES_URN_PREFIX.length())
		is_urn = true
	elif lower.begins_with("urn:"):
		return urn_or_id

	var canonical := _get_embedded_id(id)
	if canonical.is_empty():
		return urn_or_id
	if is_emote_utility(canonical):
		return canonical
	return get_base_emote_urn(canonical) if is_urn else canonical


static func _get_embedded_id(id: String) -> String:
	if _embedded_ids_by_lower.is_empty():
		for key in DEFAULT_EMOTE_NAMES:
			_embedded_ids_by_lower[key.to_lower()] = key
		for key in UTILITY_EMOTE_NAMES:
			_embedded_ids_by_lower[key.to_lower()] = key
	return _embedded_ids_by_lower.get(id.to_lower(), "")
