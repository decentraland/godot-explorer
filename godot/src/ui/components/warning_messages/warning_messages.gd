extends Control

const POPUP_WARNING = preload("res://src/ui/components/popup_warning/popup_warning.tscn")


func async_create_popup_warning(
	warning_type: PopupWarning.WarningType, title: String, description: String
):
	var popup_warning = POPUP_WARNING.instantiate()
	popup_warning.modulate = Color.TRANSPARENT
	get_tree().create_tween().tween_property(popup_warning, "modulate", Color.WHITE, 0.25)

	add_child(popup_warning)
	var description_length = popup_warning.set_warning(warning_type, title, description)

	var timeout = max(0, float(description_length) / 20.0) # 30 characters per second to read

	await get_tree().create_timer(timeout).timeout

	var fade_out_tween = get_tree().create_tween()
	fade_out_tween.tween_property(popup_warning, "modulate", Color.TRANSPARENT, 0.5)
	await fade_out_tween.finished

	popup_warning.queue_free()
