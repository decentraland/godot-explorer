class_name TipCard
extends PanelContainer

const TIPS: Array[String] = [
	"Wearables shape how you appear over time. Made by the community.",
	"Use Emotes to wave, react, or celebrate. Press B to open the Emote Wheel.",
	"Use the Creator Hub to build your own places and experiences.",
	"Decentraland is better with friends. Meet new people and see where they’re hanging out."
]

@export var tip_interval: float = 5.0

@onready var icon: TextureRect = %Icon
@onready var timer: Timer = %Timer_Tip
@onready var rich_text_label_tip: RichTextLabel = %RichTextLabel_Tip

var _current_index: int = 0


func _ready() -> void:
	_current_index = randi() % TIPS.size()
	rich_text_label_tip.text = "[b]Tip:[/b] " + TIPS[_current_index]
	timer.wait_time = tip_interval
	timer.timeout.connect(_on_timer_timeout)
	timer.start()


func _on_timer_timeout() -> void:
	_current_index = (_current_index + 1) % TIPS.size()
	rich_text_label_tip.text = "[b]Tip:[/b] " + TIPS[_current_index]
