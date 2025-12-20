@tool
extends EditorPlugin

const EMOTES_PATH = "res://assets/no-export/emotes/"
const ANIMATIONS_OUTPUT_PATH = "res://assets/animations/emotes/"
const LIBRARIES_OUTPUT_PATH = "res://assets/animations/"

# Base emotes (original 10 default slot emotes)
const BASE_EMOTES = {
	"Clapping_Particles.glb": "clap",
	"M_Dance.glb": "dance",
	"M_Fist_Pump.glb": "fistpump",
	"Hands_In_Air_Particles.glb": "handsair",
	"M_Head_Explode.glb": "headexplode",
	"Kiss_Particles.glb": "kiss",
	"Money_Particles.glb": "money",
	"Raise_Hand.glb": "raiseHand",
	"Shrug.glb": "shrug",
	"Wave_Male.glb": "wave",
}

# Extended base emotes (additional emotes from avatar-assets)
const EXTENDED_EMOTES = {
	"Disco.glb": "disco",
	"Cry_Particles.glb": "cry",
	"M_Dab.glb": "dab",
	"Dont_See.glb": "dontsee",
	"Hammer.glb": "hammer",
	"HoHoHo_Particles.glb": "hohoho",
	"Robot.glb": "robot",
	"Snowfall_Particles.glb": "snowfall",
	"Tektonik.glb": "tektonik",
	"Tik.glb": "tik",
	"Confetti_Popper.glb": "confettipopper",
	"Crafting.glb": "crafting",
}

# Utility/action emotes (triggered by scenes)
const ACTION_EMOTES = {
	"Button_45.glb": "buttonDown",
	"Button_Front.glb": "buttonFront",
	"Get_Hit.glb": "getHit",
	"KO.glb": "knockOut",
	"Lever.glb": "lever",
	"OpenChest.glb": "openChest",
	"OpenDoor.glb": "openDoor",
	"Punch.glb": "punch",
	"Push.glb": "push",
	"SittingChair_v01.glb": "sittingChair1",
	"SittingChair_v02.glb": "sittingChair2",
	"SittingGround_v01.glb": "sittingGround1",
	"SittingGround_v02.glb": "sittingGround2",
	"Swing_OneHand.glb": "swingWeaponOneHand",
	"Swing_DoubleHanded.glb": "swingWeaponTwoHands",
	"Throw.glb": "throw",
}


func _enter_tree():
	add_tool_menu_item("Convert All Emotes & Generate Libraries", _convert_all_emotes)


func _exit_tree():
	remove_tool_menu_item("Convert All Emotes & Generate Libraries")


func _convert_all_emotes():
	print("=== Converting ALL GLB animations to .tres ===")
	print("")

	# Create output directory if it doesn't exist
	var dir = DirAccess.open("res://assets/animations/")
	if dir and not dir.dir_exists("emotes"):
		dir.make_dir("emotes")

	# Step 1: Convert all GLBs to individual .tres files
	var base_results = _convert_emote_group(BASE_EMOTES, "Base Emotes")
	var extended_results = _convert_emote_group(EXTENDED_EMOTES, "Extended Emotes")
	var action_results = _convert_emote_group(ACTION_EMOTES, "Action Emotes")

	print("")
	print("=== Generating Animation Libraries ===")
	print("")

	# Step 2: Generate animation libraries that reference the .tres files
	var lib_success = 0
	var lib_failed = 0

	# Merge BASE_EMOTES and EXTENDED_EMOTES into a single default_emotes library
	var all_emotes = {}
	all_emotes.merge(BASE_EMOTES)
	all_emotes.merge(EXTENDED_EMOTES)

	if _generate_animation_library("default_emotes", all_emotes):
		lib_success += 1
	else:
		lib_failed += 1

	if _generate_animation_library("default_actions", ACTION_EMOTES):
		lib_success += 1
	else:
		lib_failed += 1

	print("")
	print("=== Conversion Complete ===")
	print("Animations converted: %d" % (base_results[0] + extended_results[0] + action_results[0]))
	print("Animations skipped: %d" % (base_results[1] + extended_results[1] + action_results[1]))
	print("Animations failed: %d" % (base_results[2] + extended_results[2] + action_results[2]))
	print("Libraries generated: %d" % lib_success)
	print("Libraries failed: %d" % lib_failed)

	# Refresh the filesystem
	get_editor_interface().get_resource_filesystem().scan()

	# Show result popup
	var total_converted = base_results[0] + extended_results[0] + action_results[0]
	var total_skipped = base_results[1] + extended_results[1] + action_results[1]
	var total_failed = base_results[2] + extended_results[2] + action_results[2]
	var message = """Conversion complete!

Animations:
  Converted: %d
  Skipped: %d
  Failed: %d

Libraries:
  Generated: %d
  Failed: %d

Output:
  - default_emotes.tres (22 emotes)
  - default_actions.tres (16 action emotes)""" % [total_converted, total_skipped, total_failed, lib_success, lib_failed]
	OS.alert(message, "Emote Converter")


