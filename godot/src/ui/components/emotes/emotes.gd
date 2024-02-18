class_name Emotes
extends RefCounted

const DEFAULT_EMOTE_NAMES = {
	"handsair": "Hands Air",
	"wave": "Wave",
	"fistpump": "Fist Pump",
	"dance": "Dance",
	"raiseHand": "Raise Hand",
	"clap": "Clap",
	"money": "Money",
	"kiss": "Kiss",
	"shrug": "Shrug",
	"headexplode": "Head Explode"
}


static func is_emote_default(urn_or_id: String) -> bool:
	return DEFAULT_EMOTE_NAMES.keys().has(urn_or_id)
