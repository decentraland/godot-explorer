class_name IapPurchaseSuccessModal
extends CanvasLayer

# Confirmation modal shown after the backend has granted credits for a
# purchase (or a re-delivered transaction). Dismissed by tapping anywhere
# on the backdrop. Visual is intentionally placeholder — design pending.

var _pending_credits: int = 0

@onready var _credits_label: Label = %CreditsLabel
@onready var _backdrop: ColorRect = %Backdrop


func setup(credits: int) -> void:
	# Safe to call before _ready: the @onready vars haven't resolved yet, so
	# stash the value and apply it once the scene is in the tree.
	_pending_credits = credits
	if is_node_ready():
		_apply_credits()


func _ready() -> void:
	_apply_credits()


func _apply_credits() -> void:
	if _credits_label != null:
		_credits_label.text = "Credits x%d" % _pending_credits



func _on_backdrop_gui_input(event: InputEvent) -> void:
	queue_free()
