extends Node3D
class_name NftShape

var frame: Node3D = null
var loading: bool = false
var scheduled_load_nft: Variant = null


func _get_mesh_instance_3d(node: Node3D) -> MeshInstance3D:
	var root_node = node.find_child("RootNode", false)

	for child in root_node.get_children():
		if child is MeshInstance3D:
			return child
	return null


func load_nft(urn: String, style: NftFrameStyleLoader.NFTFrameStyles, background_color: Color):
	# debounce the call, to avoid multiple calls being processed
	# we just process the current one, and the last one
	if loading:
		scheduled_load_nft = self.load_nft.bind(urn, style, background_color)
		return

	loading = true
	_load_nft(urn, style, background_color)
	loading = false

	if scheduled_load_nft is Callable:
		var callable = scheduled_load_nft
		scheduled_load_nft = null
		callable.call()


func _load_nft(urn: String, style: NftFrameStyleLoader.NFTFrameStyles, background_color: Color):
	for child in get_children():
		remove_child(child)

	var dcl_urn: DclUrn = DclUrn.new(urn)
	if not dcl_urn.valid:
		printerr("NftShape::load_nft Error, invalid urn: ", urn)
		return

	var promise = Global.nft_fetcher.fetch_nft(dcl_urn)
	var result = await promise.co_awaiter()
	if result is Promise.Error:
		printerr("NftShape::load_nft Error on fetching nft: ", result.get_error())
		return
	set_opensea_nft(style, result, background_color)


func _get_material_by_resource_name(mesh: Mesh, resource_name: String) -> StandardMaterial3D:
	for surf_idx in range(mesh.get_surface_count()):
		var material: StandardMaterial3D = mesh.surface_get_material(surf_idx)
		if material.resource_name.begins_with(resource_name):
			return material
	return null


func set_opensea_nft(
	type: NftFrameStyleLoader.NFTFrameStyles, asset: OpenSeaFetcher.Asset, background_color: Color
):
	frame = Global.nft_frame_loader.instantiate(type)

	var mesh_instance_3d: MeshInstance3D = _get_mesh_instance_3d(frame)
	if mesh_instance_3d == null:
		printerr("set nft mesh_instance_3d is null")
		return

	if type == NftFrameStyleLoader.NFTFrameStyles.NFT_NONE:
		var picture_material: StandardMaterial3D = mesh_instance_3d.mesh.get_material()
		picture_material.albedo_texture = asset.texture
	else:
		var picture_material: StandardMaterial3D = _get_material_by_resource_name(
			mesh_instance_3d.mesh, "PictureFrame"
		)
		picture_material.albedo_texture = asset.texture
		picture_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		if mesh_instance_3d.mesh.get_surface_count() >= 3 and not asset.background_color.is_empty():
			var background_material: StandardMaterial3D = _get_material_by_resource_name(
				mesh_instance_3d.mesh, "Background"
			)
			background_material.albedo_color = background_color  #asset.background_color
	add_child(frame)
