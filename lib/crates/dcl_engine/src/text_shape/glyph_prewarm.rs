//! Renders a TextShape's missing glyphs on a worker thread before its text is applied, so the
//! first use of a character (an MSDF glyph costs ~20-30 ms on phones) does not stall the frame.
//! Text whose characters are all rendered is applied immediately.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use godot::classes::{Font, TextServer, TextServerManager, WorkerThreadPool};
use godot::prelude::*;

use super::glyph_cache;
use super::node::DclTextShape;
use crate::content::font::MSDF_SIZE;
use crate::engine::local_task::defer_from_any_thread;

/// Codepoints Inter SemiBold, the default face, pre-renders at import.
const LATIN: std::ops::RangeInclusive<u32> = 0x20..=0x7E;

thread_local! {
    // Cache key -> characters known to be rendered.
    static WARM: RefCell<HashMap<String, HashSet<char>>> = RefCell::new(HashMap::new());
    // Font RID -> whether it is MSDF, asked on the main thread before a worker holds its lock.
    static MSDF: RefCell<HashMap<u64, bool>> = RefCell::new(HashMap::new());
    // Plates waiting for the disk cache to finish restoring.
    static WAITING: RefCell<HashSet<InstanceId>> = RefCell::new(HashSet::new());
}

pub struct GlyphRequest {
    pub fonts: Vec<Gd<Font>>,
    pub text: String,
    pub size: i32,
}

fn server() -> Option<Gd<TextServer>> {
    TextServerManager::singleton().get_primary_interface()
}

/// MSDF glyphs are rendered once for every size; bitmap glyphs (emoji) per size.
fn key(rid: Rid, msdf: bool, size: i32) -> String {
    if msdf {
        rid.to_u64().to_string()
    } else {
        format!("{}_{size}", rid.to_u64())
    }
}

fn rids(fonts: &[Gd<Font>], server: &Gd<TextServer>) -> Vec<Rid> {
    let mut rids = Vec::new();
    for font in fonts {
        for rid in font.get_rids().iter_shared() {
            if rids.contains(&rid) {
                continue;
            }
            rids.push(rid);
            MSDF.with(|msdf| {
                msdf.borrow_mut()
                    .entry(rid.to_u64())
                    .or_insert_with(|| server.font_is_multichannel_signed_distance_field(rid));
            });
        }
    }
    rids
}

fn is_msdf(rid: Rid) -> bool {
    MSDF.with(|msdf| msdf.borrow().get(&rid.to_u64()).copied().unwrap_or(false))
}

fn missing(rids: &[Rid], text: &str, size: i32) -> Vec<char> {
    let mut missing = Vec::new();
    WARM.with(|warm| {
        let warm = warm.borrow();
        for c in text.chars() {
            if c as u32 <= 0x20 || missing.contains(&c) {
                continue;
            }
            let rendered = rids.iter().any(|&rid| {
                warm.get(&key(rid, is_msdf(rid), size))
                    .is_some_and(|chars| chars.contains(&c))
            });
            if !rendered {
                missing.push(c);
            }
        }
    });
    missing
}

/// Whether every character of `request` is rendered.
pub fn is_ready(request: &GlyphRequest) -> bool {
    let Some(server) = server() else {
        return true;
    };
    let all = rids(&request.fonts, &server);
    missing(&all, &request.text, request.size).is_empty()
}

/// Ready when every character is rendered. Otherwise starts a worker for the missing ones (or
/// waits for the disk cache restore) and returns false: the plate gets `glyphs_rendered`
/// afterwards. Nothing starts while `task` holds an unfinished job.
pub fn request(owner: InstanceId, task: &mut Option<i64>, request: &GlyphRequest) -> bool {
    let Some(server) = server() else {
        return true;
    };
    if task.is_some() {
        return false;
    }
    if glyph_cache::is_restoring() {
        WAITING.with(|waiting| waiting.borrow_mut().insert(owner));
        return false;
    }
    let all = rids(&request.fonts, &server);
    let chars = missing(&all, &request.text, request.size);
    if chars.is_empty() {
        return true;
    }
    let msdf: Vec<bool> = all.iter().map(|&rid| is_msdf(rid)).collect();
    let (server_id, size) = (server.instance_id(), request.size);
    let job = Callable::from_sync_fn("glyph_prewarm", move |_| {
        let done = render(server_id, &all, &msdf, &chars, size);
        defer_from_any_thread(&[], move |_| finish(owner, done));
        Variant::nil()
    });
    *task = Some(
        WorkerThreadPool::singleton()
            .add_task_ex(&job)
            .description("GlyphPrewarm")
            .done(),
    );
    false
}

