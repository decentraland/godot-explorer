## Keeps the TextShape faces' rendered MSDF glyphs on disk between launches, so characters seen
## in an earlier session are not rendered again. Saved when the app pauses or closes, restored on
## a worker the first time a face is used; GlyphPrewarm holds new text until restores finish.
class_name GlyphDiskCache
extends RefCounted

const DIR := "user://glyph_cache"
const FORMAT := 1
## Texture pages saved per font cache; glyphs on later pages render again next launch.
const MAX_PAGES := 8


class _Saver:
	extends Node

	func _notification(what: int) -> void:
		if what == NOTIFICATION_APPLICATION_PAUSED or what == NOTIFICATION_WM_CLOSE_REQUEST:
			GlyphDiskCache.save_all()


# Name -> { "font": FontFile, "key": String, "saved": int (glyph count on disk) }.
static var _tracked: Dictionary = {}
static var _restoring := 0
static var _saver: Node


## Registers a TextShape face: marks its import pre-rendered Latin as rendered and restores
## the glyphs saved by a previous launch. Call before the face renders anything.
static func track(font: FontFile, name: String) -> void:
	if _tracked.has(name):
		return
	var base := _glyph_count(font)
	var key := (
		"%d|%s|%d|%d|%d|%d|%s"
		% [
			FORMAT,
			name,
			font.data.size(),
			base,
			int(TextLayout.MSDF_SIZE),
			int(TextLayout.MSDF_PIXEL_RANGE),
			Engine.get_version_info().get("hash", ""),
		]
	)
	_tracked[name] = {"font": font, "key": key, "saved": base}
	GlyphPrewarm.seed_rendered(font.find_variation({}), GlyphPrewarm.latin_chars())
	_ensure_saver()
	var path := "%s/%s.bin" % [DIR, name]
	if not FileAccess.file_exists(path):
		return
	_restoring += 1
	WorkerThreadPool.add_task(_read.bind(name, path, key), false, "GlyphDiskCache")


static func is_restoring() -> bool:
	return _restoring > 0


## Writes every tracked face whose glyph count grew since it was last saved or restored.
static func save_all() -> void:
	DirAccess.make_dir_recursive_absolute(DIR)
	for name in _tracked:
		var entry: Dictionary = _tracked[name]
		var count := _glyph_count(entry["font"])
		if count <= entry["saved"]:
			continue
		var file := FileAccess.open_compressed(
			"%s/%s.bin" % [DIR, name], FileAccess.WRITE, FileAccess.COMPRESSION_ZSTD
		)
		if file == null:
			push_warning("GlyphDiskCache: cannot write %s" % name)
			return
		file.store_var({"key": entry["key"], "caches": _snapshot(entry["font"])})
		file.close()
		entry["saved"] = count


static func _ensure_saver() -> void:
	if _saver != null:
		return
	_saver = _Saver.new()
	_saver.name = "GlyphDiskCacheSaver"
	(Engine.get_main_loop() as SceneTree).root.add_child.call_deferred(_saver)


static func _size() -> Vector2i:
	return Vector2i(int(TextLayout.MSDF_SIZE), 0)


# Plain (non-emboldened, untransformed) MSDF caches of `font` and their variation coordinates.
static func _caches(font: FontFile) -> Array:
	var caches := []
	for i in font.get_cache_count():
		if font.get_embolden(i) != 0.0 or font.get_transform(i) != Transform2D.IDENTITY:
			continue
		var coords := font.get_variation_coordinates(i)
		caches.append([coords, font.find_variation(coords)])
	return caches


static func _glyph_count(font: FontFile) -> int:
	var ts := TextServerManager.get_primary_interface()
	var count := 0
	for cache in _caches(font):
		count += ts.font_get_glyph_list(cache[1], _size()).size()
	return count


static func _snapshot(font: FontFile) -> Array:
	var ts := TextServerManager.get_primary_interface()
	var size := _size()
	var out := []
	for cache in _caches(font):
		var rid: RID = cache[1]
		var pages := []
		var page_glyph := {}
		for glyph in ts.font_get_glyph_list(rid, size):
			page_glyph.get_or_add(ts.font_get_glyph_texture_idx(rid, size, glyph), glyph)
		for i in mini(ts.font_get_texture_count(rid, size), MAX_PAGES):
			var image := ts.font_get_texture_image(rid, size, i)
			if image == null and page_glyph.has(i):
				# Full pages only live on the GPU: read them back.
				image = RenderingServer.texture_2d_get(
					ts.font_get_glyph_texture_rid(rid, size, page_glyph[i])
				)
			if image == null:
				break
			(
				pages
				. append(
					[
						image.get_width(),
						image.get_height(),
						image.get_format(),
						image.get_data(),
						ts.font_get_texture_offsets(rid, size, i),
					]
				)
			)
		var glyphs := []
		for glyph in ts.font_get_glyph_list(rid, size):
			var page := ts.font_get_glyph_texture_idx(rid, size, glyph)
			if page >= pages.size():
				continue
			(
				glyphs
				. append(
					[
						glyph,
						ts.font_get_glyph_advance(rid, size.x, glyph),
						ts.font_get_glyph_offset(rid, size, glyph),
						ts.font_get_glyph_size(rid, size, glyph),
						ts.font_get_glyph_uv_rect(rid, size, glyph),
						page,
					]
				)
			)
		(
			out
			. append(
				{
					"coords": cache[0],
					"pages": pages,
					"glyphs": glyphs,
					"chars": GlyphPrewarm.rendered_chars(rid),
				}
			)
		)
	return out


# Worker thread: read and decode only.
static func _read(name: String, path: String, key: String) -> void:
	var data = null
	var file := FileAccess.open_compressed(path, FileAccess.READ, FileAccess.COMPRESSION_ZSTD)
	if file != null:
		data = file.get_var()
		file.close()
	if data is Dictionary and data.get("key") == key:
		for cache in data["caches"]:
			cache["images"] = cache["pages"].map(
				func(p: Array) -> Image:
					return Image.create_from_data(p[0], p[1], false, p[2], p[3])
			)
	else:
		data = null
		DirAccess.remove_absolute(path)
	_apply.call_deferred(name, data)


static func _apply(name: String, data) -> void:
	var entry: Dictionary = _tracked[name]
	var font: FontFile = entry["font"]
	# Restore only into an untouched cache; anything rendered meanwhile would be overwritten.
	if data != null and _glyph_count(font) == entry["saved"]:
		var ts := TextServerManager.get_primary_interface()
		var size := _size()
		for cache in data["caches"]:
			var rid := font.find_variation(cache["coords"])
			for i in cache["images"].size():
				ts.font_set_texture_image(rid, size, i, cache["images"][i])
				ts.font_set_texture_offsets(rid, size, i, cache["pages"][i][4])
			for g in cache["glyphs"]:
				ts.font_set_glyph_advance(rid, size.x, g[0], g[1])
				ts.font_set_glyph_offset(rid, size, g[0], g[2])
				ts.font_set_glyph_size(rid, size, g[0], g[3])
				ts.font_set_glyph_uv_rect(rid, size, g[0], g[4])
				ts.font_set_glyph_texture_idx(rid, size, g[0], g[5])
			GlyphPrewarm.seed_rendered(rid, cache["chars"])
		entry["saved"] = _glyph_count(font)
	_restoring -= 1
	if _restoring == 0:
		GlyphPrewarm.resume_waiting()
