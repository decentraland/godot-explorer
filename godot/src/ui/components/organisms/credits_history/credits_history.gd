extends VBoxContainer

const HISTORY_ITEM_SCENE = preload(
	"res://src/ui/components/molecules/credits_history_item/credits_history_item.tscn"
)

@onready var item_container: VBoxContainer = $VBoxContainer


func _ready() -> void:
	_rebuild()
	# History is server-backed and refreshed asynchronously; rebuild whenever the
	# manager signals an update (fetch completed, new purchase, refund).
	Iap.transaction_history_updated.connect(_rebuild)
	# Pull fresh data from the backend now that the view is open.
	Iap.refresh_history()


func _rebuild() -> void:
	# remove_child before queue_free (which is deferred): if two history updates land in the
	# same frame, queue_free'd nodes still in the tree would otherwise re-render as duplicates.
	for child in item_container.get_children():
		item_container.remove_child(child)
		child.queue_free()
	for entry in Iap.get_transaction_history():
		_add_item(int(entry.credits), bool(entry.is_refund), str(entry.timestamp))


func _add_item(credits: int, is_refund: bool, timestamp: String) -> void:
	var item = HISTORY_ITEM_SCENE.instantiate()
	item_container.add_child(item)
	item.setup(credits, is_refund, timestamp)
