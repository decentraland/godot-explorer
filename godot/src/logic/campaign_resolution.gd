class_name CampaignResolution
extends RefCounted

## Pure resolution logic for ad/referrer campaigns (issues #2670 / #2669).
##
## A campaign is always a destination: an attributed install boots straight into its target
## and skips the FTUE. An install with no campaign — or one that does not resolve — gets
## today's FTUE, unchanged.
##
## Deliberately free of Global and of any node state so it can be exercised headless — see
## src/test/campaigns/test_campaigns_resolution.gd. The fetching, caching and token
## lifecycle live in campaigns.gd, which is the node that owns the IO.

const TARGET_GENESIS := "genesis"
const TARGET_WORLD := "world"

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
		return [Vector2i(int(parts[0]), int(parts[1])), String(DclUrls.main_realm())]

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
