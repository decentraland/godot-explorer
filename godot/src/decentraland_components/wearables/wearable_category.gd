extends Node
class_name WearableCategory

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

static func is_texture(category: String) -> bool:
	if category == EYES or category == EYEBROWS or category == MOUTH:
		return true
	return false
