@tool
# One-shot bake for the double-jump & glide animation libraries.
#
# Run from the project root with:
#   .bin/godot/godot4_bin --headless --path godot --script res://src/tool/bake_double_jump_and_glide.gd --quit
#
# Produces:
#   - res://assets/animations/double_jump_and_glide.tres  (12 avatar clips, Avatar_Hips rig)
#   - res://assets/avatar/glider_prop_anims.tres          (8 glider prop clips, Prop_Root rig)
#   - res://assets/avatar/glider_prop_base.scn            (mesh+skin+textures, self-contained)
#
# The 20 source .glb files live on the local `double-jump-glide/sources`
# branch (not tracked on main) — drop them into res://assets/animations/
# before running this script.
extends SceneTree

const ANIM_DIR := "res://assets/animations/"
const PROP_OUT_DIR := "res://assets/avatar/"
const AVATAR_LIB_OUT := "res://assets/animations/double_jump_and_glide.tres"
const PROP_LIB_OUT := "res://assets/avatar/glider_prop_anims.tres"
# Self-contained prop scene: mesh + skin + materials + textures are all inlined,
# so we don't need any of the source .glb files tracked in git.
const PROP_BASE_SRC := "res://assets/animations/Gliding_PropIdle.glb"
const PROP_BASE_OUT := "res://assets/avatar/glider_prop_base.scn"

# Source GLB → library clip name. The key side matches the .glb file stem; the
# value side is the name under which the clip will be stored in the library.
# We intentionally keep the value identical to the animation name inside the
# GLB (confirmed via `gltf.animations[0].name` when probed) so the library key
# and the track paths stay legible.
const AVATAR_CLIPS := {
	"DoubleJump_Base2": "Double_Jump_Base",
	"DoubleJump_Base_Right": "Double_Jump_Base_Right",
	"DoubleJump_Jog2": "Double_Jump_Jog",
	"DoubleJump_Jog_Right": "Double_Jump_Jog_Right",
	"DoubleJump_Run2": "Double_Jump_Run",
	"DoubleJump_Run_Right": "Double_Jump_Run_Right",
	"Gliding_AvatarStart": "Gliding_Start",
	"Gliding_AvatarIdle": "Gliding_Idle",
	"Gliding_AvatarForward": "Gliding_Forward",
	"Gliding_AvatarLeft": "Gliding_TurnLeft",
	"Gliding_AvatarRight": "Gliding_TurnRight",
	"Gliding_AvatarEnd": "Gliding_End",
}

const PROP_CLIPS := {
	"Gliding_PropOpen": "Glider_Open",
	"Gliding_PropClose": "Glider_Close",
	"Gliding_PropStart": "Glider_Start",
	"Gliding_PropIdle": "Glider_Idle",
	"Gliding_PropForward": "Glider_Forward",
	"Gliding_PropLeft": "Glider_TurnLeft",
	"Gliding_PropRight": "Glider_TurnRight",
	"Gliding_PropEnd": "Glider_End",
}


func _init() -> void:
	var failures: Array[String] = []

	var avatar_lib := _bake_library(AVATAR_CLIPS, ANIM_DIR, failures)
	_save_library(avatar_lib, AVATAR_LIB_OUT, failures)

	var prop_lib := _bake_library(PROP_CLIPS, ANIM_DIR, failures)
	_save_library(prop_lib, PROP_LIB_OUT, failures)

	_bake_glider_prop_base(failures)

	if failures.is_empty():
		print("\n[bake] OK — wrote %s, %s, %s" % [AVATAR_LIB_OUT, PROP_LIB_OUT, PROP_BASE_OUT])
	else:
		push_error("[bake] completed with %d failure(s):" % failures.size())
		for err in failures:
			push_error("  - %s" % err)

	quit(0 if failures.is_empty() else 1)


func _bake_library(
	mapping: Dictionary, source_dir: String, failures: Array[String]
) -> AnimationLibrary:
	var lib := AnimationLibrary.new()
	for stem: String in mapping.keys():
		var library_key: String = mapping[stem]
		var glb_path := "%s%s.glb" % [source_dir, stem]
		var anim := _extract_single_animation(glb_path, failures)
		if anim == null:
			continue
		# Save as a sub-resource (inline in the .tres). Cheap, stable, avoids
		# one-file-per-clip churn under assets/animations.
		var add_err := lib.add_animation(StringName(library_key), anim)
		if add_err != OK:
			failures.append("add_animation(%s) → %d" % [library_key, add_err])
		else:
			print(
				(
					"[bake]   %s → %s (len=%.3fs, tracks=%d)"
					% [stem, library_key, anim.length, anim.get_track_count()]
				)
			)
	return lib


func _extract_single_animation(glb_path: String, failures: Array[String]) -> Animation:
	var scene: PackedScene = ResourceLoader.load(glb_path) as PackedScene
	if scene == null:
		failures.append("could not load %s as PackedScene" % glb_path)
		return null
	var root: Node = scene.instantiate()
	if root == null:
		failures.append("could not instantiate %s" % glb_path)
		return null

	var anim: Animation = null
	var player := _find_animation_player(root)
	if player != null:
		var libs := player.get_animation_library_list()
		for lib_name in libs:
			var lib := player.get_animation_library(lib_name)
			for clip_name in lib.get_animation_list():
				anim = lib.get_animation(clip_name)
				# Duplicate so the saved resource is decoupled from the imported GLB.
				anim = anim.duplicate(true) as Animation
				break
			if anim != null:
				break

	root.queue_free()
	if anim == null:
		failures.append("no Animation found in %s" % glb_path)
	return anim


