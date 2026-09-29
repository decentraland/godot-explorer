class_name ActionIcons
extends RefCounted
## The single source of truth for what an `ia_*` action looks like on screen.
##
## The joypad button and the in-world interaction pill must show the SAME glyph for the same
## action — the pill is a hint about which button to press, so a mismatch is a straight-up lie.
## They used to keep two private tables keyed by the same action ids, and the tables drifted: when
## the HUD revamp (PR #2723) replaced the joypad icon set, the tooltip's copy was left behind and
## kept showing the old filled-white interact and jump glyphs for months.
##
## Anything that needs an action's appearance reads it from here. Adding an action means adding one
## row, not two. This sits in `components/utils/` so a molecule (the tooltip) never has to depend on
## an organism (the joypad) — the same arrangement `SdkTouchControlsApplier` already uses for the
## scene-supplied half of this problem.
##
## Not covered here, by design: the joypad's per-frame jump state (jump / double-jump / glide, see
## `joypad.gd::_update_jump_icon`) and a creator's `PBTouchScreenControls` override. Both are
## runtime state, not a static property of the action. The pill shows the base glyph for the action;
## if it ever needs to track the live joypad state, that wants a lookup against the joypad itself.

# The keys below are only ever passed as function arguments, which `extract_strings.py` cannot see.
# i18n-keys: TOOLTIP_ACTION_TAP, TOOLTIP_ACTION_JUMP, TOOLTIP_ACTION_PRIMARY
# i18n-keys: TOOLTIP_ACTION_SECONDARY, TOOLTIP_ACTION_1, TOOLTIP_ACTION_2
# i18n-keys: TOOLTIP_ACTION_3, TOOLTIP_ACTION_4

# Per-action glyphs (normal / pressed). 100x100, sized to the button through expand_icon.
const IC_INTERACT_NORMAL = preload("uid://c55dfgqwdxs8f")
const IC_INTERACT_PRESSED = preload("uid://ct0wqa804vtni")
const IC_E_NORMAL = preload("uid://ck3e0eaelc3rq")
const IC_E_PRESSED = preload("uid://01qlcj0sqqnw")
const IC_F_NORMAL = preload("uid://72h2xkpj1hgk")
const IC_F_PRESSED = preload("uid://c5u8stl6jg8cl")
const IC_1_NORMAL = preload("uid://e0ug4dbj1y10")
const IC_1_PRESSED = preload("uid://ddrk8qdneg8lw")
const IC_2_NORMAL = preload("uid://cqxtpai3pix5u")
const IC_2_PRESSED = preload("uid://bbqcb676u1mrv")
const IC_3_NORMAL = preload("uid://sw0euo71n3gv")
const IC_3_PRESSED = preload("uid://dm5mjc6eto6v1")
const IC_4_NORMAL = preload("uid://dvpirmcnk4c2a")
const IC_4_PRESSED = preload("uid://dx8f2nledowsj")
const IC_JUMP_NORMAL = preload("uid://d4neuk8df8m4y")
const IC_JUMP_PRESSED = preload("uid://dykud4ptnkdei")

## action -> { normal, pressed, keycap, label_key }
##
## `keycap` is the letter a keyboard-style action shows instead of a glyph, empty when the action is
## icon-based. `label_key` is a raw translation KEY, not a TranslationKey — building one is a
## runtime call and cannot appear in a `const`; the caller wraps it (see tooltip_label.gd).
const ACTIONS := {
	"ia_pointer":
	{
		"normal": IC_INTERACT_NORMAL,
		"pressed": IC_INTERACT_PRESSED,
		"keycap": "",
		"label_key": "TOOLTIP_ACTION_TAP",
	},
	# The joypad drives jump per frame and never reads this row; it is the pill's base glyph.
	"ia_jump":
	{
		"normal": IC_JUMP_NORMAL,
		"pressed": IC_JUMP_PRESSED,
		"keycap": "",
		"label_key": "TOOLTIP_ACTION_JUMP",
	},
	"ia_primary":
	{
		"normal": IC_E_NORMAL,
		"pressed": IC_E_PRESSED,
		"keycap": "E",
		"label_key": "TOOLTIP_ACTION_PRIMARY",
	},
	"ia_secondary":
	{
		"normal": IC_F_NORMAL,
		"pressed": IC_F_PRESSED,
		"keycap": "F",
		"label_key": "TOOLTIP_ACTION_SECONDARY",
	},
	"ia_action_3":
	{
		"normal": IC_1_NORMAL,
		"pressed": IC_1_PRESSED,
		"keycap": "1",
		"label_key": "TOOLTIP_ACTION_1",
	},
	"ia_action_4":
	{
		"normal": IC_2_NORMAL,
		"pressed": IC_2_PRESSED,
		"keycap": "2",
		"label_key": "TOOLTIP_ACTION_2",
	},
	"ia_action_5":
	{
		"normal": IC_3_NORMAL,
		"pressed": IC_3_PRESSED,
		"keycap": "3",
		"label_key": "TOOLTIP_ACTION_3",
	},
	"ia_action_6":
	{
		"normal": IC_4_NORMAL,
		"pressed": IC_4_PRESSED,
		"keycap": "4",
		"label_key": "TOOLTIP_ACTION_4",
	},
}


## True when the action has an entry here at all.
static func has_action(action: String) -> bool:
	return ACTIONS.has(action)


## The normal/pressed pair the joypad skins its button with, or [] for an unknown action.
## Jump is excluded on purpose: the joypad resolves it per frame from the player's jump state.
static func button_icons(action: String) -> Array:
	if action == "ia_jump" or not ACTIONS.has(action):
		return []
	var entry: Dictionary = ACTIONS[action]
	return [entry["normal"], entry["pressed"]]


## The glyph the interaction pill shows, or null when the action uses a letter keycap instead.
static func tooltip_icon(action: String) -> Texture2D:
	if not ACTIONS.has(action):
		return null
	var entry: Dictionary = ACTIONS[action]
	return null if String(entry["keycap"]) != "" else entry["normal"]


## The letter the interaction pill shows, or "" when the action uses a glyph instead.
static func tooltip_keycap(action: String) -> String:
	if not ACTIONS.has(action):
		return ""
	return String(ACTIONS[action]["keycap"])


## Fallback label for the pill when the scene supplies no hover text of its own.
static func tooltip_label_key(action: String) -> TranslationKey:
	if not ACTIONS.has(action):
		return null
	return TranslationKey.new(String(ACTIONS[action]["label_key"]))
