class_name SocialItemData
extends RefCounted

enum SocialType { ONLINE, OFFLINE, REQUEST, NEARBY, BLOCKED }

var name: String
var address: String
var profile_picture_url: String
var has_claimed_name: bool
var friendship_id: String = ""
