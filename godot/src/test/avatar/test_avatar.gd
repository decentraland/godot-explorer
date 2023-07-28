extends Node3D
@onready var avatar = $Avatar

var body_shape: String = "urn:decentraland:off-chain:base-avatars:BaseFemale"
var wearables: PackedStringArray = [
	"urn:decentraland:off-chain:base-avatars:f_sweater",
	"urn:decentraland:off-chain:base-avatars:f_jeans",
	"urn:decentraland:off-chain:base-avatars:bun_shoes",
	"urn:decentraland:off-chain:base-avatars:standard_hair",
	"urn:decentraland:off-chain:base-avatars:f_eyes_01",
	"urn:decentraland:off-chain:base-avatars:f_eyebrows_00",
	"urn:decentraland:off-chain:base-avatars:f_mouth_00"
]
var eyes_color: Color = Color(0.3, 0.8, 0.5)
var hair_color: Color = Color(0.5960784554481506, 0.37254902720451355, 0.21568627655506134)
var skin_color: Color = Color(0.4901960790157318, 0.364705890417099, 0.27843138575553894)
var emotes: Array = []


# Called when the node enters the scene tree for the first time.
func _ready():
	avatar.update_avatar(
		"https://peer.decentraland.org/content",
		"Godot User",
		body_shape,
		eyes_color,
		hair_color,
		skin_color,
		wearables,
		emotes
	)


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass

#	avatar.set_running()