func _find_animation_player(node: Node) -> AnimationPlayer:
	if node is AnimationPlayer:
		return node
	for child in node.get_children():
		var hit := _find_animation_player(child)
		if hit != null:
			return hit
	return null


func _save_library(lib: AnimationLibrary, path: String, failures: Array[String]) -> void:
	if lib.get_animation_list().is_empty():
		failures.append("refusing to save empty library at %s" % path)
		return
	# Make sure the parent directory exists.
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(path.get_base_dir()))
	var err := ResourceSaver.save(lib, path)
	if err != OK:
		failures.append("ResourceSaver.save(%s) → %d" % [path, err])
		return
	print("[bake] wrote %s (%d clips)" % [path, lib.get_animation_list().size()])


# Builds a .scn for the glider prop that has no references to the source GLB
# or its imported .ctex files. This lets us delete every .glb/.png under
# assets/animations/ and still have the prop render correctly.
func _bake_glider_prop_base(failures: Array[String]) -> void:
	var src := ResourceLoader.load(PROP_BASE_SRC) as PackedScene
	if src == null:
		failures.append("could not load %s as PackedScene" % PROP_BASE_SRC)
		return
	var root: Node = src.instantiate()
	if root == null:
		failures.append("could not instantiate %s" % PROP_BASE_SRC)
		return

	# Drop the imported AnimationPlayer library — glider_prop.tscn re-attaches
	# the real one (glider_prop_anims.tres) via scene-inheritance override.
	var ap := _find_animation_player(root)
	if ap != null:
		for lib_name in ap.get_animation_library_list():
			ap.remove_animation_library(lib_name)

	# Break every external-resource reference so PackedScene.pack() embeds
	# meshes, skins, materials, and textures as sub-resources.
	_break_external_deps(root)

	# Every descendant must list `root` as its owner or PackedScene.pack()
	# silently drops it.
	_assign_owner_recursively(root, root)

	var packed := PackedScene.new()
	var pack_err := packed.pack(root)
	if pack_err != OK:
		failures.append("PackedScene.pack() → %d" % pack_err)
		root.queue_free()
		return

	DirAccess.make_dir_recursive_absolute(
		ProjectSettings.globalize_path(PROP_BASE_OUT.get_base_dir())
	)
	var save_err := ResourceSaver.save(packed, PROP_BASE_OUT)
	if save_err != OK:
		failures.append("ResourceSaver.save(%s) → %d" % [PROP_BASE_OUT, save_err])
	else:
		print("[bake] wrote %s (self-contained)" % PROP_BASE_OUT)
	root.queue_free()


func _break_external_deps(node: Node) -> void:
	if node is MeshInstance3D:
		var mi := node as MeshInstance3D
		if mi.mesh != null:
			var mesh_copy: Mesh = mi.mesh.duplicate(true)
			mesh_copy.resource_path = ""
			for i in mesh_copy.get_surface_count():
				var mat := mesh_copy.surface_get_material(i)
				if mat != null:
					mesh_copy.surface_set_material(i, _embed_material(mat))
			mi.mesh = mesh_copy
		if mi.skin != null:
			var skin_copy: Skin = mi.skin.duplicate(true)
			skin_copy.resource_path = ""
			mi.skin = skin_copy
		# Override material slots on the MeshInstance3D itself (rare, but safe).
		for i in mi.get_surface_override_material_count():
			var override := mi.get_surface_override_material(i)
			if override != null:
				mi.set_surface_override_material(i, _embed_material(override))
	for c in node.get_children():
		_break_external_deps(c)


func _embed_material(mat: Material) -> Material:
	var copy: Material = mat.duplicate(true)
	copy.resource_path = ""
	if copy is BaseMaterial3D:
		var bm := copy as BaseMaterial3D
		var slots := [
			BaseMaterial3D.TEXTURE_ALBEDO,
			BaseMaterial3D.TEXTURE_METALLIC,
			BaseMaterial3D.TEXTURE_ROUGHNESS,
			BaseMaterial3D.TEXTURE_EMISSION,
			BaseMaterial3D.TEXTURE_NORMAL,
			BaseMaterial3D.TEXTURE_RIM,
			BaseMaterial3D.TEXTURE_CLEARCOAT,
			BaseMaterial3D.TEXTURE_FLOWMAP,
			BaseMaterial3D.TEXTURE_AMBIENT_OCCLUSION,
			BaseMaterial3D.TEXTURE_HEIGHTMAP,
			BaseMaterial3D.TEXTURE_SUBSURFACE_SCATTERING,
			BaseMaterial3D.TEXTURE_SUBSURFACE_TRANSMITTANCE,
			BaseMaterial3D.TEXTURE_BACKLIGHT,
			BaseMaterial3D.TEXTURE_REFRACTION,
			BaseMaterial3D.TEXTURE_DETAIL_MASK,
			BaseMaterial3D.TEXTURE_DETAIL_ALBEDO,
			BaseMaterial3D.TEXTURE_DETAIL_NORMAL,
			BaseMaterial3D.TEXTURE_ORM,
		]
		for slot in slots:
			_embed_texture_slot(bm, slot)
	return copy


func _embed_texture_slot(mat: BaseMaterial3D, slot: int) -> void:
	var tex := mat.get_texture(slot)
	if tex == null:
		return
	if tex is ImageTexture and tex.resource_path.is_empty():
		return
	var img := tex.get_image()
	if img == null:
		return
	var embedded := ImageTexture.create_from_image(img)
	embedded.resource_path = ""
	mat.set_texture(slot, embedded)


func _assign_owner_recursively(node: Node, owner: Node) -> void:
	for child in node.get_children():
		if child != owner:
			child.owner = owner
		_assign_owner_recursively(child, owner)
