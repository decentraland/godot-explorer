class_name CampaignResolution
extends RefCounted

## Pure resolution logic for ad/referrer campaigns (issues #2670 / #2669). Free of Global and
## of node state so it can be exercised headless — see src/test/campaigns/. The fetching,
## caching and token lifecycle live in campaigns.gd, which owns the IO.

const TARGET_GENESIS := "genesis"
const TARGET_WORLD := "world"
const TOKEN_MAX_LENGTH := 64
## Matches POSITION_ABS_MAX in mobile-bff and the campaigns_position_format CHECK: at most
## four digits per coordinate. Also keeps the parsed value inside the int32 Vector2i holds.
const POSITION_ABS_MAX := 9999

# Why a resolve fell back to the default FTUE. Shipped on the existing FTUE metrics so
# personalized-vs-default can be compared without a new pipeline.
const FALLBACK_NONE := ""
const FALLBACK_NO_TOKEN := "no_token"
const FALLBACK_UNKNOWN_TOKEN := "unknown_token"
const FALLBACK_EXPIRED_TOKEN := "expired_token"
const FALLBACK_ALREADY_CONSUMED := "already_consumed"
const FALLBACK_RESOLVER_UNAVAILABLE := "resolver_unavailable"
## The campaign resolved but its target cannot be turned into a destination.
const FALLBACK_UNUSABLE_TARGET := "unusable_target"
## Routable target, but the boot did not happen: the pre-boot gate sent the user to Discover
## (private world), or a redirect was already in flight. Distinct from `unusable_target` so
## the metric does not blame campaign data for a routing decision.
const FALLBACK_BOOT_DECLINED := "boot_declined"


## Extracts the campaign map from the mobile-bff response:
## `{"ok": true, "data": {"campaigns": {"<token>": {...}}}}`.
## Returns an empty dictionary on any shape mismatch (fail-open).
static func parse_response(json) -> Dictionary:
	if typeof(json) != TYPE_DICTIONARY or not json.get("ok", false):
		return {}
	var data = json.get("data", {})
	if typeof(data) != TYPE_DICTIONARY:
		return {}
	var campaigns = data.get("campaigns", {})
	if typeof(campaigns) != TYPE_DICTIONARY:
		return {}
	return campaigns


## The BFF's ^[a-z0-9]+(-[a-z0-9]+)*$, written out rather than as a regex so this and its two
## copies (the BFF, install_attribution.rs) read the same way.
static func is_valid_token(token: String) -> bool:
	if token.is_empty() or token.length() > TOKEN_MAX_LENGTH:
		return false
	if token.begins_with("-") or token.ends_with("-") or token.contains("--"):
		return false
	for c in token:
		if not ((c >= "a" and c <= "z") or (c >= "0" and c <= "9") or c == "-"):
			return false
	return true


## Reads a campaign's target as [position, realm], the pair Global.async_teleport_to and the
## FTUE jump signals already speak. Returns [] when the target is unusable, which callers
## treat as "render the default FTUE" rather than stranding the user somewhere.
static func target_position_and_realm(campaign: Dictionary) -> Array:
	var target = campaign.get("target")
	if typeof(target) != TYPE_DICTIONARY:
		return []

	var target_type := String(target.get("type", ""))

	if target_type == TARGET_WORLD:
		var world_name := String(target.get("name", ""))
		# A name Realm.is_dcl_ens rejects is treated as a realm URL further down the teleport
		# path and never reaches the intended world, so it does not resolve at all here.
		if not Realm.is_dcl_ens(world_name):
			return []
		return [Vector2i.ZERO, world_name]

	if target_type == TARGET_GENESIS:
		var parts := String(target.get("position", "")).split(",")
		if parts.size() != 2:
			return []
		var x_raw := parts[0].strip_edges()
		var y_raw := parts[1].strip_edges()
		# Both guards exist to keep a junk position from reading as the valid parcel 0,0 — the
		# boot would "succeed" and burn the token on the Genesis spawn. int() answers 0 for
		# what it cannot parse; is_valid_int() only checks characters, so a value too large for
		# Vector2i's int32 truncates to 0 too.
		if not x_raw.is_valid_int() or not y_raw.is_valid_int():
			return []
		var x := int(x_raw)
		var y := int(y_raw)
		# Bounded without absi(): absi(INT64_MIN) is INT64_MIN, which would pass the bound and
		# then truncate to 0. to_int() wraps "9223372036854775808" to INT64_MIN, so it is
		# reachable input, not a curiosity.
		if x < -POSITION_ABS_MAX or x > POSITION_ABS_MAX:
			return []
		if y < -POSITION_ABS_MAX or y > POSITION_ABS_MAX:
			return []
		return [Vector2i(x, y), String(DclUrls.main_realm())]

	return []


static func is_world_target(campaign: Dictionary) -> bool:
	var target = campaign.get("target")
	if typeof(target) != TYPE_DICTIONARY:
		return false
	return String(target.get("type", "")) == TARGET_WORLD


## Metrics payload describing how this launch resolved. Merged into the existing FTUE
## screen-viewed / jump-in events rather than shipping a new event, so a launch that fell
## back to the default FTUE says which failure put it there.
static func metrics_context(resolution: Dictionary) -> Dictionary:
	var campaign = resolution.get("campaign", {})
	if typeof(campaign) != TYPE_DICTIONARY:
		campaign = {}
	return {
		"campaign_token": resolution.get("token", ""),
		"campaign_resolved": not campaign.is_empty(),
		"campaign_fallback_reason": resolution.get("fallback_reason", FALLBACK_NO_TOKEN),
	}
