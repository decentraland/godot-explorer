extends Control

var avatar_wf: DclAvatarWireFormat

@onready var avatar_preview = %AvatarPreview
@onready var avatar: Avatar = avatar_preview.avatar
@onready var emote_editor := %EmoteEditor


# Called when the node enters the scene tree for the first time.
func _ready():
	var profile: DclUserProfile = DclUserProfile.new()
	avatar_wf = profile.get_avatar()

	emote_editor.avatar = avatar
	emote_editor.set_new_emotes.connect(self._on_set_new_emotes)

	avatar.async_update_avatar(avatar_wf, "No Name")


func _on_set_new_emotes(emotes: PackedStringArray):
	avatar_wf.set_emotes(emotes)
