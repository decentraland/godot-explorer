//! Keeps the TextShape faces' rendered MSDF glyphs on disk between launches, so characters seen
//! in an earlier session are not rendered again. Saved when the app pauses or closes, restored on
//! a worker the first time a face is used; `glyph_prewarm` holds new text until restores finish.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use godot::classes::file_access::{CompressionMode, ModeFlags};
use godot::classes::image::Format;
use godot::classes::{
    DirAccess, FileAccess, FontFile, Image, RenderingServer, TextServer, TextServerManager,
    WorkerThreadPool,
};
use godot::prelude::*;

use super::glyph_prewarm;
use crate::content::font::{MSDF_PIXEL_RANGE, MSDF_SIZE};
use crate::engine::local_task::defer_from_any_thread;

const DIR: &str = "user://glyph_cache";
const FORMAT: i64 = 1;
/// Texture pages saved per font cache; glyphs on later pages render again next launch.
const MAX_PAGES: i64 = 8;

struct Tracked {
    font: Gd<FontFile>,
    key: String,
    /// Glyph count on disk (or in the import pre-render).
    saved: usize,
}

thread_local! {
    static TRACKED: RefCell<HashMap<String, Tracked>> = RefCell::new(HashMap::new());
    static RESTORING: Cell<u32> = const { Cell::new(0) };
}

fn server() -> Option<Gd<TextServer>> {
    TextServerManager::singleton().get_primary_interface()
}

fn size() -> Vector2i {
    Vector2i::new(MSDF_SIZE, 0)
}

/// Registers a TextShape face: marks its import pre-rendered Latin as rendered and restores the
/// glyphs saved by a previous launch. Call before the face renders anything.
pub fn track(font: &Gd<FontFile>, name: &str) {
    if TRACKED.with(|tracked| tracked.borrow().contains_key(name)) {
        return;
    }
    let base = glyph_count(font);
    let engine_hash = godot::classes::Engine::singleton()
        .get_version_info()
        .get("hash")
        .map(|hash| hash.to_string())
        .unwrap_or_default();
    let key = format!(
        "{FORMAT}|{name}|{}|{base}|{MSDF_SIZE}|{MSDF_PIXEL_RANGE}|{engine_hash}",
        font.get_data().len(),
    );
    TRACKED.with(|tracked| {
        tracked.borrow_mut().insert(
            name.to_owned(),
            Tracked {
                font: font.clone(),
                key: key.clone(),
                saved: base,
            },
        );
    });
    glyph_prewarm::seed_rendered(
        font.find_variation_ex(&VarDictionary::new()).done(),
        glyph_prewarm::latin_chars(),
    );
    let path = format!("{DIR}/{name}.bin");
    if !FileAccess::file_exists(&path) {
        return;
    }
    RESTORING.with(|restoring| restoring.set(restoring.get() + 1));
    let name = name.to_owned();
    let read_task = Callable::from_sync_fn("glyph_cache_read", move |_| {
        let data = read(&path, &key);
        let name = name.clone();
        defer_from_any_thread(&[data.to_variant()], move |args| {
            let data = args.first().and_then(|v| v.try_to::<VarDictionary>().ok());
            apply(&name, data.filter(|data| !data.is_empty()));
        });
        Variant::nil()
    });
    WorkerThreadPool::singleton()
        .add_task_ex(&read_task)
        .description("GlyphDiskCache")
        .done();
}

pub fn is_restoring() -> bool {
    RESTORING.with(|restoring| restoring.get() > 0)
}

/// Writes every tracked face whose glyph count grew since it was last saved or restored.
pub fn save_all() {
    DirAccess::make_dir_recursive_absolute(DIR);
    TRACKED.with(|tracked| {
        for (name, entry) in tracked.borrow_mut().iter_mut() {
            let count = glyph_count(&entry.font);
            if count <= entry.saved {
                continue;
            }
            let path = format!("{DIR}/{name}.bin");
            let Some(mut file) = FileAccess::open_compressed_ex(&path, ModeFlags::WRITE)
                .compression_mode(CompressionMode::ZSTD)
                .done()
            else {
                godot_warn!("glyph cache: cannot write {name}");
                return;
            };
            let mut payload = VarDictionary::new();
            payload.set("key", entry.key.as_str());
            payload.set("caches", &snapshot(&entry.font));
            file.store_var(&payload.to_variant());
            file.close();
            entry.saved = count;
        }
    });
}

