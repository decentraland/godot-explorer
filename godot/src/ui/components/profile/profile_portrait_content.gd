extends MarginContainer

signal link_clicked(url: String)
signal emote_pressed(urn: String)
signal stop_emote
signal edit_profile_pressed

var _address: String = ""

@onready var custom_button_edit_profile: Button = %CustomButton_EditProfile
@onready var label_nickname: Label = %Label_Nickname
@onready var label_tag: Label = %Label_Tag
@onready var label_address: Label = %Label_Address
@onready var texture_rect_claimed_checkmark: TextureRect = %TextureRect_ClaimedCheckmark
@onready var button_copy_name: TextureButton = %Button_CopyName
@onready var button_copy_address: TextureButton = %Button_CopyAddress
@onready var profile_about: VBoxContainer = %ProfileAbout
@onready var profile_equipped: VBoxContainer = %ProfileEquipped
@onready var profile_links: VBoxContainer = %ProfileLinks
@onready var separator_about: HSeparator = %HSeparator2
@onready var separator_links: HSeparator = %HSeparator4


func _ready() -> void:
	profile_links.link_clicked.connect(func(url): link_clicked.emit(url))
	profile_equipped.emote_pressed.connect(func(urn): emote_pressed.emit(urn))
	profile_equipped.stop_emote.connect(func(): stop_emote.emit())
	button_copy_name.pressed.connect(_copy_name_and_tag)
	button_copy_address.pressed.connect(_copy_address)
	custom_button_edit_profile.pressed.connect(func(): edit_profile_pressed.emit())


func refresh(profile: DclUserProfile) -> void:
	_refresh_name_and_address(profile)
	profile_about.refresh(profile)
	profile_equipped.async_refresh(profile)
	profile_links.refresh(profile)
	separator_about.visible = profile_about.visible
	separator_links.visible = profile_links.visible


func _refresh_name_and_address(profile: DclUserProfile) -> void:
	_address = profile.get_ethereum_address()
	label_address.text = Global.shorten_address(_address)

	label_nickname.text = profile.get_name()
	var nickname_color = DclAvatar.get_nickname_color(profile.get_name())
	label_nickname.add_theme_color_override("font_color", nickname_color)

	if profile.has_claimed_name():
		texture_rect_claimed_checkmark.show()
		label_tag.text = ""
		label_tag.hide()
	else:
		texture_rect_claimed_checkmark.hide()
		label_tag.show()
		label_tag.text = "#" + _address.substr(_address.length() - 4, 4)


func _copy_name_and_tag() -> void:
	DisplayServer.clipboard_set(label_nickname.text + label_tag.text)


func _copy_address() -> void:
	DisplayServer.clipboard_set(_address)
