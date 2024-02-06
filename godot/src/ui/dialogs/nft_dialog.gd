extends Control

var current_asset: OpenSeaFetcher.Asset = null
var opensea_url = "https://decentraland.org/"


func _ready():
	%VBoxContainer_InfoPanel.hide()
	%Button_ViewOnOpenSea.disabled = true


func _on_visibility_changed():
	if is_visible():
		Global.release_mouse()


func async_load_nft(urn: String):
	var dcl_urn: DclUrn = DclUrn.new(urn)
	if not dcl_urn.valid:
		printerr("NftShape::load_nft Error, invalid urn: ", urn)
		return

	var promise = Global.nft_fetcher.fetch_nft(dcl_urn)
	var asset = await PromiseUtils.async_awaiter(promise)
	if asset is OpenSeaFetcher.Asset:
		%VBoxContainer_InfoPanel.show()
		opensea_url = asset.opensea_url
		%Button_ViewOnOpenSea.disabled = false
		%LoadingAnimation.hide()
		current_asset = asset
		%TextureRect_NFTImage.texture = asset.texture

		%ColorRect_Background.color = Color(asset.background_color)

		var owner_name = asset.get_owner_name()
		if DclEther.is_valid_ethereum_address(asset.address):
			var owner_url = "https://opensea.io/" + asset.address
			%RichTextBox_Owner.parse_bbcode("[url=%s]%s[/url]" % [owner_url, owner_name])
		else:
			%RichTextBox_Owner.parse_bbcode(owner_name)
		%Label_Title.text = asset.name
		%Label_Description.text = asset.description


func _on_button_cancel_pressed():
	queue_free()


func _on_button_view_on_open_sea_pressed():
	OS.shell_open(opensea_url)


func _on_rich_text_box_owner_meta_clicked(meta):
	OS.shell_open(meta)