/// Plain (non-emboldened, untransformed) MSDF caches of `font`: variation coordinates and RID.
fn caches(font: &Gd<FontFile>) -> Vec<(VarDictionary, Rid)> {
    (0..font.get_cache_count())
        .filter(|&i| font.get_embolden(i) == 0.0 && font.get_transform(i) == Transform2D::IDENTITY)
        .map(|i| {
            let coords = font.get_variation_coordinates(i);
            let rid = font.find_variation_ex(&coords).done();
            (coords, rid)
        })
        .collect()
}

fn glyph_count(font: &Gd<FontFile>) -> usize {
    let Some(server) = server() else {
        return 0;
    };
    caches(font)
        .iter()
        .map(|(_, rid)| server.font_get_glyph_list(*rid, size()).len())
        .sum()
}

fn snapshot(font: &Gd<FontFile>) -> VarArray {
    let mut out = VarArray::new();
    let Some(server) = server() else {
        return out;
    };
    let size = size();
    for (coords, rid) in caches(font) {
        let list = server.font_get_glyph_list(rid, size);
        let mut page_glyph: HashMap<i64, i64> = HashMap::new();
        for &glyph in list.as_slice() {
            let page = server.font_get_glyph_texture_idx(rid, size, glyph as i64);
            page_glyph.entry(page).or_insert(glyph as i64);
        }
        let mut pages = VarArray::new();
        for i in 0..server.font_get_texture_count(rid, size).min(MAX_PAGES) {
            let mut image = server.font_get_texture_image(rid, size, i);
            if image.is_none() {
                // Full pages only live on the GPU: read them back.
                if let Some(&glyph) = page_glyph.get(&i) {
                    let texture = server.font_get_glyph_texture_rid(rid, size, glyph);
                    image = RenderingServer::singleton().texture_2d_get(texture);
                }
            }
            let Some(image) = image else {
                break;
            };
            let mut page = VarArray::new();
            page.push(&image.get_width().to_variant());
            page.push(&image.get_height().to_variant());
            page.push(&(image.get_format().ord() as i64).to_variant());
            page.push(&image.get_data().to_variant());
            page.push(&server.font_get_texture_offsets(rid, size, i).to_variant());
            pages.push(&page.to_variant());
        }
        let mut glyphs = VarArray::new();
        for &glyph in list.as_slice() {
            let glyph = glyph as i64;
            let page = server.font_get_glyph_texture_idx(rid, size, glyph);
            if page as usize >= pages.len() {
                continue;
            }
            let mut entry = VarArray::new();
            entry.push(&glyph.to_variant());
            entry.push(
                &server
                    .font_get_glyph_advance(rid, size.x as i64, glyph)
                    .to_variant(),
            );
            entry.push(&server.font_get_glyph_offset(rid, size, glyph).to_variant());
            entry.push(&server.font_get_glyph_size(rid, size, glyph).to_variant());
            entry.push(&server.font_get_glyph_uv_rect(rid, size, glyph).to_variant());
            entry.push(&page.to_variant());
            glyphs.push(&entry.to_variant());
        }
        let chars: PackedStringArray = glyph_prewarm::rendered_chars(rid)
            .iter()
            .map(|c| GString::from(c.to_string().as_str()))
            .collect();
        let mut cache = VarDictionary::new();
        cache.set("coords", &coords);
        cache.set("pages", &pages);
        cache.set("glyphs", &glyphs);
        cache.set("chars", &chars);
        out.push(&cache.to_variant());
    }
    out
}

