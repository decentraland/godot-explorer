extends Control

var nft_fetcher: OpenSeaFetcher = OpenSeaFetcher.new()


func add_nft(urn: String):
	var promise = nft_fetcher.fetch_nft(DclUrn.new(urn))
	var result: OpenSeaFetcher.Asset = await promise.co_awaiter()
	prints("image_url:", result.image_url)
	var texture_rect = Sprite2D.new()
	texture_rect.texture = result.texture
	texture_rect.centered = false
	texture_rect.scale = Vector2(0.33, 0.33)
	var x = $GridContainer.get_child_count() % 3
	var y = $GridContainer.get_child_count() / 3
	texture_rect.position = Vector2(x, y) * 200
	$GridContainer.add_child(texture_rect)


func _on_button_pressed():
	var urns = $LineEdit.text.split("\n")
	for children in $GridContainer.get_children():
		$GridContainer.remove_child(children)
	for urn in urns:
		prints("urn", urn)
		add_nft(urn)
