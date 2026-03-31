extends Control

signal toast_clicked(notification: Dictionary)
signal toast_closed
signal mark_as_read(notification: Dictionary)

const DISPLAY_DURATION = 5.0
const SLIDE_IN_DURATION = 0.3
const SLIDE_OUT_DURATION = 0.2
const ICON_MAP: Dictionary = {
	"poor_connection": "res://assets/ui/modal-connection-icon.svg",
	"system": "res://assets/ui/notifications/DefaultNotification.png",
}

var notification_data: Dictionary = {}
var _timer: Timer

@onready var panel: PanelContainer = $Panel
@onready var label_title: Label = %LabelTitle
@onready var label_description: RichTextLabel = %LabelDescription
@onready var notification_image: TextureRect = %NotificationImage


func _ready() -> void:
	_timer = Timer.new()
	_timer.one_shot = true
	_timer.timeout.connect(_on_timer_timeout)
	add_child(_timer)


func async_show_notification(notification: Dictionary) -> void:
	notification_data = notification
	var metadata: Dictionary = notification.get("metadata", {})
	label_title.text = metadata.get("title", "")
	label_description.text = metadata.get("description", "")

	var notif_type: String = notification.get("type", "")
	var icon_path: String = ICON_MAP.get(notif_type, "")
	if not icon_path.is_empty() and ResourceLoader.exists(icon_path):
		notification_image.texture = load(icon_path)

	position.y = -size.y

	var tween = create_tween()
	tween.set_ease(Tween.EASE_OUT)
	tween.set_trans(Tween.TRANS_BACK)
	tween.tween_property(self, "position:y", -15.0, SLIDE_IN_DURATION)
	await tween.finished

	_timer.start(DISPLAY_DURATION)


func _on_timer_timeout() -> void:
	NotificationsManager.resume_queue()
	async_hide_toast()


func async_hide_toast() -> void:
	var tween = create_tween()
	tween.set_ease(Tween.EASE_IN)
	tween.set_trans(Tween.TRANS_BACK)
	tween.tween_property(self, "position:y", -size.y - 20.0, SLIDE_OUT_DURATION)
	await tween.finished
	toast_closed.emit()
	queue_free()
