class_name Destination
extends RefCounted

## Where the player asked to go, and whether that place is real. Only a READY one may
## start a load.
##
## Copied, never mutated: the resolver returns a new Destination per transition, so a
## password retry produces several while `intent_id` keys one analytics episode.

enum Kind { GENESIS, WORLD, PREVIEW, CUSTOM_REALM }

enum State { RESOLVING, READY, NEEDS_PASSWORD, NOT_ALLOWED, FAILED }

enum Failure {
	NONE,
	WORLD_NOT_FOUND,
	PARCEL_OUT_OF_BOUNDS,
	PARCEL_EMPTY,
	FETCH_FAILED,
	OFFLINE,
	PREVIEW_UNREACHABLE,
	TIMEOUT,
	RATE_LIMITED,
}

## No parcel requested: the destination picks the landing spot. Same sentinel
## `deep_link_obj.location` uses, so a deeplink needs no translation.
const UNSPECIFIED := Vector2i.MAX

var kind: Kind = Kind.GENESIS
var realm_string: String = ""
var target_parcel: Vector2i = UNSPECIFIED

## An intent is gated: an unreal destination is refused before the current scene is
## offloaded. A restoration is not -- there is no scene to protect and nothing to
## cancel back to, so an empty parcel lands on grass.
var is_intent: bool = true

## A coordinate given with no realm: retried against Genesis City before being refused.
var fallback_to_genesis: bool = false

var state: State = State.RESOLVING
var failure: Failure = Failure.NONE
## Credential attempts spent. Survives copies, like intent_id.
var attempt: int = 0

## Written by the resolver and handed to Realm, so a navigation costs one /about.
var realm_url: String = ""
var about: Dictionary = {}
## Handed to Realm, so applying the destination does not fetch the listing again.
var world_scenes: Dictionary = {}
var credential: String = ""

## What the loading screen shows before the load starts (#2698), from metadata the
## existence check already paid for. Empty is a normal READY, and Places fills the gaps.
var scene_id: String = ""
var scene_title: String = ""
var scene_creator: String = ""
var scene_image_url: String = ""
var asset_count: int = 0

## One intent_id is one loading-funnel episode, credential retries included.
var intent_id: int = 0

static var _next_intent_id: int = 0


static func genesis(parcel: Vector2i, realm: String = "") -> Destination:
	return _make(Kind.GENESIS, realm if not realm.is_empty() else DclUrls.main_realm(), parcel)


static func world(name_or_url: String, parcel: Vector2i = UNSPECIFIED) -> Destination:
	return _make(Kind.WORLD, name_or_url, parcel)


static func preview(server_url: String, parcel: Vector2i = UNSPECIFIED) -> Destination:
	return _make(Kind.PREVIEW, server_url, parcel)


static func custom_realm(url: String, parcel: Vector2i = UNSPECIFIED) -> Destination:
	return _make(Kind.CUSTOM_REALM, url, parcel)


## The one adapter for untrusted realm strings: deeplinks, chat commands, teleportTo.
##
## A creator can legitimately say "go to 5,5" inside their world, but coordinates are
## Genesis City's addressing scheme, so a world without that parcel hands the question
## on rather than refusing it.
static func from_input(realm: String, parcel: Vector2i = UNSPECIFIED) -> Destination:
	if realm.is_empty():
		if parcel == UNSPECIFIED:
			return current()
		var bare := current(parcel)
		bare.fallback_to_genesis = true
		return bare
	if Realm.is_local_preview(realm):
		return preview(realm, parcel)
	var world_name := WorldPermissionsHelper.world_name_from_realm(
		realm, Realm.resolve_realm_url(realm)
	)
	if not world_name.is_empty():
		return world(realm, parcel)
	if Realm.is_genesis_city(realm):
		return genesis(parcel, realm)
	return custom_realm(realm, parcel)


## Falls back to genesis before a realm is set.
static func current(parcel: Vector2i = UNSPECIFIED) -> Destination:
	var realm := ""
	if is_instance_valid(Global.realm):
		realm = Global.realm.get_realm_string()
	if realm.is_empty():
		return genesis(parcel)
	return from_input(realm, parcel)


