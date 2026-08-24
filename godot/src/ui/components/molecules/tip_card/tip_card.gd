class_name TipCard
extends PanelContainer

## Keys, not copy. Unlike the profile option tables these are display-only — nothing persists
## or publishes them — so the array can hold keys directly and feed them to tr().

@export var tip_interval: float = 5.0

var _current_index: int = 0

@onready var icon: TextureRect = %Icon
@onready var timer: Timer = %Timer_Tip
@onready var rich_text_label_tip: RichTextLabel = %RichTextLabel_Tip

static var tip_keys: Array[TranslationKey] = TranslationKey.many(
	["TIP_WEARABLES", "TIP_EMOTES", "TIP_CREATOR_HUB", "TIP_FRIENDS"]
)
static var tip_card_format := TranslationKey.new("TIP_CARD_FORMAT")


func _ready() -> void:
	_current_index = randi() % tip_keys.size()
	rich_text_label_tip.text = tip_card_format.format(tip_keys[_current_index].text())
	timer.wait_time = tip_interval
	timer.timeout.connect(_on_timer_timeout)
	timer.start()


func _on_timer_timeout() -> void:
	_current_index = (_current_index + 1) % tip_keys.size()
	rich_text_label_tip.text = tip_card_format.format(tip_keys[_current_index].text())


func _notification(what: int) -> void:
	if what == NOTIFICATION_TRANSLATION_CHANGED and is_node_ready():
		rich_text_label_tip.text = tip_card_format.format(tip_keys[_current_index].text())
