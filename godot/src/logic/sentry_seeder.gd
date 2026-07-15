class_name SentrySeeder
extends RefCounted

## Seeds Sentry user/context/tag state from Decentraland runtime signals.
## RefCounted, kept alive by Global's strong reference; signal connections
## do not own RefCounted targets on their own. No scene-tree presence.
##
## Sentry SDK init + static (process-lifetime) tags live in
## project_main_loop.gd — this controller only handles the dynamic state
## that depends on Global subsystems being ready.


## Called by Global from _ready() after realm / scene_fetcher /
## player_identity / comms are constructed and `session_id` is assigned.
func setup() -> void:
	var sentry_user := SentryUser.new()
	sentry_user.id = Global.config.analytics_user_id
	SentrySDK.set_user(sentry_user)
	SentrySDK.set_tag("dcl_session_id", Global.session_id)
	# Refreshed by _on_wallet_connected once auth fires.
	SentrySDK.set_tag("is_guest", "true")

	Global.realm.realm_changed.connect(_on_realm_changed)
	Global.scene_fetcher.player_parcel_changed.connect(_on_parcel_changed)
	Global.comms.on_adapter_changed.connect(_on_adapter_changed)
	Global.player_identity.wallet_connected.connect(_on_wallet_connected)
	Global.player_identity.profile_changed.connect(_on_profile_changed)
	Global.player_identity.logout.connect(_on_logout)


func _on_realm_changed() -> void:
	var realm_ctx := {
		"name": Global.realm.realm_name,
		"url": Global.realm.realm_url,
		"network_id": Global.realm.network_id,
		"content_base_url": Global.realm.content_base_url,
	}
	SentrySDK.set_context("realm", realm_ctx)
	SentrySDK.set_tag("realm", Global.realm.realm_name)


func _on_parcel_changed(new_position: Vector2i) -> void:
	var location_ctx := {
		"parcel": "%d,%d" % [new_position.x, new_position.y],
		"scene_entity_id": Global.scene_fetcher.current_scene_entity_id,
	}
	SentrySDK.set_context("location", location_ctx)


func _on_adapter_changed(_voice_chat_enabled: bool, new_adapter: String) -> void:
	SentrySDK.set_tag("comms_adapter", new_adapter)


# Keep user.id pinned to analytics_user_id across auth changes so
# "Users affected" stays attributed to a single install. Only username
# tracks the wallet/profile state.
func _on_wallet_connected(address: String, _chain_id: int, is_guest_value: bool) -> void:
	var sentry_user := SentryUser.new()
	sentry_user.id = Global.config.analytics_user_id
	if not is_guest_value:
		sentry_user.username = address
	SentrySDK.set_user(sentry_user)
	SentrySDK.set_tag("is_guest", "true" if is_guest_value else "false")


func _on_profile_changed(new_profile: DclUserProfile) -> void:
	var sentry_user := SentryUser.new()
	sentry_user.id = Global.config.analytics_user_id
	if new_profile != null:
		var display := new_profile.get_name()
		if not display.is_empty():
			sentry_user.username = display
	SentrySDK.set_user(sentry_user)


func _on_logout() -> void:
	var sentry_user := SentryUser.new()
	sentry_user.id = Global.config.analytics_user_id
	SentrySDK.set_user(sentry_user)
	SentrySDK.set_tag("is_guest", "true")
