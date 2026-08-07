class_name MarketplaceUrl
extends RefCounted
## Helpers for building Decentraland web-marketplace URLs.


## Appends the mobile-IAP view flag so the web marketplace renders its
## in-app-purchase layout, plus a body-shape gender filter so only items
## compatible with the player's body shape are surfaced — equipping an item that
## has no representation for the player's body shape renders the avatar naked for
## that slot. Preserves any existing query string (e.g. ?section=).
static func with_mobile_iap(url: String) -> String:
	var separator := "&" if "?" in url else "?"
	var result := url + separator + "view=mobile-iap"
	# Marketplace web reads the body-shape filter from the `genders` URL param.
	var gender := current_player_gender()
	if not gender.is_empty():
		result += "&genders=%s" % gender
	return result


## Returns the marketplace gender filter value ("male"/"female") for the player's
## current body shape, or "" when the body shape is unknown.
static func current_player_gender() -> String:
	if Global.player_identity == null:
		return ""
	var avatar = Global.player_identity.get_mutable_avatar()
	if avatar == null:
		return ""
	return body_shape_to_gender(avatar.get_body_shape())


## Maps a body-shape urn (e.g. urn:...:BaseMale / BaseFemale) to the marketplace
## gender filter value. Checks "female" first because "male" is a substring of it.
static func body_shape_to_gender(body_shape: String) -> String:
	var lower := body_shape.to_lower()
	if "female" in lower:
		return "female"
	if "male" in lower:
		return "male"
	return ""
