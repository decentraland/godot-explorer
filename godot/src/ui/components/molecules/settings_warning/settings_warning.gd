class_name SettingsWarning
extends MarginContainer

## Reusable "heads up" panel for Settings, replicating the skybox/camera-mode warning panels.
## Root is a MarginContainer so each place that instances this can set its own outer spacing
## via the margin theme overrides, independent of the panel's own internal padding.
# i18n-keys: SETTINGS_DEVICE_NO_LONGER_SUPPORTED, SETTINGS_LIMITED_PERFORMANCE_ON_THIS_DEVICE

enum Type { SKYBOX, UNSUPPORTED, BELOW_MINSPEC }

const _SKYBOX_TEXT := "SETTINGS_THE_SKYBOX_IS_SET_BY_THE"
const _UNSUPPORTED_TEXT := "SETTINGS_DEVICE_NO_LONGER_SUPPORTED"
const _BELOW_MINSPEC_TEXT := "SETTINGS_LIMITED_PERFORMANCE_ON_THIS_DEVICE"

@export var type: Type = Type.SKYBOX

@onready var label_description: Label = %Label_Description


func _ready() -> void:
	if type == Type.SKYBOX:
		Global.sdk_skybox_time_active_changed.connect(_on_sdk_skybox_time_active_changed)
	elif type == Type.UNSUPPORTED or type == Type.BELOW_MINSPEC:
		# The device-support check runs async in lobby.gd; this panel can be instantiated (lazily,
		# whenever Settings is opened) before it resolves. Re-check instead of staying frozen on
		# the fail-open default.
		Global.device_support_status_resolved.connect(_refresh)
	_refresh()


func _exit_tree() -> void:
	if (
		type == Type.SKYBOX
		and Global.sdk_skybox_time_active_changed.is_connected(_on_sdk_skybox_time_active_changed)
	):
		Global.sdk_skybox_time_active_changed.disconnect(_on_sdk_skybox_time_active_changed)
	elif (
		(type == Type.UNSUPPORTED or type == Type.BELOW_MINSPEC)
		and Global.device_support_status_resolved.is_connected(_refresh)
	):
		Global.device_support_status_resolved.disconnect(_refresh)


func _refresh() -> void:
	match type:
		Type.SKYBOX:
			label_description.text = TranslationKey.new(_SKYBOX_TEXT).raw()
			visible = is_instance_valid(Global.get_explorer()) and Global.sdk_skybox_time_active
		Type.UNSUPPORTED:
			label_description.text = TranslationKey.new(_UNSUPPORTED_TEXT).raw()
			visible = (
				DeviceSupportCoordinator.check() == DeviceSupportCoordinator.Status.END_OF_SUPPORT
			)
		Type.BELOW_MINSPEC:
			label_description.text = TranslationKey.new(_BELOW_MINSPEC_TEXT).raw()
			visible = (
				DeviceSupportCoordinator.check() == DeviceSupportCoordinator.Status.BELOW_MINSPEC
			)


func _on_sdk_skybox_time_active_changed(_is_active: bool) -> void:
	_refresh()
