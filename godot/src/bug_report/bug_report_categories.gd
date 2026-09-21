class_name BugReportCategories
extends RefCounted

## Issue-type options for the native bug report form (issue #2652).
##
## The UUIDs are Intercom option ids on the shared "Bug Report" ticket type, taken
## from the Unity Explorer client (`BugReportIssueTypes.cs`) so mobile tickets land
## in the same buckets as desktop ones. The proxy validates them: sending a label
## instead of a UUID, or an id that isn't declared on the ticket type, gets the
## whole ticket rejected.
##
## This is the subset of Unity's 16 that the mobile design keeps — Voice Chat,
## Hangouts/Events, Outfits, Communities, Map & Minimap, Rewards, Gifting and Scene
## are deliberately absent. Order matches the Figma dropdown.

## `label` is the ENGLISH wording and must stay English: it composes the Intercom
## ticket title, which support reads. `key` is what the player sees in the dropdown.
## The two are separate on purpose — translating the ticket title would make the
## Intercom queue unsearchable for a support team working in English.
##
# i18n-keys: BUG_REPORT_CATEGORY_*
const CATEGORIES: Array[Dictionary] = [
	{
		"label": "Performance",
		"key": "BUG_REPORT_CATEGORY_PERFORMANCE",
		"uuid": "84d3e47f-396f-40be-bb93-a8b36196cf97"
	},
	{
		"label": "Crash / Freeze",
		"key": "BUG_REPORT_CATEGORY_CRASH",
		"uuid": "10ab00f9-e944-4a7f-8b75-c8bf4e4ff270"
	},
	{
		"label": "Chat",
		"key": "BUG_REPORT_CATEGORY_CHAT",
		"uuid": "b2db7b2e-3634-4c9d-9f55-b732bfe41319"
	},
	{
		"label": "Streaming",
		"key": "BUG_REPORT_CATEGORY_STREAMING",
		"uuid": "dbceaef0-c69c-409b-afb3-5e9523a4dec5"
	},
	{
		"label": "Friends",
		"key": "BUG_REPORT_CATEGORY_FRIENDS",
		"uuid": "4d3a9289-da5a-47a6-a3e0-772effdd78f0"
	},
	{
		"label": "Wearables/Emotes",
		"key": "BUG_REPORT_CATEGORY_WEARABLES",
		"uuid": "e4e9abb6-8304-48eb-a622-b2516e0a1719"
	},
	{
		"label": "Profile",
		"key": "BUG_REPORT_CATEGORY_PROFILE",
		"uuid": "30f4a7a7-6366-4e05-9393-d1419a5b4008"
	},
	{
		"label": "Other",
		"key": "BUG_REPORT_CATEGORY_OTHER",
		"uuid": "30b90385-7138-4d42-99aa-87eeb1c85619"
	},
]


## Translation keys in dropdown order. The caller resolves them — DropdownList
## items carry finished text, matching how settings.gd feeds the graphics presets.
static func keys() -> PackedStringArray:
	var out := PackedStringArray()
	for category in CATEGORIES:
		out.append(category["key"])
	return out


## UUID for a dropdown index, or "" when the index is out of range (nothing picked).
static func uuid_at(index: int) -> String:
	if index < 0 or index >= CATEGORIES.size():
		return ""
	return CATEGORIES[index]["uuid"]


## Label for a dropdown index, or "" when out of range.
static func label_at(index: int) -> String:
	if index < 0 or index >= CATEGORIES.size():
		return ""
	return CATEGORIES[index]["label"]