func _convert_emote_group(emote_map: Dictionary, group_name: String) -> Array:
	print("--- Converting %s ---" % group_name)
	var converted = 0
	var skipped = 0
	var failed = 0

	for glb_file in emote_map.keys():
		var emote_id = emote_map[glb_file]
		var glb_path = EMOTES_PATH + glb_file
		var output_path = ANIMATIONS_OUTPUT_PATH + emote_id + ".tres"

		print("  Converting: %s -> %s" % [glb_file, emote_id])

		# Check if GLB exists
		if not FileAccess.file_exists(glb_path):
			print("    ERROR: GLB not found: %s" % glb_path)
			failed += 1
			continue

		# Load the GLB as AnimationLibrary
		var anim_lib = load(glb_path) as AnimationLibrary
		if anim_lib == null:
			print("    ERROR: Failed to load: %s" % glb_path)
			failed += 1
			continue

		# Get animation names from the library
		var anim_names = anim_lib.get_animation_list()
		if anim_names.is_empty():
			print("    ERROR: No animations in: %s" % glb_path)
			failed += 1
			continue

		# Get the first animation
		var anim_name = anim_names[0]
		var anim = anim_lib.get_animation(anim_name)
		if anim == null:
			print("    ERROR: Could not get animation: %s" % anim_name)
			failed += 1
			continue

		# Create a duplicate as standalone resource
		var anim_copy = anim.duplicate(true)
		anim_copy.resource_name = emote_id

		# Fix bone paths if needed (convert node hierarchy to Skeleton3D bone paths)
		_fix_animation_bone_paths(anim_copy)

		# Save with emote_id as filename
		var error = ResourceSaver.save(anim_copy, output_path)
		if error == OK:
			print("    Saved: %s" % output_path)
			converted += 1
		else:
			print("    ERROR: Failed to save (error %d)" % error)
			failed += 1

	return [converted, skipped, failed]


func _generate_animation_library(lib_name: String, emote_map: Dictionary) -> bool:
	var output_path = LIBRARIES_OUTPUT_PATH + lib_name + ".tres"
	print("Generating: %s" % output_path)

	# Create a new AnimationLibrary
	var lib = AnimationLibrary.new()

	# Add each animation to the library
	var added = 0
	var missing = 0

	for glb_file in emote_map.keys():
		var emote_id = emote_map[glb_file]
		var anim_path = ANIMATIONS_OUTPUT_PATH + emote_id + ".tres"

		if not FileAccess.file_exists(anim_path):
			print("  WARNING: Animation not found: %s" % anim_path)
			missing += 1
			continue

		var anim = load(anim_path) as Animation
		if anim == null:
			print("  WARNING: Failed to load animation: %s" % anim_path)
			missing += 1
			continue

		lib.add_animation(emote_id, anim)
		added += 1

	if added == 0:
		print("  ERROR: No animations added to library")
		return false

	# Save the library
	var error = ResourceSaver.save(lib, output_path)
	if error == OK:
		print("  Success: %d animations (%d missing)" % [added, missing])
		return true
	else:
		print("  ERROR: Failed to save library (error %d)" % error)
		return false


# Fix animation bone paths from node hierarchy format to Skeleton3D bone format
# Converts: "Armature/Avatar_Hips/Avatar_Spine/..." -> "Armature/Skeleton3D:Avatar_Spine"
func _fix_animation_bone_paths(anim: Animation) -> void:
	var fixed_count = 0

	for track_idx in range(anim.get_track_count()):
		var path = anim.track_get_path(track_idx)
		var path_str = str(path)

		# Check if this is a node hierarchy path that needs fixing
		# Pattern: "Armature/Avatar_*" without "Skeleton3D:"
		if path_str.begins_with("Armature/Avatar_") and not path_str.contains("Skeleton3D:"):
			# Extract the last bone name from the path
			# e.g., "Armature/Avatar_Hips/Avatar_LeftUpLeg/Avatar_LeftLeg" -> "Avatar_LeftLeg"
			var parts = path_str.split("/")
			var last_bone = ""

			# Find the last Avatar_* part
			for i in range(parts.size() - 1, -1, -1):
				if parts[i].begins_with("Avatar_"):
					last_bone = parts[i]
					break

			if not last_bone.is_empty():
				# Create new path in Skeleton3D format
				var new_path = NodePath("Armature/Skeleton3D:" + last_bone)
				anim.track_set_path(track_idx, new_path)
				fixed_count += 1

	if fixed_count > 0:
		print("    Fixed %d bone paths (node hierarchy -> Skeleton3D)" % fixed_count)
