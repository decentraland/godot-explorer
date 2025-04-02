extends PanelContainer

signal open_menu_profile

const HOVER_COLOR = Color("#43404a")
const PRESSED_COLOR = Color("#222026")
const NORMAL_COLOR = Color("#2b2930")

@export var button_group: ButtonGroup = null

var profile_stylebox: StyleBoxFlat = null
var profile_panel_stylebox: StyleBoxFlat = null

@onready var texture_rect_profile = %TextureRect_Profile
@onready var label_avatar_name = %LabelAvatarName
@onready var panel_profile_picture = %Panel_Profile_Picture

@onready var profile_button = %ProfileButton


# gdlint:ignore = async-function-name
func _ready():
	profile_button.set_button_group(button_group)

	profile_stylebox = get_theme_stylebox("panel")
	add_theme_stylebox_override("panel", profile_stylebox)

	profile_panel_stylebox = panel_profile_picture.get_theme_stylebox("panel")
	panel_profile_picture.add_theme_stylebox_override("panel", profile_panel_stylebox)

	profile_panel_stylebox.border_color = NORMAL_COLOR
	profile_stylebox.bg_color = NORMAL_COLOR

	var profile := Global.player_identity.get_profile_or_null()
	if profile != null:
		await _async_on_profile_changed(profile)
	Global.player_identity.profile_changed.connect(self._async_on_profile_changed)


func _async_on_profile_changed(new_profile: DclUserProfile):
	var face256_hash = new_profile.get_avatar().get_snapshots_face_hash()
	var face256_url = new_profile.get_avatar().get_snapshots_face_url()
	var promise = Global.content_provider.fetch_texture_by_url(face256_hash, face256_url)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("menu_profile_button::_async_download_image promise error: ", result.get_error())
		return
	texture_rect_profile.texture = result.texture

	label_avatar_name.load_from_profile(new_profile)


func _on_profile_button_mouse_entered():
	profile_panel_stylebox.border_color = HOVER_COLOR
	profile_stylebox.bg_color = HOVER_COLOR


func _on_profile_button_mouse_exited():
	profile_panel_stylebox.border_color = (
		PRESSED_COLOR if profile_button.button_pressed else NORMAL_COLOR
	)
	profile_stylebox.bg_color = PRESSED_COLOR if profile_button.button_pressed else NORMAL_COLOR


func _on_texture_button_toggled(toggled_on):
	profile_panel_stylebox.border_color = PRESSED_COLOR if toggled_on else NORMAL_COLOR
	profile_stylebox.bg_color = PRESSED_COLOR if toggled_on else NORMAL_COLOR


func _on_profile_button_pressed():
	open_menu_profile.emit()