/// Worker thread: only TextServer calls, one character per call so each font lock is short.
fn render(
    server_id: InstanceId,
    rids: &[Rid],
    msdf: &[bool],
    chars: &[char],
    size: i32,
) -> Vec<(String, char)> {
    let Ok(mut server) = Gd::<TextServer>::try_from_instance_id(server_id) else {
        return Vec::new();
    };
    let mut done = Vec::with_capacity(chars.len());
    for &c in chars {
        let cp = c as i64;
        let target = rids
            .iter()
            .position(|&rid| server.font_has_char(rid, cp))
            .unwrap_or(0);
        let glyph_size = Vector2i::new(if msdf[target] { MSDF_SIZE } else { size }, 0);
        server.font_render_range(rids[target], glyph_size, cp, cp);
        done.push((key(rids[target], msdf[target], size), c));
    }
    done
}

fn finish(owner: InstanceId, done: Vec<(String, char)>) {
    WARM.with(|warm| {
        let mut warm = warm.borrow_mut();
        for (key, c) in done {
            warm.entry(key).or_default().insert(c);
        }
    });
    if let Ok(mut plate) = Gd::<DclTextShape>::try_from_instance_id(owner) {
        plate.bind_mut().glyphs_rendered();
    }
}

/// Marks `chars` as rendered in `rid` when its cache really holds them (import pre-render,
/// disk cache restore).
pub fn seed_rendered(rid: Rid, chars: impl IntoIterator<Item = char>) {
    let Some(server) = server() else {
        return;
    };
    let size = Vector2i::new(MSDF_SIZE, 0);
    let cached: HashSet<i32> = server
        .font_get_glyph_list(rid, size)
        .as_slice()
        .iter()
        .copied()
        .collect();
    if cached.is_empty() {
        return;
    }
    WARM.with(|warm| {
        let mut warm = warm.borrow_mut();
        let entry = warm.entry(key(rid, true, 0)).or_default();
        for c in chars {
            let glyph = server.font_get_glyph_index(rid, MSDF_SIZE as i64, c as i64, 0);
            if cached.contains(&(glyph as i32)) {
                entry.insert(c);
            }
        }
    });
}

/// The characters `rid` is known to have rendered (saved by the disk cache).
pub fn rendered_chars(rid: Rid) -> Vec<char> {
    WARM.with(|warm| {
        warm.borrow()
            .get(&key(rid, true, 0))
            .map(|chars| chars.iter().copied().collect())
            .unwrap_or_default()
    })
}

pub fn latin_chars() -> Vec<char> {
    LATIN.filter_map(char::from_u32).collect()
}

/// Retries the plates that waited for the disk cache to restore.
pub fn resume_waiting() {
    let owners: Vec<InstanceId> = WAITING.with(|waiting| waiting.borrow_mut().drain().collect());
    for owner in owners {
        if let Ok(mut plate) = Gd::<DclTextShape>::try_from_instance_id(owner) {
            plate.bind_mut().glyphs_rendered();
        }
    }
}

/// Joins a finished worker task.
pub fn join(task: i64) {
    WorkerThreadPool::singleton().wait_for_task_completion(task);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin_covers_printable_ascii() {
        let chars = latin_chars();
        assert_eq!(chars.first(), Some(&' '));
        assert_eq!(chars.last(), Some(&'~'));
        assert_eq!(chars.len(), 0x7E - 0x20 + 1);
    }

    #[test]
    fn bitmap_glyphs_are_keyed_per_size() {
        let rid = Rid::new(7);
        assert_eq!(key(rid, true, 10), key(rid, true, 20));
        assert_ne!(key(rid, false, 10), key(rid, false, 20));
    }
}
