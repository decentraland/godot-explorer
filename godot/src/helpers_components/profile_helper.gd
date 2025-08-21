class_name ProfileHelper


static func get_mutable_profile() -> DclUserProfile:
	var backpack = Global.get_backpack()
	if backpack != null:
		return backpack.mutable_profile
	return null


static func async_save_profile(generate_snapshots: bool = true):
	var backpack = Global.get_backpack()
	if backpack != null:
		await backpack.async_save_profile(generate_snapshots)


static func has_changes() -> bool:
	var backpack = Global.get_backpack()
	if backpack != null:
		return backpack.has_changes()
	return false
