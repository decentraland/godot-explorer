extends Control

@onready var avatar_preview = %AvatarPreview
@onready var avatar: Avatar = avatar_preview.avatar
@onready var emote_editor = %EmoteEditor

# Called when the node enters the scene tree for the first time.
func _ready():
	var profile: DclUserProfile = DclUserProfile.new()
	var avatar_wf: DclAvatarWireFormat = profile.get_avatar()
	
	emote_editor.avatar = avatar

	avatar.async_update_avatar(avatar_wf)
