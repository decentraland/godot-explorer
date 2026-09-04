class_name BugReportModal
extends ModalShell

## Native bug report form (issue #2652). Collects an issue type, a description
## and a screenshot, then files an Intercom ticket via BugReportService.
##
## The chrome comes from ModalShell and every control is a library component
## (DropdownList, DclTextEdit, FieldLabel, ScreenshotSlot, ModalActions), so this
## script holds only form logic — no styling.
##
## TEMPORARY: the approved design has three screenshot slots, but the
## intercom-proxy's `evidence` field carries a single image, so slots 2 and 3
## offered an upload that was silently dropped at submit — worse than not
## offering it. They are hidden until the proxy accepts multiple images; see
## MAX_SCREENSHOTS.

signal submitted(ticket_id: String)
signal cancelled
signal failed(message: String)

const SCREENSHOT_SLOT = preload(
	"res://src/ui/components/molecules/screenshot_slot/screenshot_slot.tscn"
)

# TEMPORARY: 3 in the approved design, capped at 1 while the proxy accepts only one
# image (issue #2652). Restoring the design row is this constant and nothing else —
# the slot/gap logic below is already written for the three-tile case.
const MAX_SCREENSHOTS := 1

# Design uses a wider gap when the row isn't full, and tightens it at three so
# the tiles still fit the content column.
const GAP_SPARSE := 32
const GAP_FULL := 20

const SLOT_WIDTH := 170
const SLOT_HEIGHT := 170

# Orientation to return to when the modal closes. The form is portrait-only by
# design, but in-game the app runs landscape and the modal was being clipped
# (PR #2779 review). Follows the existing precedent: the lobby and Discover pin
# portrait via Global.set_orientation_portrait(), and chatbar.gd toggles it at
# runtime.
var _restore_landscape: bool = false

# Attached images in slot order. Slot 0 is pre-filled with the viewport captured
# when Settings opened, unless the user removes it.
var _images: Array[Image] = []

@onready var dropdown_issue_type: DropdownList = %DropdownList_IssueType
@onready var dcl_text_edit: DclTextEdit = %DclTextEdit
@onready var hbox_screenshots: HBoxContainer = %HBox_Screenshots
@onready var modal_actions: ModalActions = %ModalActions


func _ready() -> void:
	super()
	_force_portrait()
	# Covers the paths that bypass close(): the modal being freed by
	# ModalManager, or the whole screen going away. Leaving the game stuck in
	# portrait would be far worse than a redundant restore.
	tree_exiting.connect(_restore_orientation)
	_populate_issue_types()
	_rebuild_screenshot_slots()
	_update_submit_enabled()
	UiSounds.install_audio_recusirve(self)


## Pre-fills slot 0 with an already-captured image (the viewport grabbed when
## Settings opened). Safe to call with null.
func set_initial_screenshot(image: Image) -> void:
	if image == null or image.is_empty():
		return
	_images.clear()
	_images.append(image)
	if is_node_ready():
		_rebuild_screenshot_slots()


func _populate_issue_types() -> void:
	dropdown_issue_type.clear()
	# Finished text, not keys: DropdownList items are shown by a mode-2 label, the
	# same contract settings.gd uses for the graphics presets.
	for key in BugReportCategories.keys():
		dropdown_issue_type.add_item(tr(key))
	# Left unselected on purpose: Issue Type is required, and DropdownList shows
	# its "Select" placeholder while `selected` is -1.


