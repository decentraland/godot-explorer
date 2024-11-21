class_name PopupWarning
extends PanelContainer

enum WarningType {
	TIMEOUT,
	MESSAGE,
	WARNING,
}

@onready var texture_icon = %TextureRect_Icon
@onready var label_title = %Label_Title
@onready var label_description = %Label_Description


# @returns the computed length of the description
func set_warning(warning_type: PopupWarning.WarningType, title: String, description: String) -> int:
	match warning_type:
		WarningType.TIMEOUT:
			texture_icon.texture = load("res://assets/themes/dark_dcl_theme/icons/Delay.svg")
		WarningType.MESSAGE:
			texture_icon.texture = load("res://assets/themes/dark_dcl_theme/icons/Chat.svg")
		WarningType.WARNING:
			texture_icon.texture = load("res://assets/ui/warning.svg")

	label_title.text = title
	label_description.text = description
	return label_description.get_parsed_text().length()


func _on_texture_button_exit_pressed():
	self.hide()
