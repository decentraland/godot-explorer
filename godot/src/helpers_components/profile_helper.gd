class_name ProfileHelper


static func get_mutable_profile() -> DclUserProfile:
	var backpack = Global.get_backpack()
	if backpack != null:
		return backpack.mutable_profile
	return null


# ADR-290: Snapshots no longer uploaded
static func async_save_profile():
	var backpack = Global.get_backpack()
	if backpack != null:
		await backpack.async_save_profile()


static func has_changes() -> bool:
	var backpack = Global.get_backpack()
	if backpack != null:
		return backpack.has_changes()
	return false
