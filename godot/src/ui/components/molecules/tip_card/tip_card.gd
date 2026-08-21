class_name TipCard
extends PanelContainer

## Keys, not copy. Unlike the profile option tables these are display-only — nothing persists
## or publishes them — so the array can hold keys directly and feed them to tr().
# i18n-keys: TIP_WEARABLES, TIP_EMOTES, TIP_CREATOR_HUB, TIP_FRIENDS
const TIP_KEYS: Array[String] = [
	"TIP_WEARABLES",
	"TIP_EMOTES",
	"TIP_CREATOR_HUB",
	"TIP_FRIENDS",
]

@export var tip_interval: float = 5.0

var _current_index: int = 0

@onready var icon: TextureRect = %Icon
@onready var timer: Timer = %Timer_Tip
@onready var rich_text_label_tip: RichTextLabel = %RichTextLabel_Tip


func _ready() -> void:
	_current_index = randi() % TIP_KEYS.size()
	rich_text_label_tip.text = tr("TIP_CARD_FORMAT") % tr(TIP_KEYS[_current_index])
	timer.wait_time = tip_interval
	timer.timeout.connect(_on_timer_timeout)
	timer.start()


func _on_timer_timeout() -> void:
	_current_index = (_current_index + 1) % TIP_KEYS.size()
	rich_text_label_tip.text = tr("TIP_CARD_FORMAT") % tr(TIP_KEYS[_current_index])


func _notification(what: int) -> void:
	if what == NOTIFICATION_TRANSLATION_CHANGED and is_node_ready():
		rich_text_label_tip.text = tr("TIP_CARD_FORMAT") % tr(TIP_KEYS[_current_index])
