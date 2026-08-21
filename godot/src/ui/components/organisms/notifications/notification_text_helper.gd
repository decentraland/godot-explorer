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


## Get the header/title for a notification based on its type
static func get_notification_header(notif_type: String, metadata: Dictionary) -> String:
	match notif_type:
		# Friend notifications
		"social_service_friendship_request":
			return TranslationServer.translate("NOTIF_HEADER_FRIEND_REQUEST_RECEIVED")
		"social_service_friendship_accepted":
			return TranslationServer.translate("NOTIF_HEADER_FRIEND_REQUEST_ACCEPTED")
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

		# Badge notifications - use metadata title
		"badge_granted":
			return metadata.get(
				"title", TranslationServer.translate("NOTIF_HEADER_NEW_BADGE_UNLOCKED")
			)

		# Marketplace/Credits - use metadata title
		"credits_reminder_do_not_miss_out":
			return TranslationServer.translate("NOTIF_HEADER_DONT_MISS_OUT")
		"item_sold", "bid_accepted", "bid_received", "royalties_earned":
			return metadata.get("title", "Notification")

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

		# Events
		"events_started":
			return metadata.get("title", TranslationServer.translate("NOTIF_HEADER_EVENT_STARTED"))
		"events_ended":
			return metadata.get("title", TranslationServer.translate("NOTIF_HEADER_EVENT_ENDED"))

		# Rewards
		"reward_assigned":
			return TranslationServer.translate("NOTIF_HEADER_REWARD_ASSIGNED")
		"reward_in_progress":
			return TranslationServer.translate("NOTIF_HEADER_REWARD_IN_PROGRESS")

		_:
			return metadata.get("title", "Notification")


## Get the description/title text for a notification based on its type and metadata
static func get_notification_title(notif_type: String, metadata: Dictionary) -> String:
	match notif_type:
		# Friend notifications
		"social_service_friendship_request":
			if "sender" in metadata and metadata["sender"] is Dictionary:
				var sender = metadata["sender"]
				var sender_name = sender.get("name", "Unknown")
				var has_claimed_name = sender.get("hasClaimedName", false)
				var color_hex = _get_avatar_color_hex(sender_name)

				if has_claimed_name:
					return (
						TranslationServer.translate("NOTIF_TITLE_WANTS_TO_BE_YOUR_FRIEND_2")
						% [color_hex, sender_name]
					)

				var address = sender.get("address", "")
				var short_address = (
					address.substr(address.length() - 4) if address.length() > 4 else address
				)
				return (
					TranslationServer.translate("NOTIF_TITLE_WANTS_TO_BE_YOUR_FRIEND_3")
					% [color_hex, sender_name, short_address]
				)
			return TranslationServer.translate("NOTIF_TITLE_WANTS_TO_BE_YOUR_FRIEND")

		"social_service_friendship_accepted":
			if "sender" in metadata and metadata["sender"] is Dictionary:
				var sender = metadata["sender"]
				var sender_name = sender.get("name", "Unknown")
				var has_claimed_name = sender.get("hasClaimedName", false)
				var color_hex = _get_avatar_color_hex(sender_name)

				if has_claimed_name:
					return (
						TranslationServer.translate("NOTIF_TITLE_ACCEPTED_YOUR_FRIEND_REQUEST_2")
						% [color_hex, sender_name]
					)

				var address = sender.get("address", "")
				var short_address = (
					address.substr(address.length() - 4) if address.length() > 4 else address
				)
				return (
					TranslationServer.translate("NOTIF_TITLE_ACCEPTED_YOUR_FRIEND_REQUEST_3")
					% [color_hex, sender_name, short_address]
				)
			return TranslationServer.translate("NOTIF_TITLE_ACCEPTED_YOUR_FRIEND_REQUEST")

		# Community notifications
		"community_invite_received":
			var community_name = metadata.get(
				"communityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return (
				TranslationServer.translate("NOTIF_TITLE_YOUVE_BEEN_INVITED_TO_JOIN_THE")
				% community_name
			)

		"community_member_banned":
			var community_name = metadata.get(
				"communityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return (
				TranslationServer.translate("NOTIF_TITLE_YOUVE_BEEN_BANNED_FROM_THE_COMMUNITY")
				% community_name
			)

		"community_member_removed":
			var community_name = metadata.get(
				"communityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return (
				TranslationServer.translate("NOTIF_TITLE_YOUVE_BEEN_REMOVED_FROM_THE_COMMUNITY")
				% community_name
			)

		"community_request_to_join_accepted":
			var community_name = metadata.get(
				"communityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return (
				TranslationServer.translate("NOTIF_TITLE_CONGRATS_YOURE_NOW_A_MEMBER_OF")
				% community_name
			)

		"community_user_request_to_join":
			var user_name = metadata.get("userName", "Someone")
			var community_name = metadata.get(
				"communityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return (
				TranslationServer.translate("NOTIF_TITLE_WANTS_TO_JOIN_THE_COMMUNITY")
				% [user_name, community_name]
			)

		"community_deleted":
			var community_name = metadata.get(
				"communityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return (
				TranslationServer.translate("NOTIF_TITLE_THE_COMMUNITY_HAS_BEEN_DELETED")
				% community_name
			)

		"community_deleted_content_violation":
			var community_name = metadata.get(
				"communityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return (
				TranslationServer.translate("NOTIF_TITLE_THE_COMMUNITY_WAS_DELETED_FOR_VIOLATING")
				% community_name
			)

		"event_created":
			var community_name = metadata.get(
				"communityName", TranslationServer.translate("NOTIF_TITLE_UNKNOWN_COMMUNITY")
			)
			return (
				TranslationServer.translate("NOTIF_TITLE_THE_COMMUNITY_HAS_ADDED_A_NEW")
				% community_name
			)

		"community_renamed":
			var old_name = metadata.get("oldCommunityName", "Unknown")
			var new_name = metadata.get("newCommunityName", "Unknown")
			return (
				"The [b][%s][/b] Community has been renamed to [b][%s][/b]." % [old_name, new_name]
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

		# Rewards
		"reward_assigned":
			return metadata.get(
				"description",
				TranslationServer.translate("NOTIF_TITLE_YOUVE_BEEN_ASSIGNED_A_REWARD")
			)
		"reward_in_progress":
			return metadata.get(
				"description", TranslationServer.translate("NOTIF_TITLE_YOUR_REWARD_IS_IN_PROGRESS")
			)

		_:
			return metadata.get("description", "")
