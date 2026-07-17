class_name RewardModal
extends ColorRect

@onready var button_claim: Button = %Button_Claim
@onready
var animation_player: AnimationPlayer = $VBoxContainer/PanelContainer/MarginContainer_Reward/AnimationPlayer
@onready var texture_rect_reward: TextureRect = %TextureRect_Reward


func async_setup(urn: String) -> void:
	hide()

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
	if event is InputEventScreenTouch:
		if event.pressed:
			Global.modal_manager.close_reward_modal()


func _on_button_claim_pressed() -> void:
	button_claim.text = "CLAIMED"
	button_claim.disabled = true
	animation_player.play("claim")
