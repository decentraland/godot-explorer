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


## Mirrors the token rule the mobile-bff enforces on save (kebab-case, max 64), and the one
## the Rust attribution path applies. The deeplink capture path needs it too: a token stored
## from a link is never replaced, so an unvalidated junk `?c=` would permanently block the
## real install-attribution token on that install.
static func is_valid_token(token: String) -> bool:
	# Equivalent to the BFF's ^[a-z0-9]+(-[a-z0-9]+)*$ and to is_valid_token in
	# lib/src/analytics/install_attribution.rs, written out rather than as a regex so the
	# three copies of this rule read the same way.
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
		# int() answers 0 for anything it cannot parse, so an unparseable position would read
		# as the perfectly valid parcel 0,0: the boot would "succeed", the token would be
		# consumed, and every install of that campaign would land on the Genesis spawn.
		if not x_raw.is_valid_int() or not y_raw.is_valid_int():
			return []
		# is_valid_int() only checks the characters, so a value too large for the int32 in
		# Vector2i still truncates — "4294967296" lands on 0,0 exactly like the case above.
		# The bound matches the 1-4 digits the BFF regex and the DB CHECK both enforce, so a
		# position that cannot have been stored is not routed either.
		var x := int(x_raw)
		var y := int(y_raw)
		if absi(x) > POSITION_ABS_MAX or absi(y) > POSITION_ABS_MAX:
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
