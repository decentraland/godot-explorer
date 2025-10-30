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
	var color = player_avatar.get_nickname_color(username)
	# Return as hex string without #
	return color.to_html(false)


## Get the header/title for a notification based on its type
static func get_notification_header(notif_type: String, metadata: Dictionary) -> String:
	match notif_type:
		# Friend notifications
		"social_service_friendship_request":
			return "Friend Request Received"
		"social_service_friendship_accepted":
			return "Friend Request Accepted!"

		# Community notifications
		"community_invite_received":
			return "Community Invite Received"
		"community_user_banned":
			return "Banned From Community"
		"community_user_removed":
			return "Removed from Community"
		"community_user_request_to_join_accepted":
			return "Membership Request Accepted"
		"community_user_request_to_join":
			return "Membership Request Received"
		"community_deleted":
			return "Community Deleted"
		"community_deleted_content_violation":
			return "Your Community Has Been Deleted"
		"community_event_created":
			return "Community Event Added"
		"community_renamed":
			return "Community Renamed"

		# Badge notifications - use metadata title
		"badge_granted":
			return metadata.get("title", "New Badge Unlocked!")

		# Marketplace/Credits - use metadata title
		"credits_reminder_do_not_miss_out":
			return "Don't Miss Out!"
		"item_sold", "bid_accepted", "bid_received", "royalties_earned":
			return metadata.get("title", "Notification")

		# Governance
		"governance_announcement":
			return "DAO Announcement"
		"governance_proposal_enacted":
			return "Proposal Enacted"
		"governance_voting_ended":
			return "Voting Ended"
		"governance_coauthor_requested":
			return "Co-Author Requested"

		# Land
		"land":
			return "Land Update"

		# Worlds
		"worlds_access_restored":
			return "World Access Restored"
		"worlds_access_restricted":
			return "World Access Restricted"
		"worlds_missing_resources":
			return "World Missing Resources"
		"worlds_permission_granted":
			return "World Permission Granted"
		"worlds_permission_revoked":
			return "World Permission Revoked"

		# Events
		"event_started":
			return "Event Started"
		"event_ended":
			return "Event Ended"

		# Rewards
		"reward_assigned":
			return "Reward Assigned"
		"reward_in_progress":
			return "Reward In Progress"

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
						"[color=#%s]%s [color=#ECEBED]wants to be your friend!"
						% [color_hex, sender_name]
					)

				var address = sender.get("address", "")
				var short_address = (
					address.substr(address.length() - 4) if address.length() > 4 else address
				)
				return (
					"[color=#%s]%s[color=#A09BA8]#%s [color=#ECEBED]wants to be your friend!"
					% [color_hex, sender_name, short_address]
				)
			return "wants to be your friend!"

		"social_service_friendship_accepted":
			if "sender" in metadata and metadata["sender"] is Dictionary:
				var sender = metadata["sender"]
				var sender_name = sender.get("name", "Unknown")
				var has_claimed_name = sender.get("hasClaimedName", false)
				var color_hex = _get_avatar_color_hex(sender_name)

				if has_claimed_name:
					return (
						"[color=#%s]%s [color=#ECEBED]accepted your friend request."
						% [color_hex, sender_name]
					)

				var address = sender.get("address", "")
				var short_address = (
					address.substr(address.length() - 4) if address.length() > 4 else address
				)
				return (
					"[color=#%s]%s[color=#A09BA8]#%s [color=#ECEBED]accepted your friend request."
					% [color_hex, sender_name, short_address]
				)
			return "accepted your friend request."

		# Community notifications
		"community_invite_received":
			var community_name = metadata.get("communityName", "Unknown Community")
			return "You've been invited to join the [b][%s][/b] Community." % community_name

		"community_user_banned":
			var community_name = metadata.get("communityName", "Unknown Community")
			return "You've been banned from the [b][%s][/b] Community." % community_name

		"community_user_removed":
			var community_name = metadata.get("communityName", "Unknown Community")
			return "You've been removed from the [b][%s][/b] Community." % community_name

		"community_user_request_to_join_accepted":
			var community_name = metadata.get("communityName", "Unknown Community")
			return "Congrats! You're now a member of the [b][%s][/b] Community." % community_name

		"community_user_request_to_join":
			var user_name = metadata.get("userName", "Someone")
			var community_name = metadata.get("communityName", "Unknown Community")
			return "[b]%s[/b] wants to join the [b]%s[/b] Community." % [user_name, community_name]

		"community_deleted":
			var community_name = metadata.get("communityName", "Unknown Community")
			return "The [b][%s][/b] Community has been deleted." % community_name

		"community_deleted_content_violation":
			var community_name = metadata.get("communityName", "Unknown Community")
			return (
				"The [b][%s][/b] Community was deleted for violating Decentraland's Guidelines."
				% community_name
			)

		"community_event_created":
			var community_name = metadata.get("communityName", "Unknown Community")
			return "The [b][%s][/b] Community has added a new event." % community_name

		"community_renamed":
			var old_name = metadata.get("oldCommunityName", "Unknown")
			var new_name = metadata.get("newCommunityName", "Unknown")
			return (
				"The [b][%s][/b] Community has been renamed to [b][%s][/b]." % [old_name, new_name]
			)

		# Badge notifications - use metadata description
		"badge_granted":
			return metadata.get("description", "You've unlocked a new badge!")

		# Marketplace/Credits - use metadata description
		"credits_reminder_do_not_miss_out":
			return "Explore Decentraland and earn rewards!"
		"item_sold", "bid_accepted", "bid_received", "royalties_earned":
			return metadata.get("description", "")

		# Governance
		"governance_announcement":
			return metadata.get("description", "New governance announcement")
		"governance_proposal_enacted":
			return metadata.get("description", "A proposal has been enacted")
		"governance_voting_ended":
			return metadata.get("description", "Voting has ended")
		"governance_coauthor_requested":
			return metadata.get("description", "You've been requested as co-author")

		# Land
		"land":
			return metadata.get("description", "Land update notification")

		# Worlds
		"worlds_access_restored":
			return metadata.get("description", "Your world access has been restored")
		"worlds_access_restricted":
			return metadata.get("description", "Your world access has been restricted")
		"worlds_missing_resources":
			return metadata.get("description", "Your world is missing resources")
		"worlds_permission_granted":
			return metadata.get("description", "World permission granted")
		"worlds_permission_revoked":
			return metadata.get("description", "World permission revoked")

		# Events
		"event_started":
			return metadata.get("description", "An event has started")
		"event_ended":
			return metadata.get("description", "An event has ended")

		# Rewards
		"reward_assigned":
			return metadata.get("description", "You've been assigned a reward")
		"reward_in_progress":
			return metadata.get("description", "Your reward is in progress")

		_:
			return metadata.get("description", "")
