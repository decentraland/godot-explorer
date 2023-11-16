extends Control

var current_asset: OpenSeaFetcher.Asset = null
var permalink = "https://decentraland.org/"


func _ready():
	%DetailsPanel.hide()
	%ViewOnOpenSea.disabled = true


func _on_visibility_changed():
	if is_visible():
		Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)


func co_load_nft(urn: String):
	var dcl_urn: DclUrn = DclUrn.new(urn)
	if not dcl_urn.valid:
		printerr("NftShape::load_nft Error, invalid urn: ", urn)
		return

	var promise = Global.nft_fetcher.fetch_nft(dcl_urn)
	var asset = await promise.co_awaiter()
	if asset is OpenSeaFetcher.Asset:
		%DetailsPanel.show()
		permalink = asset.permalink
		%ViewOnOpenSea.disabled = false
		%LoadingAnimation.hide()
		current_asset = asset
		%NFTImage.texture = asset.texture

		%Background.color = Color(asset.background_color)

		var owner_name = asset.get_owner_name()
		if Ether.is_valid_ethereum_address(asset.address):
			var owner_url = "https://opensea.io/" + asset.address
			%Owner.parse_bbcode("[url=%s]%s[/url]" % [owner_url, owner_name])
		else:
			%Owner.parse_bbcode(owner_name)
		%Title.text = asset.name
		%Description.text = asset.description

		if asset.last_sell_erc20 != null:
			var last_sell = asset.last_sell_erc20
			%LastSoldFor.text = (
				last_sell.ether_to_string() + " (" + last_sell.dollar_to_string() + ")"
			)
		else:
			%LastSoldFor.text = "NEVER SOLD"

		%AvgPrice.text = asset.average_price_to_string()


func _on_view_on_open_sea_pressed():
	OS.shell_open(permalink)


func _on_cancel_pressed():
	queue_free()


func _on_owner_meta_clicked(meta):
	OS.shell_open(meta)
