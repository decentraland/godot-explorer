extends PanelContainer

signal toast_clicked(notification: Dictionary)

const DISPLAY_DURATION = 5.0
const SLIDE_IN_DURATION = 0.3
const SLIDE_OUT_DURATION = 0.2

var notification_data: Dictionary = {}
var _timer: Timer

@onready var icon_texture: TextureRect = %IconTexture
@onready var label_title: Label = %LabelTitle
@onready var label_description: Label = %LabelDescription


func _ready() -> void:
	gui_input.connect(_on_gui_input)

	_timer = Timer.new()
	_timer.one_shot = true
	_timer.timeout.connect(_on_timer_timeout)
	add_child(_timer)

	# Start above screen
	position.y = -size.y


func show_notification(notification: Dictionary) -> void:
	notification_data = notification
	_update_ui()

	# Animate slide in from top
	var tween = create_tween()
	tween.set_ease(Tween.EASE_OUT)
	tween.set_trans(Tween.TRANS_BACK)
	tween.tween_property(self, "position:y", 20.0, SLIDE_IN_DURATION)

	# Start auto-hide timer
	_timer.start(DISPLAY_DURATION)


func _update_ui() -> void:
	if notification_data.is_empty():
		return

	# Set title and description from metadata
	if "metadata" in notification_data and notification_data["metadata"] is Dictionary:
		var metadata: Dictionary = notification_data["metadata"]
		label_title.text = metadata.get("title", "Notification")
		label_description.text = metadata.get("description", "")
	else:
		label_title.text = notification_data.get("type", "notification")
		label_description.text = ""

	# Set icon based on notification type
	_set_icon_for_type(notification_data.get("type", ""))


func _set_icon_for_type(notif_type: String) -> void:
	var icon_path := ""

	match notif_type:
		"item_sold", "bid_accepted", "bid_received", "royalties_earned":
			icon_path = "res://assets/ui/notifications/RewardNotification.png"
		"governance_announcement", "governance_proposal_enacted", "governance_voting_ended", "governance_coauthor_requested":
			icon_path = "res://assets/ui/notifications/ProposalFinishedNotification.png"
		"land":
			icon_path = "res://assets/ui/notifications/LandRentedNotification.png"
		"worlds_access_restored", "worlds_access_restricted", "worlds_missing_resources", "worlds_permission_granted", "worlds_permission_revoked":
			icon_path = "res://assets/ui/notifications/WorldAccessRestoredNotification.png"
		_:
			icon_path = "res://assets/ui/notifications/DefaultNotification.png"

	if ResourceLoader.exists(icon_path):
		icon_texture.texture = load(icon_path)


func _on_timer_timeout() -> void:
	hide_toast()


func hide_toast() -> void:
	# Animate slide out to top
	var tween = create_tween()
	tween.set_ease(Tween.EASE_IN)
	tween.set_trans(Tween.TRANS_BACK)
	tween.tween_property(self, "position:y", -size.y - 20.0, SLIDE_OUT_DURATION)
	await tween.finished
	queue_free()


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			toast_clicked.emit(notification_data)
			Global.notification_clicked.emit(notification_data)
			hide_toast()
	elif event is InputEventScreenTouch:
		if event.pressed:
			toast_clicked.emit(notification_data)
			Global.notification_clicked.emit(notification_data)
			hide_toast()
