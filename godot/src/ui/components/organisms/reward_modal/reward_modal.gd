class_name RewardModal
extends ColorRect

## Emitted on a backdrop tap when `managed_by_modal_manager` is off.
signal dismissed
## Emitted instead of claiming when `guests_must_upgrade` is on and the player is a guest who
## has not added an email.
signal upgrade_required

## Two-minute window (in seconds) used to tell a fresh claim apart from an item the
## wallet already owned, mirroring Genesis' claim.ts `assigned_at` check.
const ALREADY_CLAIMED_WINDOW_SEC := 120.0

## Translation KEY of the body copy shown until the claim resolves.
@export var body_key := "REWARD_MODAL_YOUR_EMAIL_HAS_BEEN_VERIFIED_ENJOY"
## Off for a scene shown by something other than ModalManager, which then closes it on
## `dismissed`.
@export var managed_by_modal_manager := true
@export var guests_must_upgrade := false

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
	# A scene with an emote preview shows the player performing the reward over the art.
	var emote_preview := get_node_or_null("%EmoteAvatarPreview") as EmoteAvatarPreview
	var urn := str(campaign.get("urn", ""))
	if emote_preview != null and not urn.is_empty():
		emote_preview.async_play(urn)


func _ready() -> void:
	# Label_Text never auto-translates (its error state carries server text), and
	# _show_success() deliberately leaves this copy in place, so seed it here.
	label_text.text = tr(body_key)


func _on_gui_input(event: InputEvent) -> void:
	# Never dismiss while a claim request is in flight.
	if _claiming:
		return
	if event is InputEventScreenTouch:
		if event.pressed:
			if managed_by_modal_manager:
				Global.modal_manager.close_reward_modal()
			else:
				dismissed.emit()


func _on_button_claim_pressed() -> void:
	var identity := Global.player_identity
	if (
		guests_must_upgrade
		and identity.is_thirdweb_guest()
		and not identity.is_thirdweb_guest_upgraded()
	):
		upgrade_required.emit()
		return
	_async_claim()


## Performs the real claim via the shared RewardClaimer (client-side equivalent of Genesis'
## claimToken/requestToken) and drives this modal's own visuals off the result.
func _async_claim() -> void:
	if _claiming or _claimed:
		return

	_start_processing()
	var result := await RewardClaimer.async_claim(
		{"campaign_id": _campaign_id, "campaign_key": _campaign_key}
	)
	# The modal can be force-closed (close_reward_modal) while the request is in flight; bail out
	# before touching any node so we never operate on a freed instance.
	if not is_instance_valid(self):
		return

	if not result.ok:
		_show_error(result.error_message)
		return
	if result.already_owned:
		_show_already_claimed()
	else:
		_show_success()


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
