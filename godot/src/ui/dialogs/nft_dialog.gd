extends Control

var current_asset: OpenSeaFetcher.Asset = null

func _on_visibility_changed():
	if is_visible():
		Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)

func load_nft(urn: String):
	var dcl_urn: DclUrn = DclUrn.new(urn)
	if not dcl_urn.valid:
		printerr("NftShape::load_nft Error, invalid urn: ", urn)
		return

	var promise = Global.nft_fetcher.fetch_nft(dcl_urn)
	var asset = await promise.co_awaiter()
	if asset is OpenSeaFetcher.Asset:
		current_asset = asset

func _on_view_on_open_sea_pressed():
	pass # Replace with function body.


func _on_cancel_pressed():
	queue_free()
