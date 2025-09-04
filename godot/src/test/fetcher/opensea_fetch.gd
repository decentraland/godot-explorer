extends Control

var nft_fetcher: OpenSeaFetcher = OpenSeaFetcher.new()


func async_add_nft(urn: String):
	var promise = nft_fetcher.fetch_nft(DclUrn.new(urn))
	var result: OpenSeaFetcher.Asset = await PromiseUtils.async_awaiter(promise)
	prints("image_url:", result.image_url)
	var texture_rect = TextureRect.new()
	texture_rect.texture = result.texture
	$GridContainer/VBoxContainer.add_child(texture_rect)


func _on_button_pressed():
	var urns = $LineEdit.text.split("\n")
	for children in $GridContainer/VBoxContainer.get_children():
		$GridContainer/VBoxContainer.remove_child(children)
		children.queue_free()

	for urn in urns:
		prints("urn", urn)
		async_add_nft(urn)
