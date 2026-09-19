class_name PresetAvatarCarousel
extends Control

signal preset_selected(preset_data: Dictionary)

const PRESET_CARD_SCENE = preload(
	"res://src/ui/components/atoms/controls/preset_avatar_card/preset_avatar_card.tscn"
)
const PRESET_COUNT = 12

var _button_group: ButtonGroup
var _loaded = false

@onready var h_box_container_cards: HBoxContainer = %HBoxContainerCards
@onready var scroll_container: Container = %ScrollContainer


func _ready() -> void:
	_button_group = ButtonGroup.new()
	_button_group.allow_unpress = false
	_button_group.pressed.connect(_on_button_group_pressed)
	visibility_changed.connect(_on_visibility_changed)
	if is_visible_in_tree():
		_async_load_presets()


func _on_visibility_changed() -> void:
	if is_visible_in_tree() and not _loaded:
		_async_load_presets()


func _async_load_presets() -> void:
	_loaded = true
	var cards: Array[PresetAvatarCard] = []
	for i in range(1, PRESET_COUNT + 1):
		var card: PresetAvatarCard = PRESET_CARD_SCENE.instantiate()
		card.button_group = _button_group
		h_box_container_cards.add_child(card)
		if i == 1:
			card.set_pressed_no_signal(true)
		cards.append(card)
		# 0-based avatar_id surfaced in the PRESET_SELECT metric (issue #2377).
		card.async_load_preset("default%d" % i, i - 1)


func _on_button_group_pressed(button: BaseButton) -> void:
	var card = button as PresetAvatarCard
	if card:
		preset_selected.emit(card.preset_data)