# Rebuilt wholesale rather than patched: at most four tiles, and this keeps
# add/delete from drifting out of sync with `_images`.
func _rebuild_screenshot_slots() -> void:
	for child in hbox_screenshots.get_children():
		child.queue_free()

	var slot_count := _images.size() + (1 if _images.size() < MAX_SCREENSHOTS else 0)
	# Design keeps tiles at their natural 170px until the row is full, then lets
	# them flex (to ~157) with a tighter gap so three still fit the content column.
	var is_full := slot_count >= MAX_SCREENSHOTS
	hbox_screenshots.add_theme_constant_override("separation", GAP_FULL if is_full else GAP_SPARSE)

	for i in _images.size():
		var slot: ScreenshotSlot = SCREENSHOT_SLOT.instantiate()
		hbox_screenshots.add_child(slot)
		_size_slot(slot)
		slot.locked = modal_actions.is_busy()
		slot.set_image(_images[i])
		slot.delete_pressed.connect(_on_slot_delete_pressed.bind(i))
		slot.replace_pressed.connect(_on_slot_replace_pressed.bind(i))

	if _images.size() < MAX_SCREENSHOTS:
		var add_slot: ScreenshotSlot = SCREENSHOT_SLOT.instantiate()
		hbox_screenshots.add_child(add_slot)
		_size_slot(add_slot)
		# Desktop has no native picker, so the tile is visibly inert there rather
		# than looking tappable and doing nothing.
		add_slot.locked = modal_actions.is_busy() or not ImagePickerService.is_supported()
		add_slot.clear()
		add_slot.add_pressed.connect(_on_slot_add_pressed)


# Tiles keep a fixed footprint whatever the count — a preview that resized as
# you added images made the row feel unstable. Only the gap tightens so three
# still fit the content column.
func _size_slot(slot: ScreenshotSlot) -> void:
	slot.custom_minimum_size = Vector2(SLOT_WIDTH, SLOT_HEIGHT)
	slot.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN


func _on_slot_delete_pressed(index: int) -> void:
	if modal_actions.is_busy() or index < 0 or index >= _images.size():
		return
	_images.remove_at(index)
	_rebuild_screenshot_slots()


func _on_slot_replace_pressed(index: int) -> void:
	_async_replace_screenshot(index)


func _async_replace_screenshot(index: int) -> void:
	if modal_actions.is_busy() or index < 0 or index >= _images.size():
		return
	var image := await ImagePickerService.async_pick_image()
	# Cancelling a replace must leave the existing screenshot alone.
	if image == null:
		return
	_images[index] = image
	_rebuild_screenshot_slots()


func _on_slot_add_pressed() -> void:
	_async_add_screenshot()


func _async_add_screenshot() -> void:
	if modal_actions.is_busy() or _images.size() >= MAX_SCREENSHOTS:
		return
	var image := await ImagePickerService.async_pick_image()
	# Null covers cancel, an unsupported platform and a decode failure alike —
	# none of which should disturb what the user has already filled in.
	if image == null:
		return
	_images.append(image)
	_rebuild_screenshot_slots()


func _on_issue_type_selected(_index: int) -> void:
	_update_submit_enabled()


func _on_dcl_text_edit_changed() -> void:
	_update_submit_enabled()


func _update_submit_enabled() -> void:
	var has_type := dropdown_issue_type.selected >= 0
	var has_text := not dcl_text_edit.get_text_value().strip_edges().is_empty()
	modal_actions.set_primary_enabled(has_type and has_text)


func _on_submit_pressed() -> void:
	_async_submit()


func _async_submit() -> void:
	if modal_actions.is_busy():
		return
	var uuid := BugReportCategories.uuid_at(dropdown_issue_type.selected)
	if uuid.is_empty():
		return

	modal_actions.set_busy(true)
	_rebuild_screenshot_slots()
	var result := await BugReportService.async_submit(uuid, dcl_text_edit.get_text_value(), _images)
	modal_actions.set_busy(false)
	_rebuild_screenshot_slots()
	_update_submit_enabled()

	# Deliberately unlike the Unity client, which reports success optimistically
	# before the request resolves and swallows failures: a reporter who believes
	# their report was filed when it wasn't is worse off than one who retries.
	if result.get("ok", false):
		submitted.emit(str(result.get("id", "")))
		close()
	else:
		failed.emit(str(result.get("error", "")))


func _on_cancel_pressed() -> void:
	if modal_actions.is_busy():
		return
	cancelled.emit()
	close()


func close() -> void:
	_restore_orientation()
	hide()


func _force_portrait() -> void:
	_restore_landscape = not Global.is_orientation_portrait()
	if _restore_landscape:
		Global.set_orientation_portrait()


# Idempotent: close() and tree_exiting can both fire for one modal.
func _restore_orientation() -> void:
	if not _restore_landscape:
		return
	_restore_landscape = false
	Global.set_orientation_landscape()
