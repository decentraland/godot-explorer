class_name NotificationTextHelper
extends RefCounted

## Notification Text Helper
##
## Generates notification headers (titles) and descriptions based on notification type and metadata.
## Follows the same pattern as Unity's notification system.


## Get avatar color from username (uses DclAvatar's color algorithm)
static func _get_avatar_color_hex(username: String) -> String:
	var explorer = Global.get_explorer()
	if explorer == null or explorer.player == null:
		return "FFFFFF"  # Default white if no player

	var player_avatar = explorer.player.avatar
	if player_avatar == null:
		return "FFFFFF"  # Default white if no player avatar

	# Use player's avatar instance to calculate the color
	var color = DclAvatar.get_nickname_color(username)
	# Return as hex string without #
	return color.to_html(false)


## Wraps a player's nickname in their avatar colour for use as a notification title. The name is
## server data (not a translatable string), so only the colour is applied here.
static func _get_sender_name_colored(metadata: Dictionary) -> String:
	var sender_name = TranslationServer.translate("COMMON_UNKNOWN_USER")
	if "sender" in metadata and metadata["sender"] is Dictionary:
		sender_name = metadata["sender"].get("name", sender_name)
	var color_hex = _get_avatar_color_hex(sender_name)
	# Escape BBCode in the (user-controlled) name before it reaches the bbcode_enabled title label.
	return "[color=#%s]%s[/color]" % [color_hex, sender_name.replace("[", "[lb]")]


## Get the header/title for a notification based on its type
static func get_notification_header(notif_type: String, metadata: Dictionary) -> String:
	match notif_type:
		# Friend notifications: the title is the sender's nickname tinted with their avatar colour;
		# the action ("wants to be your friend!", …) moves to the description below.
		"social_service_friendship_request", "social_service_friendship_accepted":
			return _get_sender_name_colored(metadata)
		# Community notifications
		"community_invite_received":
			return TranslationServer.translate("NOTIF_HEADER_COMMUNITY_INVITE_RECEIVED")
		"community_member_banned":
			return TranslationServer.translate("NOTIF_HEADER_BANNED_FROM_COMMUNITY")
		"community_member_removed":
			return TranslationServer.translate("NOTIF_HEADER_REMOVED_FROM_COMMUNITY")
		"community_request_to_join_accepted":
			return TranslationServer.translate("NOTIF_HEADER_MEMBERSHIP_REQUEST_ACCEPTED")
		"community_request_to_join_received":
			return TranslationServer.translate("NOTIF_HEADER_MEMBERSHIP_REQUEST_RECEIVED")
		"community_deleted":
			return TranslationServer.translate("NOTIF_HEADER_COMMUNITY_DELETED")
		"community_deleted_content_violation":
			return TranslationServer.translate("NOTIF_HEADER_YOUR_COMMUNITY_HAS_BEEN_DELETED")
		"event_created":
			return TranslationServer.translate("NOTIF_HEADER_COMMUNITY_EVENT_ADDED")
		"community_renamed":
			return TranslationServer.translate("NOTIF_HEADER_COMMUNITY_RENAMED")

		# Badge notifications. The header is a category label we own, so our key wins; the
		# server's `title` is English-only and used to shadow it. The per-notification
		# specifics still come from the server, in get_notification_title() below.
		"badge_granted":
			return TranslationServer.translate("NOTIF_HEADER_NEW_BADGE_UNLOCKED")

		# Marketplace/Credits - use metadata title
		"credits_reminder_do_not_miss_out":
			return TranslationServer.translate("NOTIF_HEADER_DONT_MISS_OUT")
		"item_sold":
			return TranslationServer.translate("NOTIF_HEADER_ITEM_SOLD")
		"bid_accepted":
			return TranslationServer.translate("NOTIF_HEADER_BID_ACCEPTED")
		"bid_received":
			return TranslationServer.translate("NOTIF_HEADER_BID_RECEIVED")
		"royalties_earned":
			return TranslationServer.translate("NOTIF_HEADER_ROYALTIES_EARNED")

		# Governance
		"governance_announcement":
			return TranslationServer.translate("NOTIF_HEADER_DAO_ANNOUNCEMENT")
		"governance_proposal_enacted":
			return TranslationServer.translate("NOTIF_HEADER_PROPOSAL_ENACTED")
		"governance_voting_ended":
			return TranslationServer.translate("NOTIF_HEADER_VOTING_ENDED")
		"governance_coauthor_requested":
			return TranslationServer.translate("NOTIF_HEADER_CO_AUTHOR_REQUESTED")

		# Land
		"land":
			return TranslationServer.translate("NOTIF_HEADER_LAND_UPDATE")

		# Worlds
		"worlds_access_restored":
			return TranslationServer.translate("NOTIF_HEADER_WORLD_ACCESS_RESTORED")
		"worlds_access_restricted":
			return TranslationServer.translate("NOTIF_HEADER_WORLD_ACCESS_RESTRICTED")
		"worlds_missing_resources":
			return TranslationServer.translate("NOTIF_HEADER_WORLD_MISSING_RESOURCES")
		"worlds_permission_granted":
			return TranslationServer.translate("NOTIF_HEADER_WORLD_PERMISSION_GRANTED")
		"worlds_permission_revoked":
			return TranslationServer.translate("NOTIF_HEADER_WORLD_PERMISSION_REVOKED")

		# Events: the title is the event's own name (server metadata), not a category label.
		"events_starts_soon", "events_started", "events_ended":
			var event_name: String = metadata.get("title", "")
			if not event_name.is_empty():
				return event_name
			return TranslationServer.translate("NOTIF_HEADER_EVENT")

		# Rewards: a wearable/item landed in the user's inventory.
		"reward_assignment":
			return TranslationServer.translate("NOTIF_HEADER_NEW_ITEM_RECEIVED")
		"reward_in_progress":
			return TranslationServer.translate("NOTIF_HEADER_REWARD_IN_PROGRESS")

		# Unrecognised type: there is no key to prefer, so the server's title is all we have.
		_:
			return metadata.get("title", TranslationServer.translate("NOTIF_HEADER_NOTIFICATION"))


