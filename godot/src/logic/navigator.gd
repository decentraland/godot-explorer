class_name Navigator

## Executes a navigation intent (#2948).
##
## Resolve first, act second: nothing is offloaded and no loading screen goes up until the
## destination is known to be real. That ordering is the whole point -- it turns a black
## screen over a scene that was already discarded into a modal over the scene the player
## is still standing in.
##
## Every state the resolver can answer with leaves through here. A FAILED intent that
## showed nothing would be worse than the bug this replaces.


## Runs an intent end to end. `when` is the loading-funnel bucket ("on_teleport",
## "on_world", "on_reload"...) and keeps the existing vocabulary, which the dashboards
## index on. Returns true only when the player actually went somewhere.
static func async_go(dest: Destination, when: String) -> bool:
	# The funnel episode opens on the INTENT, not on the loading screen. A navigation
	# refused before the screen goes up still has to produce a row; today it produces
	# none at all, which is why a failed pre-fetch is invisible in the funnel.
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
			return _abandon("private_world_access_denied")
		Destination.State.NEEDS_PASSWORD:
			# Backed out of the password prompt: they stay exactly where they were.
			return _abandon("cancelled")
		_:
			Global.modal_manager.async_show_invalid_destination_modal(resolved.failure)
			Global.scene_runner.loading_realm_change_failed(
				resolved.realm_string, resolved.failure_reason()
			)
			return _abandon("resolve_failed")


static func _async_enter(dest: Destination, when: String) -> bool:
	var explorer = Global.get_explorer()
	if not is_instance_valid(explorer):
		return _abandon("no_explorer")

	# The episode is already open, so the screen must not start a second one -- that
	# would report this intent as superseded by its own loading screen.
	explorer.loading_ui.enable_loading_screen(dest.realm_string, when, false)
	# What the resolve already paid for, handed over before the load starts (#2698).
	explorer.loading_ui.set_prefetched_scene(
		dest.scene_title, dest.scene_image_url, dest.asset_count
	)
	_warm_caches(dest)
	# Behind the screen, never before it: closing the menu first would flash the world
	# the player is leaving.
	explorer.hide_menu()

	if _needs_realm_change(dest) and not await Global.realm.async_apply_destination(dest):
		explorer.loading_ui.hide_loading_screen("Failed")
		return false

	if dest.target_parcel != Destination.UNSPECIFIED:
		explorer.teleport_to(dest.target_parcel)
	return true


## Starts the downloads the load is about to ask for anyway.
##
## Deliberately after the verdict and never awaited: a destination that gets refused
## spends no bytes, and the window between here and the first asset request -- the realm
## commit, the world /scenes call, the coordinator's own radius fetch -- is where these
## land for free.
static func _warm_caches(dest: Destination) -> void:
	# The thumbnail is not warmed here: the loading screen asks for the same hash one frame
	# later, so there was no head start to gain. It is warmed from the jump-in panel, where
	# there are seconds to gain -- and the card itself has usually downloaded it already.
	_warm_boot_bundle(dest.scene_id if dest.kind == Destination.Kind.GENESIS else "")


## The scene's main.js and main.crdt arrive together in one {entity}-boot.zip, and
## async_load_scene checks for those files on disk before asking for it -- so starting it
## here turns that check into a hit a second later. A 404 is the ordinary "not optimized"
## answer and costs nothing.
##
## Only genesis: a world's scenes are not known until /about has been applied. Skipped
## wherever the loader would not use the bundle either, and in preview, where the first
## load purges the scene's files anyway.
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


## Warms a place while the player is still deciding, from the jump-in panel. By the time
## JUMP IN is pressed the scene's code is already on its way down.
##
## Genesis only: a world's scene ids come from its /scenes listing, which the resolve
## reads a moment later anyway. The access warm below is the one #1725 put here.
static func warm(parcel: Vector2i, realm: String) -> void:
	Global.warm_realm_access(realm)
	if parcel == Destination.UNSPECIFIED:
		return
	if not realm.is_empty() and not Realm.is_genesis_city(realm):
		return
	_async_warm_scene(parcel)


static func _async_warm_scene(parcel: Vector2i) -> void:
	_warm_boot_bundle(await Global.async_resolve_scene_entity_id(parcel))


## Only somewhere else needs a realm change -- or a reload, which is the same realm on
## purpose. Compares resolved urls rather than raw strings: "SpaceRunner.dcl.eth" and
## "spacerunner.dcl.eth" are one destination (#2816).
static func _needs_realm_change(dest: Destination) -> bool:
	if not dest.is_intent:
		return true
	return dest.realm_url != Global.realm.get_realm_url()


static func _world_name(dest: Destination) -> String:
	return WorldPermissionsHelper.world_name_from_realm(
		dest.realm_string, Realm.resolve_realm_url(dest.realm_string)
	)


## One line per navigation, so a device log says which intent ran and how it ended. The
## string entry point used to print this; the destination path lost it, and a navigation
## that leaves no trace is one nobody can verify on device.
static func _log_verdict(dest: Destination, elapsed_ms: int) -> void:
	var detail := ""
	match dest.state:
		Destination.State.READY:
			detail = 'assets=%d title="%s"' % [dest.asset_count, dest.scene_title]
		Destination.State.FAILED:
			detail = dest.failure_reason()
	var state_name := str(Destination.State.keys()[dest.state]).to_lower()
	prints("[NAV] %s #%d in %dms" % [state_name, dest.intent_id, elapsed_ms], detail)


## Closes the funnel episode for an intent that never became a load.
static func _abandon(reason: String) -> bool:
	Global.scene_runner.loading_end_episode(reason)
	return false
