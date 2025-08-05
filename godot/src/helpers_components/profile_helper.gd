class_name ProfileHelper

static func get_backpack() -> Backpack:
	var explorer = Global.get_explorer()
	if explorer != null and is_instance_valid(explorer.control_menu):
		return explorer.control_menu.control_backpack
	return null

static func get_mutable_profile() -> DclUserProfile:
	var backpack = get_backpack()
	if backpack != null:
		return backpack.mutable_profile
	return null

static func save_profile(generate_snapshots: bool = true) -> void:
	var backpack = get_backpack()
	if backpack != null:
		await backpack.async_save_profile(generate_snapshots)

static func has_changes() -> bool:
	var backpack = get_backpack()
	if backpack != null:
		return backpack.has_changes()
	return false 
