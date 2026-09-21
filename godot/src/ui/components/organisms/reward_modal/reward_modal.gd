class_name RewardModal
extends ColorRect

## Two-minute window (in seconds) used to tell a fresh claim apart from an item the
## wallet already owned, mirroring Genesis' claim.ts `assigned_at` check.
const ALREADY_CLAIMED_WINDOW_SEC := 120.0

var _campaign_id: String = ""
var _campaign_key: String = ""
var _claiming: bool = false
var _claimed: bool = false

@onready var button_claim: Button = %Button_Claim
@onready
var animation_player: AnimationPlayer = $VBoxContainer/PanelContainer/MarginContainer_Reward/AnimationPlayer
@onready var texture_rect_reward: TextureRect = %TextureRect_Reward
@onready var label_text: Label = %Label_Text
@onready var loading_spinner: Control = %LoadingSpinner
@onready var label_tap_to_continue: Label = %Label_TapToContinue


## Configures the modal for a campaign and reveals it.
## @param campaign: { campaign_id, campaign_key } — see RewardCampaigns.CAMPAIGNS.
func async_setup(campaign: Dictionary) -> void:
	_campaign_id = str(campaign.get("campaign_id", ""))
	_campaign_key = str(campaign.get("campaign_key", ""))
	_claiming = false
	_claimed = false
	# The reward art is a FIXED baked asset (dog-reward.png in the scene) — intentionally NOT
	# fetched from the campaign urn nor from the claim response, so the modal shows the SAME image
	# whether or not the item is already in the wallet. Just reveal it (kept async for the caller's
	# await contract; no network wait here anymore).
	show()


func _ready() -> void:
	# Label_Text never auto-translates (its error state carries server text), and
	# _show_success() deliberately leaves this copy in place, so seed it here.
	label_text.text = tr("REWARD_MODAL_YOUR_EMAIL_HAS_BEEN_VERIFIED_ENJOY")


func _on_gui_input(event: InputEvent) -> void:
	# Never dismiss while a claim request is in flight.
	if _claiming:
		return
	if event is InputEventScreenTouch:
		if event.pressed:
			Global.modal_manager.close_reward_modal()


func _on_button_claim_pressed() -> void:
	_async_claim()


## Reduces a URL to its origin (scheme://host), dropping any path/query. The rewards server's
## catalyst allow-list matches by origin, so a trailing "/main/" or "/content/" gets rejected.
## Returns "" for an empty input so the caller's fallback chain keeps working.
func _catalyst_origin(url: String) -> String:
	if url.is_empty():
		return ""
	var scheme := "https://"
	var rest := url
	if url.begins_with("https://"):
		rest = url.substr(scheme.length())
	elif url.begins_with("http://"):
		scheme = "http://"
		rest = url.substr(scheme.length())
	var slash := rest.find("/")
	if slash != -1:
		rest = rest.substr(0, slash)
	if rest.is_empty():
		return ""
	return scheme + rest


## Performs the real claim (client-side equivalent of Genesis' claimToken/requestToken):
## a signed POST to the rewards server that assigns the wearable to the player's wallet.
func _async_claim() -> void:
	if _claiming or _claimed:
		return

	# Rewards need a wallet address to sign the request and receive the item. An upgraded
	# thirdweb guest keeps is_guest = true (immutable), so we must NOT gate on is_guest here
	# — a non-empty address plus the signed fetch is what actually matters.
	var beneficiary := Global.player_identity.get_address_str()
	if beneficiary.is_empty():
		_show_error(tr("REWARD_ERROR_WALLET_REQUIRED"))
		return

	if _campaign_id.is_empty() or _campaign_key.is_empty():
		_show_error(tr("REWARD_NOT_AVAILABLE"))
		return

	_start_processing()

	# The rewards server matches `catalyst` by ORIGIN (scheme://host) against the DAO catalyst
	# allow-list AND then actually reaches it, so we must send the origin of a REAL, reachable
	# catalyst peer — NOT the realm-provider load balancer:
	#   - content_base_url = /about content.publicUrl → the resolved peer we're connected to
	#     (e.g. https://peer-ec1.decentraland.org/content/ → https://peer-ec1.decentraland.org ✓).
	#     Genesis' claim.ts sends realmInfo.baseUrl, which is likewise the resolved catalyst.
	#   - fallback (claiming outside a realm — post-upgrade from Discover / the dev-tools button):
	#     a concrete DAO catalyst peer. We must NOT fall back to main_realm(): its origin
	#     (realm-provider-ea…) passes the allow-list but is "catalyst_unreachable" at the bare
	#     origin — the load balancer only responds under a /main path we can't include here.
	var catalyst := _catalyst_origin(str(Global.realm.content_base_url))
	if catalyst.is_empty():
		catalyst = _catalyst_origin(Realm.DAO_SERVERS[0])
	var url := RewardCampaigns.get_rewards_server() + "/api/rewards?campaign_id=" + _campaign_id
	var body_dict := {
		"campaign_key": _campaign_key,
		"catalyst": catalyst,
		"beneficiary": beneficiary,
	}
	var body := JSON.stringify(body_dict)
	print("[RewardModal] claim POST ", url, " catalyst=", catalyst, " beneficiary=", beneficiary)

	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_POST, body)
	# The modal can be force-closed (close_reward_modal) while the request is in flight; bail out
	# before touching any node so we never operate on a freed instance.
	if not is_instance_valid(self):
		return

	_process_claim_response(response)


