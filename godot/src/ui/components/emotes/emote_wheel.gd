extends Control

@export var player: Player = null

var avatar: Avatar = null
var emote_items: Array[EmoteWheelItem] = []

@onready var emote_wheel_container = %EmoteWheelContainer
@onready var label_emote_name = %Label_EmoteName
@onready var label_for_desktop = %Label_ForDesktop


func _ready():
	self.hide()
	avatar = player.avatar
	for child in emote_wheel_container.get_children():
		if child is EmoteWheelItem:
			child.play_emote.connect(self._on_play_emote)
			child.select_emote.connect(self._on_select_emote)
			emote_items.push_back(child)

	label_for_desktop.set_visible(not Global.is_mobile())

	# Temporal mock data until we fetch
	# the emotes of the players with emote v2
	emote_items[0].emote_id = "handsair"
	emote_items[1].emote_id = "wave"
	emote_items[2].emote_id = "fistpump"
	emote_items[3].emote_id = "dance"
	emote_items[4].emote_id = "raiseHand"
	emote_items[5].emote_id = "clap"
	emote_items[6].emote_id = "money"
	emote_items[7].emote_id = "kiss"
	emote_items[8].emote_id = "shrug"
	emote_items[9].emote_id = "headexplode"

	for i in range(emote_items.size()):
		var emote_item = emote_items[i]
		emote_item.number = i
		emote_item.rarity = Wearables.ItemRarityEnum.COMMON
		emote_item.picture = load(
			"res://assets/avatar/default_emotes_thumbnails/%s.png" % emote_item.emote_id
		)


func _gui_input(event):
	if event is InputEventScreenTouch:
		hide()
		Global.explorer_grab_focus()

	if event is InputEventKey:
		# Play emotes
		if event.keycode >= KEY_0 and event.keycode <= KEY_9:
			if event.pressed:
				var index = event.keycode - KEY_0
				var emote_id = emote_items[index].emote_id
				_on_play_emote(emote_id)


func _physics_process(_delta):
	if Input.is_action_just_pressed("ia_open_emote_wheel"):
		show()
		grab_focus()
		Global.release_mouse()


func _on_play_emote(emote_id: String):
	self.hide()
	Global.explorer_grab_focus()
	if avatar:
		avatar.play_emote(emote_id)
		avatar.broadcast_avatar_animation(emote_id)


func _on_select_emote(selected: bool, emote_id: String):
	if !selected:
		label_emote_name.text = ""
		return

	label_emote_name.text = emote_id
