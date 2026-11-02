## Renders a TextShape's missing glyphs on a worker thread before its text is applied, so the
## first use of a character (an MSDF glyph costs ~20-30 ms on phones) doesn't stall the frame.
## Text whose characters are all rendered is applied immediately.
class_name GlyphPrewarm
extends RefCounted

## Codepoints Inter SemiBold, the default face, pre-renders at import (`preload` in its .import).
const LATIN_RANGES: Array = [[0x20, 0x7E]]

# Cache key -> { character: true } of glyphs known to be rendered.
static var _warm: Dictionary = {}
# RID id -> whether that font is MSDF (asked once, before any worker can hold its lock).
static var _msdf: Dictionary = {}
# Renderer instance id -> { "ref": WeakRef, "latest": Array, "task": int }.
static var _jobs: Dictionary = {}
# Renderer instance ids waiting for GlyphDiskCache to finish restoring.
static var _waiting: Dictionary = {}


## Calls `on_ready` once every character of `text` is rendered in `fonts` (including their
## fallbacks): right away when they already are, otherwise after a worker renders them.
## Only the latest request per `owner` is applied.
static func request(
	owner: Object, fonts: Array, text: String, font_size: int, on_ready: Callable
) -> void:
	var id := owner.get_instance_id()
	var state: Dictionary = _jobs.get(id, {})
	if state.is_empty():
		state = {"ref": weakref(owner), "task": -1}
		_jobs[id] = state
	state["latest"] = [fonts, text, font_size, on_ready]
	if state["task"] != -1:
		return
	if GlyphDiskCache.is_restoring():
		_waiting[id] = true
		return
	var rids := _rids(fonts)
	var missing := _missing(rids, text, font_size)
	if missing.is_empty():
		_jobs.erase(id)
		on_ready.call()
		return
	var job := {
		"owner_id": id,
		"fonts": fonts,
		"rids": rids,
		"msdf": rids.map(func(rid: RID) -> bool: return _msdf[rid.get_id()]),
		"chars": missing,
		"size": font_size,
		"done": [],
		"request": state["latest"],
	}
	state["task"] = WorkerThreadPool.add_task(_render.bind(job), false, "GlyphPrewarm")


## Marks `chars` as rendered in `rid` when its cache really holds them (import pre-render,
## disk cache restore).
static func seed_rendered(rid: RID, chars: Array) -> void:
	var ts := TextServerManager.get_primary_interface()
	var size := Vector2i(int(TextLayout.MSDF_SIZE), 0)
	var cached := {}
	for glyph in ts.font_get_glyph_list(rid, size):
		cached[glyph] = true
	if cached.is_empty():
		return
	var warm: Dictionary = _warm.get_or_add(_key(rid, true, 0), {})
	for c in chars:
		if cached.has(ts.font_get_glyph_index(rid, size.x, c.unicode_at(0), 0)):
			warm[c] = true


## The characters `rid` is known to have rendered (saved by GlyphDiskCache).
static func rendered_chars(rid: RID) -> Array:
	return _warm.get(_key(rid, true, 0), {}).keys()


static func latin_chars() -> Array:
	var chars := []
	for r in LATIN_RANGES:
		for cp in range(r[0], r[1] + 1):
			chars.append(String.chr(cp))
	return chars


## Retries the requests that waited for GlyphDiskCache to restore.
static func resume_waiting() -> void:
	var ids := _waiting.keys()
	_waiting.clear()
	for id in ids:
		_retry(id)


static func _rids(fonts: Array) -> Array:
	var ts := TextServerManager.get_primary_interface()
	var rids := []
	for font in fonts:
		if font == null:
			continue
		for rid in font.get_rids():
			if rid in rids:
				continue
			rids.append(rid)
			if not _msdf.has(rid.get_id()):
				_msdf[rid.get_id()] = ts.font_is_multichannel_signed_distance_field(rid)
	return rids


static func _missing(rids: Array, text: String, font_size: int) -> Array:
	var missing := {}
	for c in text:
		if c.unicode_at(0) <= 0x20 or missing.has(c):
			continue
		var warm := false
		for rid in rids:
			if _warm.get(_key(rid, _msdf[rid.get_id()], font_size), {}).has(c):
				warm = true
				break
		if not warm:
			missing[c] = true
	return missing.keys()


static func _key(rid: RID, msdf: bool, font_size: int) -> String:
	# MSDF glyphs are rendered once for every size; bitmap glyphs (emoji) per size.
	return str(rid.get_id()) if msdf else "%d_%d" % [rid.get_id(), font_size]


# Worker thread: only TextServer calls, one character per call so each font lock is short.
static func _render(job: Dictionary) -> void:
	var ts := TextServerManager.get_primary_interface()
	var rids: Array = job["rids"]
	for c in job["chars"]:
		var cp: int = c.unicode_at(0)
		var target := 0
		for i in rids.size():
			if ts.font_has_char(rids[i], cp):
				target = i
				break
		var msdf: bool = job["msdf"][target]
		var size := Vector2i(int(TextLayout.MSDF_SIZE) if msdf else job["size"], 0)
		ts.font_render_range(rids[target], size, cp, cp)
		job["done"].append([_key(rids[target], msdf, job["size"]), c])
	_finish.call_deferred(job)


static func _finish(job: Dictionary) -> void:
	for pair in job["done"]:
		_warm.get_or_add(pair[0], {})[pair[1]] = true
	var state: Dictionary = _jobs.get(job["owner_id"], {})
	if state.is_empty():
		return
	WorkerThreadPool.wait_for_task_completion(state["task"])
	state["task"] = -1
	_retry(job["owner_id"], job["request"])


static func _retry(id: int, rendered: Array = []) -> void:
	var state: Dictionary = _jobs.get(id, {})
	if state.is_empty():
		return
	var owner: Object = state["ref"].get_ref()
	if owner == null:
		_jobs.erase(id)
		return
	var latest: Array = state["latest"]
	# Text changing faster than glyphs render would otherwise never be shown: show the text
	# this job rendered while the newer one renders.
	if not rendered.is_empty() and rendered != latest:
		if not _missing(_rids(latest[0]), latest[1], latest[2]).is_empty():
			rendered[3].call()
	request(owner, latest[0], latest[1], latest[2], latest[3])
