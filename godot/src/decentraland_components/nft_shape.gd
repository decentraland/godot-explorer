class_name NftShape
extends Node3D

var current_style = null
var loading: bool = false
var scheduled_load_nft: Variant = null
var current_frame: Node3D = null


func _get_mesh_instance_3d(node: Node3D) -> MeshInstance3D:
	var root_node = node.find_child("RootNode", false)

	for child in root_node.get_children():
		if child is MeshInstance3D:
			return child
	return null


func async_load_nft(
	urn: String, style: NftFrameStyleLoader.NFTFrameStyles, background_color: Color
):
	# debounce the call, to avoid multiple calls being processed
	# we just process the current one, and the last one
	if loading:
		scheduled_load_nft = self.async_load_nft.bind(urn, style, background_color)
		return

	loading = true
	var picture_frame = _load_frame_style(style, background_color)
	await _async_load_nft(picture_frame, urn, style)
	loading = false

	if scheduled_load_nft is Callable:
		var callable = scheduled_load_nft
		scheduled_load_nft = null
		callable.call()


func _load_frame_style(
	style: NftFrameStyleLoader.NFTFrameStyles, background_color: Color
) -> Node3D:
	if style == current_style:
		return current_frame

	current_style = style

	for child in get_children():
		remove_child(child)

	current_frame = Global.nft_frame_loader.instantiate(style)

	_set_loading_material(style, current_frame, background_color)

	add_child(current_frame)
	return current_frame


func _set_picture_frame_texture(
	mesh_instance_3d: MeshInstance3D, style: NftFrameStyleLoader.NFTFrameStyles, texture: Texture2D
):
	var tex_width := 256 if texture == null else texture.get_width()
	var tex_height := 256 if texture == null else texture.get_height()
	var factor = float(maxi(tex_width, tex_height))
	var scale_x = tex_width / factor
	var scale_y = tex_height / factor
	self.scale = Vector3(0.5, 0.5, 0.5) * Vector3(scale_x, scale_y, 1.0)

	if style == NftFrameStyleLoader.NFTFrameStyles.NFT_NONE:
		# plane shape
		var material = mesh_instance_3d.mesh.get_material().duplicate()
		material.albedo_texture = texture
		mesh_instance_3d.set_surface_override_material(0, material)
	else:
		var look_at_material := "PictureFrame"
		if style == NftFrameStyleLoader.NFTFrameStyles.NFT_CLASSIC:
			look_at_material = "Mat_CK"
		var surf_idx = _get_surf_idx_by_resource_name(mesh_instance_3d.mesh, look_at_material)
		var material = mesh_instance_3d.mesh.surface_get_material(surf_idx).duplicate()
		material.albedo_texture = texture
		mesh_instance_3d.set_surface_override_material(surf_idx, material)


func _set_picture_frame_material(
	mesh_instance_3d: MeshInstance3D, style: NftFrameStyleLoader.NFTFrameStyles, material: Material
):
	if style == NftFrameStyleLoader.NFTFrameStyles.NFT_NONE:
		# plane shape
		mesh_instance_3d.set_surface_override_material(0, material)
	else:
		var surf_idx = _get_surf_idx_by_resource_name(mesh_instance_3d.mesh, "PictureFrame")
		mesh_instance_3d.set_surface_override_material(surf_idx, material)


func _set_loading_material(
	style: NftFrameStyleLoader.NFTFrameStyles, new_current_frame: Node3D, background_color: Color
):
	var mesh_instance_3d: MeshInstance3D = _get_mesh_instance_3d(new_current_frame)
	if mesh_instance_3d == null:
		printerr("set nft mesh_instance_3d is null")
		return

	var loading_material: Material = Global.nft_frame_loader.loading_material
	_set_picture_frame_material(mesh_instance_3d, style, loading_material)

	# Load background
	var background_material: StandardMaterial3D = _get_material_by_resource_name(
		mesh_instance_3d.mesh, "Background"
	)
	if background_material != null:
		background_material.albedo_color = background_color


func _async_load_nft(picture_frame: Node3D, urn: String, style: NftFrameStyleLoader.NFTFrameStyles):
	var dcl_urn: DclUrn = DclUrn.new(urn)
	if not dcl_urn.valid:
		printerr("NftShape::load_nft Error, invalid urn: ", urn)
		return

	var promise = Global.nft_fetcher.fetch_nft(dcl_urn)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("NftShape::load_nft Error on fetching nft: ", result.get_error())
		return
	await _async_set_opensea_nft(picture_frame, style, result)


func _get_surf_idx_by_resource_name(mesh: Mesh, resource_name: String) -> int:
	for surf_idx in range(mesh.get_surface_count()):
		var material: StandardMaterial3D = mesh.surface_get_material(surf_idx)
		if material.resource_name.contains(resource_name):
			return surf_idx
	return 0


func _get_material_by_resource_name(mesh: Mesh, resource_name: String) -> StandardMaterial3D:
	for surf_idx in range(mesh.get_surface_count()):
		var material: StandardMaterial3D = mesh.surface_get_material(surf_idx)
		if material.resource_name.begins_with(resource_name):
			return material
	return null


func _async_set_opensea_nft(
	picture_frame: Node3D, style: NftFrameStyleLoader.NFTFrameStyles, asset: OpenSeaFetcher.Asset
):
	var mesh_instance_3d: MeshInstance3D = _get_mesh_instance_3d(picture_frame)
	if mesh_instance_3d == null:
		printerr("set nft mesh_instance_3d is null")
		return

	# Clean loading material...
	_set_picture_frame_texture(mesh_instance_3d, style, asset.texture)