## Get the description/title text for a notification based on its type and metadata
static func get_notification_title(notif_type: String, metadata: Dictionary) -> String:
	match notif_type:
		# Friend notifications: the action line. The sender's coloured nickname is the title
		# (see get_notification_header), so the description carries only the action copy.
		"social_service_friendship_request":
			return TranslationServer.translate("NOTIF_BODY_WANT_TO_BE_YOUR_FRIEND")

		"social_service_friendship_accepted":
			return TranslationServer.translate("NOTIF_BODY_ACCEPT_YOUR_FRIEND_REQUEST")

		# Community notifications
		"community_invite_received":
			var community_name = metadata.get(
				"communityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return TranslationServer.translate("NOTIF_TITLE_YOUVE_BEEN_INVITED_TO_JOIN_THE").format(
				{"community": community_name}
			)

		"community_member_banned":
			var community_name = metadata.get(
				"communityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return (
				TranslationServer
				. translate("NOTIF_TITLE_YOUVE_BEEN_BANNED_FROM_THE_COMMUNITY")
				. format({"community": community_name})
			)

		"community_member_removed":
			var community_name = metadata.get(
				"communityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return (
				TranslationServer
				. translate("NOTIF_TITLE_YOUVE_BEEN_REMOVED_FROM_THE_COMMUNITY")
				. format({"community": community_name})
			)

		"community_request_to_join_accepted":
			var community_name = metadata.get(
				"communityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return TranslationServer.translate("NOTIF_TITLE_CONGRATS_YOURE_NOW_A_MEMBER_OF").format(
				{"community": community_name}
			)

		"community_user_request_to_join":
			var user_name = metadata.get("userName", TranslationServer.translate("COMMON_SOMEONE"))
			var community_name = metadata.get(
				"communityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return TranslationServer.translate("NOTIF_TITLE_WANTS_TO_JOIN_THE_COMMUNITY").format(
				{"name": user_name, "community": community_name}
			)

		"community_deleted":
			var community_name = metadata.get(
				"communityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return TranslationServer.translate("NOTIF_TITLE_THE_COMMUNITY_HAS_BEEN_DELETED").format(
				{"community": community_name}
			)

		"community_deleted_content_violation":
			var community_name = metadata.get(
				"communityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return (
				TranslationServer
				. translate("NOTIF_TITLE_THE_COMMUNITY_WAS_DELETED_FOR_VIOLATING")
				. format({"community": community_name})
			)

		"event_created":
			var community_name = metadata.get(
				"communityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return TranslationServer.translate("NOTIF_TITLE_THE_COMMUNITY_HAS_ADDED_A_NEW").format(
				{"community": community_name}
			)

		"community_renamed":
			var old_name = metadata.get(
				"oldCommunityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			var new_name = metadata.get(
				"newCommunityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return TranslationServer.translate("NOTIF_TITLE_COMMUNITY_RENAMED").format(
				{"old_name": old_name, "new_name": new_name}
			)

		# Badge notifications - use metadata description
		"badge_granted":
			return metadata.get(
				"description", TranslationServer.translate("NOTIF_TITLE_YOUVE_UNLOCKED_A_NEW_BADGE")
			)

		# Marketplace/Credits - use metadata description
		"credits_reminder_do_not_miss_out":
			return TranslationServer.translate("NOTIF_TITLE_EXPLORE_DECENTRALAND_AND_EARN_REWARDS")
		"item_sold", "bid_accepted", "bid_received", "royalties_earned":
			return metadata.get("description", "")

		# Governance
		"governance_announcement":
			return metadata.get(
				"description",
				TranslationServer.translate("NOTIF_TITLE_NEW_GOVERNANCE_ANNOUNCEMENT")
			)
		"governance_proposal_enacted":
			return metadata.get(
				"description",
				TranslationServer.translate("NOTIF_TITLE_A_PROPOSAL_HAS_BEEN_ENACTED")
			)
		"governance_voting_ended":
			return metadata.get(
				"description", TranslationServer.translate("NOTIF_TITLE_VOTING_HAS_ENDED")
			)
		"governance_coauthor_requested":
			return metadata.get(
				"description",
				TranslationServer.translate("NOTIF_TITLE_YOUVE_BEEN_REQUESTED_AS_CO_AUTHOR")
			)

		# Land
		"land":
			return metadata.get(
				"description", TranslationServer.translate("NOTIF_TITLE_LAND_UPDATE_NOTIFICATION")
			)

		# Worlds
		"worlds_access_restored":
			return metadata.get(
				"description",
				TranslationServer.translate("NOTIF_TITLE_YOUR_WORLD_ACCESS_HAS_BEEN_RESTORED")
			)
		"worlds_access_restricted":
			return metadata.get(
				"description",
				TranslationServer.translate("NOTIF_TITLE_YOUR_WORLD_ACCESS_HAS_BEEN_RESTRICTED")
			)
		"worlds_missing_resources":
			return metadata.get(
				"description",
				TranslationServer.translate("NOTIF_TITLE_YOUR_WORLD_IS_MISSING_RESOURCES")
			)
		"worlds_permission_granted":
			return metadata.get(
				"description", TranslationServer.translate("NOTIF_TITLE_WORLD_PERMISSION_GRANTED")
			)
		"worlds_permission_revoked":
			return metadata.get(
				"description", TranslationServer.translate("NOTIF_TITLE_WORLD_PERMISSION_REVOKED")
			)

		# Events
		"events_started":
			return metadata.get(
				"description", TranslationServer.translate("NOTIF_TITLE_AN_EVENT_HAS_STARTED")
			)
		"events_ended":
			return metadata.get(
				"description", TranslationServer.translate("NOTIF_TITLE_AN_EVENT_HAS_ENDED")
			)

		# Rewards: the description line is just the wearable's name (the label clips it to one line).
		"reward_assignment", "reward_in_progress":
			return metadata.get("tokenName", "")

		_:
			return metadata.get("description", "")