/// Worker thread: read and decode only. An empty dictionary means nothing usable.
fn read(path: &str, key: &str) -> VarDictionary {
    let data = FileAccess::open_compressed_ex(path, ModeFlags::READ)
        .compression_mode(CompressionMode::ZSTD)
        .done()
        .and_then(|mut file| {
            let data = file.get_var();
            file.close();
            data.try_to::<VarDictionary>().ok()
        })
        .filter(|data| {
            data.get("key")
                .is_some_and(|stored| stored.to_string() == key)
        });
    let Some(data) = data else {
        DirAccess::remove_absolute(path);
        return VarDictionary::new();
    };
    let Some(caches) = data
        .get("caches")
        .and_then(|caches| caches.try_to::<VarArray>().ok())
    else {
        return VarDictionary::new();
    };
    for cache in caches.iter_shared() {
        let Ok(mut cache) = cache.try_to::<VarDictionary>() else {
            continue;
        };
        let mut images = VarArray::new();
        if let Some(pages) = cache
            .get("pages")
            .and_then(|pages| pages.try_to::<VarArray>().ok())
        {
            for page in pages.iter_shared() {
                let Ok(page) = page.try_to::<VarArray>() else {
                    continue;
                };
                let image = Image::create_from_data(
                    page.at(0).to::<i32>(),
                    page.at(1).to::<i32>(),
                    false,
                    Format::from_ord(page.at(2).to::<i64>() as i32),
                    &page.at(3).to::<PackedByteArray>(),
                );
                images.push(&image.to_variant());
            }
        }
        cache.set("images", &images);
    }
    data
}

/// Main thread: restore only into an untouched cache; anything rendered meanwhile would be
/// overwritten.
fn apply(name: &str, data: Option<VarDictionary>) {
    let entry = TRACKED.with(|tracked| {
        tracked
            .borrow()
            .get(name)
            .map(|entry| (entry.font.clone(), entry.saved))
    });
    if let (Some((font, saved)), Some(data), Some(mut server)) = (entry, data, server()) {
        if glyph_count(&font) == saved {
            restore(&font, &data, &mut server);
            let count = glyph_count(&font);
            TRACKED.with(|tracked| {
                if let Some(entry) = tracked.borrow_mut().get_mut(name) {
                    entry.saved = count;
                }
            });
        }
    }
    let finished = RESTORING.with(|restoring| {
        restoring.set(restoring.get().saturating_sub(1));
        restoring.get() == 0
    });
    if finished {
        glyph_prewarm::resume_waiting();
    }
}

fn restore(font: &Gd<FontFile>, data: &VarDictionary, server: &mut Gd<TextServer>) {
    let size = size();
    let Some(caches) = data
        .get("caches")
        .and_then(|caches| caches.try_to::<VarArray>().ok())
    else {
        return;
    };
    for cache in caches.iter_shared() {
        let Ok(cache) = cache.try_to::<VarDictionary>() else {
            continue;
        };
        let field = |name: &str| cache.get(name);
        let (Some(coords), Some(images), Some(pages), Some(glyphs), Some(chars)) = (
            field("coords").and_then(|v| v.try_to::<VarDictionary>().ok()),
            field("images").and_then(|v| v.try_to::<VarArray>().ok()),
            field("pages").and_then(|v| v.try_to::<VarArray>().ok()),
            field("glyphs").and_then(|v| v.try_to::<VarArray>().ok()),
            field("chars").and_then(|v| v.try_to::<PackedStringArray>().ok()),
        ) else {
            continue;
        };
        let rid = font.find_variation_ex(&coords).done();
        for (i, image) in images.iter_shared().enumerate() {
            let Ok(image) = image.try_to::<Gd<Image>>() else {
                continue;
            };
            server.font_set_texture_image(rid, size, i as i64, &image);
            let offsets = pages
                .get(i)
                .and_then(|page| page.try_to::<VarArray>().ok())
                .and_then(|page| page.get(4))
                .and_then(|offsets| offsets.try_to::<PackedInt32Array>().ok());
            if let Some(offsets) = offsets {
                server.font_set_texture_offsets(rid, size, i as i64, &offsets);
            }
        }
        for glyph in glyphs.iter_shared() {
            let Ok(g) = glyph.try_to::<VarArray>() else {
                continue;
            };
            let id = g.at(0).to::<i64>();
            server.font_set_glyph_advance(rid, size.x as i64, id, g.at(1).to::<Vector2>());
            server.font_set_glyph_offset(rid, size, id, g.at(2).to::<Vector2>());
            server.font_set_glyph_size(rid, size, id, g.at(3).to::<Vector2>());
            server.font_set_glyph_uv_rect(rid, size, id, g.at(4).to::<Rect2>());
            server.font_set_glyph_texture_idx(rid, size, id, g.at(5).to::<i64>());
        }
        glyph_prewarm::seed_rendered(
            rid,
            chars
                .as_slice()
                .iter()
                .filter_map(|c| c.to_string().chars().next()),
        );
    }
}
