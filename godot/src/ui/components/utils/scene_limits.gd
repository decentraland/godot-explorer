class_name SceneLimits
extends RefCounted

## Preview-only scene-stats limits.
##
## Default model is FIXED_ABSOLUTE: scene-wide budgets seeded from the Genesis
## Plaza asset audit (issue #2368). Every number here is a TUNABLE default —
## adjust in one place. A PER_PARCEL model (the canonical Decentraland
## per-parcel formula) is provided as a stub for future use.

enum Model { FIXED_ABSOLUTE, PER_PARCEL }

const MB: int = 1024 * 1024

## metric_key -> { label, group, unit, soft, hard, [inverse] }
##   group: "scene" (per-scene) or "global" (whole-app, informational)
##   unit:  "count" or "bytes"
##   inverse: higher-is-better (e.g. fps) — coloring flips, no overpass
const FIXED: Dictionary = {
	"triangles":
	{"label": "Triangles", "group": "scene", "unit": "count", "soft": 1500000, "hard": 2000000},
	"entities":
	{"label": "Entities", "group": "scene", "unit": "count", "soft": 5000, "hard": 10000},
	"bodies":
	{"label": "Meshes (bodies)", "group": "scene", "unit": "count", "soft": 3000, "hard": 5000},
	"geometries":
	{"label": "Geometries", "group": "scene", "unit": "count", "soft": 1000, "hard": 2000},
	"materials":
	{"label": "Materials", "group": "scene", "unit": "count", "soft": 500, "hard": 1000},
	"textures": {"label": "Textures", "group": "scene", "unit": "count", "soft": 400, "hard": 800},
	"colliders":
	{"label": "Colliders", "group": "scene", "unit": "count", "soft": 1000, "hard": 2000},
	"content_size":
	{
		"label": "Content size",
		"group": "scene",
		"unit": "bytes",
		"soft": 100 * MB,
		"hard": 300 * MB
	},
	"texture_vram":
	{
		"label": "Texture VRAM",
		"group": "global",
		"unit": "bytes",
		"soft": 512 * MB,
		"hard": 1024 * MB
	},
	"video_mem":
	{
		"label": "Video memory",
		"group": "global",
		"unit": "bytes",
		"soft": 768 * MB,
		"hard": 1536 * MB
	},
	"static_mem":
	{
		"label": "CPU memory",
		"group": "global",
		"unit": "bytes",
		"soft": 512 * MB,
		"hard": 1024 * MB
	},
	"draw_calls":
	{"label": "Draw calls", "group": "global", "unit": "count", "soft": 1000, "hard": 2000},
	"fps":
	{"label": "FPS", "group": "global", "unit": "count", "soft": 50, "hard": 30, "inverse": true},
}

## Display order (per-scene group first, then whole-app).
const ORDER: Array = [
	"triangles",
	"entities",
	"bodies",
	"geometries",
	"materials",
	"textures",
	"colliders",
	"content_size",
	"texture_vram",
	"video_mem",
	"static_mem",
	"draw_calls",
	"fps",
]

## Active limit model. Fixed absolutes by default (per the approved plan).
static var model: int = Model.FIXED_ABSOLUTE


## Ordered metric metadata; each entry is a writable copy with a "key" field.
static func metric_order() -> Array:
	var out: Array = []
	for key in ORDER:
		var meta: Dictionary = (FIXED[key] as Dictionary).duplicate()
		meta["key"] = key
		out.append(meta)
	return out


## Returns {soft, hard} for a metric. `parcels` is ignored under FIXED_ABSOLUTE
## but consumed by the PER_PARCEL stub so the model can be switched later.
static func limits_for(key: String, parcels: int) -> Dictionary:
	var base: Dictionary = FIXED.get(key, {})
	if model == Model.PER_PARCEL:
		var hard: int = _per_parcel_hard(key, maxi(parcels, 1))
		if hard > 0:
			return {"soft": int(float(hard) * 0.75), "hard": hard}
	return {"soft": int(base.get("soft", 0)), "hard": int(base.get("hard", 0))}


## Canonical Decentraland per-parcel hard limits (stub for the PER_PARCEL model).
static func _per_parcel_hard(key: String, parcels: int) -> int:
	var log2: float = log(float(parcels) + 1.0) / log(2.0)
	match key:
		"triangles":
			return 10000 * parcels
		"entities":
			return 200 * parcels
		"bodies":
			return 300 * parcels
		"materials":
			return int(floor(log2 * 20.0))
		"textures":
			return int(floor(log2 * 10.0))
		"geometries":
			return int(floor(log2 * 200.0))
		_:
			return 0
