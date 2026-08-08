extends RefCounted

# Loaded via preload (not class_name) by ImpostorCapturer and its headless test,
# so it needs no global-class-cache registration / asset re-import.

# Pure priority pick for the ImpostorCapturer queue, extracted so it can be unit
# tested headless without the autoload or the AvatarPreview scene it pulls in.
#
# `entries` is an Array of Dictionaries {distance: float, off_frustum: bool}.
# Priority: in-frustum avatars before off-frustum ones, nearest camera first.
# Returns the index of the highest-priority entry, or -1 when `entries` is empty.


static func best_index(entries: Array) -> int:
	var best := -1
	# Rank key is (off_frustum ? 1 : 0, distance); lower wins.
	var best_off := 2
	var best_dist := INF
	for i in entries.size():
		var e = entries[i]
		var off := 1 if e.get("off_frustum", false) else 0
		var dist: float = e.get("distance", INF)
		if off < best_off or (off == best_off and dist < best_dist):
			best = i
			best_off = off
			best_dist = dist
	return best
