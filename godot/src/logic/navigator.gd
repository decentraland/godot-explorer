class_name Navigator

## Executes a navigation intent (#2948).
##
## Resolve first, act second: nothing is offloaded until the destination is known to be
## real, which turns a black screen over a discarded scene into a modal over the one the
## player is still standing in. Every resolver state leaves through here.


## `when` is the loading-funnel bucket the dashboards index on. Returns true only when
## the player actually went somewhere.
static func async_go(dest: Destination, when: String) -> bool:
	# On the INTENT, not the loading screen: a navigation refused before the screen goes
	# up still has to produce a funnel row.
	Global.scene_runner.loading_begin_episode(when, dest.realm_string)
	var started := Time.get_ticks_msec()
	prints("[NAV] intent", dest, when)

	var resolved := await DestinationResolver.async_resolve(dest)
	if resolved.state == Destination.State.NEEDS_PASSWORD:
		prints("[NAV] password needed #%d" % resolved.intent_id)
		var prompt := WorldSecretPrompt.new(resolved)
		resolved = await prompt.async_run()
	_log_verdict(resolved, Time.get_ticks_msec() - started)

	match resolved.state:
		Destination.State.READY:
			return await _async_enter(resolved, when)
		Destination.State.NOT_ALLOWED:
			Global.modal_manager.async_show_private_world_modal(_world_name(resolved))
			recover_from_refusal(resolved.realm_string)
			return _abandon("private_world_access_denied")
		Destination.State.NEEDS_PASSWORD:
			# Backed out of the password prompt: they stay exactly where they were.
			return _abandon("cancelled")
		_:
			Global.modal_manager.async_show_invalid_destination_modal(resolved.failure)
			Global.scene_runner.loading_realm_change_failed(
				resolved.realm_string, resolved.failure_reason()
			)
			recover_from_refusal(resolved.realm_string)
			return _abandon("resolve_failed")


## Undo what a refusal would otherwise leave behind (#1725). Two separate leaks: a refused
## realm kept as the boot realm reopens the same modal on every launch, and a boot that never
## committed a realm leaves nothing behind the modal to dismiss back to.
static func recover_from_refusal(realm_string: String) -> void:
	# Still standing somewhere: the refusal cost nothing and the saved realm is still good.
	# Only a refusal that left us with no realm at all is the one that repeats on every boot.
	if is_instance_valid(Global.realm) and Global.realm.has_realm():
		return
	_clear_boot_realm_if(realm_string)
	# Deferred: the modal is going up on this frame, and this re-enters async_go.
	if is_instance_valid(Global.get_explorer()):
		Navigator.async_go.call_deferred(
			Destination.restore(DclUrls.main_realm()), "on_explorer_ready"
		)


## Compares canonical urls, not world names: a dead preview server is just as capable of
## being the saved boot realm as a world that revoked access.
static func _clear_boot_realm_if(realm_string: String) -> void:
	if realm_string.is_empty():
		return
	var config = Global.get_config()
	var stored: String = config.last_realm_joined
	if stored.is_empty():
		return
	if Realm.normalize_realm_url(stored) != Realm.normalize_realm_url(realm_string):
		return
	config.last_realm_joined = DclUrls.main_realm()
	config.save_to_settings_file()


static func _async_enter(dest: Destination, when: String) -> bool:
	var explorer = Global.get_explorer()
	if not is_instance_valid(explorer):
		return _abandon("no_explorer")

	# The episode is already open; a second one would report this intent as superseded
	# by its own loading screen.
	explorer.loading_ui.enable_loading_screen(dest.realm_string, when, false)
	explorer.loading_ui.set_prefetched_scene(
		dest.scene_title, dest.scene_creator, dest.scene_image_url, dest.asset_count
	)
	_warm_caches(dest)
	# Behind the screen: closing the menu first flashes the world being left.
	explorer.hide_menu()

	if _needs_realm_change(dest) and not await Global.realm.async_apply_destination(dest):
		explorer.loading_ui.hide_loading_screen("Failed")
		return false

	# A restoration carries its parcel only so the resolve can describe it; _ready
	# already put the player there.
	if dest.is_intent and dest.target_parcel != Destination.UNSPECIFIED:
		explorer.teleport_to(dest.target_parcel)
	return true


## Starts the downloads the load will ask for anyway. After the verdict and never
## awaited, so a refused destination spends no bytes.
static func _warm_caches(dest: Destination) -> void:
	# Not the thumbnail: the screen asks for the same hash one frame later, so there is
	# no head start here. The jump-in panel warms it, where there are seconds to gain.
	_warm_boot_bundle(dest.scene_id if dest.kind == Destination.Kind.GENESIS else "")


## main.js and main.crdt arrive together in one {entity}-boot.zip, and async_load_scene
## checks disk first -- so fetching here turns that check into a hit a second later. A
## 404 is the ordinary "not optimized" answer. Genesis only, and never in preview.
static func _warm_boot_bundle(scene_id: String) -> void:
	if scene_id.is_empty():
		return
	if Global.cli.preview_mode or not Global.deep_link_obj.preview.is_empty():
		return
	if Global.is_xr() or Global.get_testing_scene_mode() or Global.cli.only_no_optimized:
		return

	var boot_zip := "%s-boot.zip" % scene_id
	var base: String = Global.content_provider.get_optimized_base_url()
	Global.content_provider.fetch_boot_bundle(boot_zip, "%s/%s" % [base, boot_zip])


## From the jump-in panel, while the player is still deciding: by the time JUMP IN is
## pressed the scene's code is on its way down. Genesis only -- a world's scene ids come
## from the listing the resolve reads a moment later anyway.
static func warm(parcel: Vector2i, realm: String) -> void:
	Global.warm_realm_access(realm)
	if parcel == Destination.UNSPECIFIED:
		return
	if not realm.is_empty() and not Realm.is_genesis_city(realm):
		return
	_async_warm_scene(parcel)


static func _async_warm_scene(parcel: Vector2i) -> void:
	_warm_boot_bundle(await Global.async_resolve_scene_entity_id(parcel))


## Compares resolved urls, not raw strings: "SpaceRunner.dcl.eth" and
## "spacerunner.dcl.eth" are one destination (#2816).
static func _needs_realm_change(dest: Destination) -> bool:
	if not dest.is_intent:
		return true
	return dest.realm_url != Global.realm.get_realm_url()


static func _world_name(dest: Destination) -> String:
	return WorldPermissionsHelper.world_name_from_realm(
		dest.realm_string, Realm.resolve_realm_url(dest.realm_string)
	)


## One line per navigation, so a device log can say which intent ran and how it ended.
static func _log_verdict(dest: Destination, elapsed_ms: int) -> void:
	var detail := ""
	match dest.state:
		Destination.State.READY:
			detail = 'assets=%d title="%s"' % [dest.asset_count, dest.scene_title]
		Destination.State.FAILED:
			detail = dest.failure_reason()
	var state_name := str(Destination.State.keys()[dest.state]).to_lower()
	prints("[NAV] %s #%d in %dms" % [state_name, dest.intent_id, elapsed_ms], detail)


static func _abandon(reason: String) -> bool:
	Global.scene_runner.loading_end_episode(reason)
	return false