## Resuming a session, or a cold-start deeplink. Not an intent, so the parcel gate is
## off -- but /about and access still run, so an unreachable world is still refused.
static func restore(realm: String, parcel: Vector2i = UNSPECIFIED) -> Destination:
	var dest := from_input(realm, parcel)
	dest.is_intent = false
	return dest


## /reload, scene-crash retry, loading-timeout retry. Not an intent: re-checking a place
## we are already inside would refuse to reload a parcel the player walked to.
static func reload_current() -> Destination:
	var dest := current()
	dest.is_intent = false
	return dest


static func _make(new_kind: Kind, realm: String, parcel: Vector2i) -> Destination:
	var dest := Destination.new()
	dest.kind = new_kind
	dest.realm_string = realm
	dest.target_parcel = parcel
	_next_intent_id += 1
	dest.intent_id = _next_intent_id
	return dest


## `scene` is what DestinationResolver.scene_info() produced, or {}.
func resolved_ready(
	new_about: Dictionary = {}, scene: Dictionary = {}, scenes: Dictionary = {}
) -> Destination:
	var dest := _copy()
	dest.state = State.READY
	dest.failure = Failure.NONE
	dest.about = new_about
	dest.world_scenes = scenes
	dest.scene_id = str(scene.get("id", ""))
	dest.scene_title = str(scene.get("title", ""))
	dest.scene_creator = str(scene.get("creator", ""))
	dest.scene_image_url = str(scene.get("image_url", ""))
	dest.asset_count = int(scene.get("asset_count", 0))
	dest.realm_url = Realm.normalize_realm_url(realm_string)
	return dest


## The same intent against Genesis City. Keeps intent_id: one navigation, one episode.
func as_genesis() -> Destination:
	var dest := _copy()
	dest.kind = Kind.GENESIS
	dest.realm_string = DclUrls.main_realm()
	dest.realm_url = ""
	dest.state = State.RESOLVING
	dest.failure = Failure.NONE
	dest.fallback_to_genesis = false
	return dest


func resolved_failed(reason: Failure) -> Destination:
	var dest := _copy()
	dest.state = State.FAILED
	dest.failure = reason
	return dest


func resolved_needs_password() -> Destination:
	var dest := _copy()
	dest.state = State.NEEDS_PASSWORD
	return dest


func resolved_not_allowed() -> Destination:
	var dest := _copy()
	dest.state = State.NOT_ALLOWED
	return dest


## Same intent, new resolve, carrying the secret the user just typed.
func with_credential(secret: String) -> Destination:
	var dest := _copy()
	dest.state = State.RESOLVING
	dest.failure = Failure.NONE
	dest.credential = secret
	dest.attempt = attempt + 1
	return dest


func is_ready() -> bool:
	return state == State.READY


func is_terminal() -> bool:
	return state != State.RESOLVING


## The vocabulary the funnel stores as `reason` and the modal reports as
## `failure_reason` (#2937).
func failure_reason() -> String:
	return Destination.failure_name(failure)


static func failure_name(reason: Failure) -> String:
	return str(Failure.keys()[reason]).to_lower()


func _copy() -> Destination:
	var dest := Destination.new()
	dest.kind = kind
	dest.realm_string = realm_string
	dest.target_parcel = target_parcel
	dest.is_intent = is_intent
	dest.fallback_to_genesis = fallback_to_genesis
	dest.state = state
	dest.failure = failure
	dest.attempt = attempt
	dest.realm_url = realm_url
	dest.about = about
	dest.world_scenes = world_scenes
	dest.credential = credential
	dest.scene_id = scene_id
	dest.scene_title = scene_title
	dest.scene_creator = scene_creator
	dest.scene_image_url = scene_image_url
	dest.asset_count = asset_count
	dest.intent_id = intent_id
	return dest


func _to_string() -> String:
	return (
		"Destination(#%d %s %s %s %s)"
		% [
			intent_id,
			Kind.keys()[kind],
			realm_string,
			target_parcel,
			State.keys()[state],
		]
	)
