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


## Configures the modal for a campaign and reveals it.
## @param campaign: { campaign_id, campaign_key, urn } — see RewardCampaigns.CAMPAIGNS.
func async_setup(campaign: Dictionary) -> void:
	hide()

	_campaign_id = str(campaign.get("campaign_id", ""))
	_campaign_key = str(campaign.get("campaign_key", ""))
	_claiming = false
	_claimed = false

	var urn := str(campaign.get("urn", ""))
	# No urn to preview: show the placeholder art now; the real item is revealed from
	# the claim response after a successful claim.
	if urn.is_empty():
		show()
		return

	var promise = Global.content_provider.fetch_wearables(
		[urn], Global.realm.get_profile_content_url()
	)
	await PromiseUtils.async_all(promise)

	var item = Global.content_provider.get_wearable(urn)
	if item == null:
		printerr("RewardModal: failed to fetch wearable definition for URN: ", urn)
		show()
		return

	var texture_promise: Promise = Global.content_provider.fetch_texture(
		item.get_thumbnail(), item.get_content_mapping()
	)
	var res = await PromiseUtils.async_awaiter(texture_promise)
	if res is PromiseError:
		printerr("RewardModal: failed to fetch texture for URN: ", urn, " — ", res.get_error())
	else:
		texture_rect_reward.texture = res.texture

	show()


func _on_gui_input(event: InputEvent) -> void:
	# Never dismiss while a claim request is in flight.
	if _claiming:
		return
	if event is InputEventScreenTouch:
		if event.pressed:
			Global.modal_manager.close_reward_modal()


func _on_button_claim_pressed() -> void:
	_async_claim()


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
		_show_error("You must be connected with a wallet to claim rewards.")
		return

	if _campaign_id.is_empty() or _campaign_key.is_empty():
		_show_error("This reward is not available right now.")
		return

	_claiming = true
	button_claim.disabled = true
	button_claim.text = "CLAIMING…"

	# The rewards server validates `catalyst`: it must be a valid URI — the realm base URL the
	# SDK exposes to scenes as realmInfo.baseUrl. It can be empty when claiming outside a realm
	# (e.g. right after upgrading from Discover), and the server rejects an empty string as
	# non-uri, so fall back to the content URL and finally the default main realm.
	var catalyst := str(Global.realm.realm_url)
	if catalyst.is_empty():
		catalyst = str(Global.realm.content_base_url)
	if catalyst.is_empty():
		catalyst = DclUrls.main_realm()
	var url := RewardCampaigns.get_rewards_server() + "/api/rewards?campaign_id=" + _campaign_id
	var body_dict := {
		"campaign_key": _campaign_key,
		"catalyst": catalyst,
		"beneficiary": beneficiary,
	}
	var body := JSON.stringify(body_dict)
	print("[RewardModal] claim POST ", url, " catalyst=", catalyst, " beneficiary=", beneficiary)

	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_POST, body)
	_claiming = false

	_process_claim_response(response)


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
			_show_error("Error reaching the rewards server.")
			return
	else:
		json = response.get_string_response_as_json()
		if not (json is Dictionary):
			_show_error("Invalid response from the rewards server.")
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
		_show_error("This reward is not available right now.")
		return

	# Generic server error.
	if ok == false:
		_show_error(str(json.get("error", code if not code.is_empty() else "Invalid response")))
		return

	var data = json.get("data", [])
	# ok but nothing granted → out of stock.
	if not (data is Array) or data.is_empty():
		_show_error("This reward is out of stock.")
		return

	var item = data[0]
	# Already owned: assigned more than the window ago (a fresh claim echoes back a recent date).
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

	_show_success(item)


func _show_success(item: Dictionary) -> void:
	_claimed = true
	button_claim.disabled = true
	button_claim.text = "CLAIMED"
	animation_player.play("claim")

	# Reveal the real reward from the server response (the source of truth for the granted item).
	var image_url := str(item.get("image", ""))
	if not image_url.is_empty():
		_async_load_reward_image(image_url)


func _show_already_claimed() -> void:
	_claimed = true
	button_claim.disabled = true
	button_claim.text = "CLAIMED"
	label_text.text = "You already have this reward."


func _show_error(message: String) -> void:
	# Re-enable so the user can retry.
	button_claim.disabled = false
	button_claim.text = "CLAIM REWARD"
	label_text.text = message


func _async_load_reward_image(url: String) -> void:
	var promise = Global.content_provider.fetch_texture_by_url(url.md5_text(), url)
	var res = await PromiseUtils.async_awaiter(promise)
	if res is PromiseError:
		printerr("RewardModal: failed to fetch reward image: ", res.get_error())
		return
	if is_instance_valid(texture_rect_reward):
		texture_rect_reward.texture = res.texture
