class_name ProfileHelper


static func has_changes() -> bool:
	var backpack = Global.get_backpack()
	if backpack != null:
		return backpack.has_changes()
	return false
