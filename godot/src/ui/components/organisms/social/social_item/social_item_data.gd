class_name SocialItemData
extends RefCounted

# REQUEST = friend requests received by us; REQUEST_SENT = requests we sent (pending outgoing).
# Appended at the end so existing values (used as player_list_type ints in .tscn) don't shift.
enum SocialType { ONLINE, OFFLINE, REQUEST, NEARBY, BLOCKED, REQUEST_SENT }

var name: String
var address: String
var profile_picture_url: String
var has_claimed_name: bool
var friendship_id: String = ""