## Enters the "claiming" visual (design: the OTP-Claimed middle state). The signed POST is ~2s of
## network with NO main-thread freeze (verified on device), so a spinner animates fine: we swap the
## reward art for the animated spinner. The button keeps its "CLAIM REWARD" label (just disabled so
## it can't be re-tapped) and only flips to CLAIMED once the response confirms the grant.
func _start_processing() -> void:
	_claiming = true
	button_claim.disabled = true
	if is_instance_valid(loading_spinner):
		loading_spinner.visible = true
	if is_instance_valid(texture_rect_reward):
		texture_rect_reward.visible = false


## Interprets the rewards-server response, mirroring claim.ts `processResponse`.
func _process_claim_response(response) -> void:
	var json
	if response is PromiseError:
		# request_json rejects on any non-2xx, but the rewards server still returns a JSON error
		# body (e.g. {"ok":false,"code":"campaign_disabled"}). Parse it so business errors get a
		# friendly message instead of raw JSON; a non-JSON reason is a real transport error.
		var reason := str(response.get_error())
		printerr("RewardModal: claim failed — ", reason)
		json = JSON.parse_string(reason)
		if not (json is Dictionary):
			_show_error(tr("REWARD_ERROR_SERVER_UNREACHABLE"))
			return
	else:
		json = response.get_string_response_as_json()
		if not (json is Dictionary):
			_show_error(tr("REWARD_ERROR_INVALID_RESPONSE"))
			return

	_handle_claim_json(json)


## Interprets a parsed rewards-server JSON body, mirroring claim.ts `processResponse`.
func _handle_claim_json(json: Dictionary) -> void:
	var ok = json.get("ok", null)
	var code := str(json.get("code", ""))

	# Campaign not currently active (uninitiated / disabled / finished).
	if (
		ok == false
		and code in ["campaign_key_uninitiated", "campaign_disabled", "campaign_finished"]
	):
		_show_error(tr("REWARD_NOT_AVAILABLE"))
		return

	# Any other non-ok response: friendly copy, never the raw code (already logged above).
	if ok == false:
		_show_error(_friendly_claim_error(code))
		return

	var data = json.get("data", [])
	# ok but nothing granted → out of stock.
	if not (data is Array) or data.is_empty():
		_show_error(tr("REWARD_OUT_OF_STOCK"))
		return

	# Already owned: assigned more than the window ago (a fresh claim echoes back a recent date).
	# The visual is the SAME either way (see _reveal_claimed) — only the copy differs.
	var item = data[0]
	var assigned_at = item.get("assigned_at", null)
	if assigned_at != null:
		# Godot's ISO parser chokes on fractional seconds / trailing 'Z'; keep YYYY-MM-DDTHH:MM:SS.
		var assigned_unix := Time.get_unix_time_from_datetime_string(str(assigned_at).substr(0, 19))
		if (
			assigned_unix > 0
			and assigned_unix < Time.get_unix_time_from_system() - ALREADY_CLAIMED_WINDOW_SEC
		):
			_show_already_claimed()
			return

	_show_success()


## Fresh grant: reveal the reward with the celebratory rays animation.
func _show_success() -> void:
	_reveal_claimed()


## Item was already in the wallet. Per QA (Elizabeth) we still reveal the same reward visual so a
## re-claim (or re-open) shows the "you've got it" state instead of a bare line of text.
func _show_already_claimed() -> void:
	_reveal_claimed()
	label_text.text = tr("REWARD_MODAL_YOU_ALREADY_HAVE_THIS_REWARD")


## Shared "claimed" end state (design: OTP-Claimed): stop the spinner, bring the reward art back
## with the rays animation, lock the button to CLAIMED, and reveal the "Tap to continue" hint (only
## shown once the reward is actually claimed). The art is the fixed baked image — decoupled from the
## urn / claim response — so it looks identical whether the item was just granted or already owned.
func _reveal_claimed() -> void:
	_claimed = true
	_claiming = false
	button_claim.disabled = true
	button_claim.text = tr("REWARD_MODAL_CLAIMED")
	if is_instance_valid(loading_spinner):
		loading_spinner.visible = false
	if is_instance_valid(texture_rect_reward):
		texture_rect_reward.visible = true
	if is_instance_valid(label_tap_to_continue):
		label_tap_to_continue.visible = true
	animation_player.play("claim")


## Maps a rewards-server error code to user-facing copy. Unknown codes fall through to a
## generic message so a raw code (e.g. "catalyst_unreachable") never reaches the UI — the
## real code is still logged by _process_claim_response for debugging.
func _friendly_claim_error(code: String) -> String:
	match code:
		"campaign_not_found", "campaign_key_not_found", "campaign_not_owner":
			return tr("REWARD_NOT_AVAILABLE")
		"already_claimed", "user_already_claimed":
			return tr("REWARD_ALREADY_OWNED")
		"supply_reached", "out_of_stock":
			return tr("REWARD_OUT_OF_STOCK")
		"catalyst_invalid", "catalyst_unreachable", "user_address_not_connected", "user_address_position":
			return tr("REWARD_SESSION_UNVERIFIED")
	return tr("REWARD_CLAIM_FAILED")


func _show_error(message: String) -> void:
	# Roll back the processing visual so the reward art is visible again and the user can retry.
	_claiming = false
	if is_instance_valid(loading_spinner):
		loading_spinner.visible = false
	if is_instance_valid(texture_rect_reward):
		texture_rect_reward.visible = true
	button_claim.disabled = false
	button_claim.text = tr("REWARD_MODAL_CLAIM_REWARD")
	label_text.text = message
