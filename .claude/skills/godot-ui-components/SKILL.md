---
name: godot-ui-components
description: Use when creating, moving, or refactoring UI in this repo (`godot/src/ui/`). Encodes the Atomic Design tier system (atoms / molecules / organisms / layouts / pages) established in PR #2021 / issue #1876, the decision tree for placing a new component, the file naming conventions, and the cross-references that must be updated when a component moves.
---

# Decentraland Godot Explorer — UI Componentization

This skill is the source of truth for *where new UI files go* and *how to refactor existing UI* in `godot/src/ui/`. It exists because the team adopted an Atomic Design layout in PR #2021 (issue [#1876](https://github.com/decentraland/godot-explorer/issues/1876)) and reviews now enforce it.

If you are creating a new UI screen, button, dialog, list item, panel, modal, toast, or anything that touches `godot/src/ui/` — read this first.

## The structure

```
godot/src/ui/
├── explorer.gd / explorer.tscn      # app shell (top-level; do NOT add new files here)
├── components/
│   ├── atoms/                       # smallest single-purpose controls
│   │   ├── buttons/                 # custom_button, animated_button, fav_button, …
│   │   ├── inputs/                  # dcl_line_edit, dcl_text_edit, line_edit_custom
│   │   ├── images/                  # async_image, profile_picture, circle_rect, …
│   │   └── controls/                # custom_slider, pagination, loading_spinner, …
│   ├── molecules/                   # 2–3 atoms working as a unit
│   └── organisms/                   # complex composite sections (navbar, modal, dialogs, …)
├── layouts/                         # page-skeleton wrappers (responsive containers, safe-area)
└── pages/                           # full screens with state and data
```

See `godot/src/ui/COMPONENT_AUDIT.md` for the **full migration map** — every existing component listed with its current path. Use it as the lookup table when you need a real example for a given tier.

## Where does a new component go? — decision tree

Walk these in order. Stop at the first match.

1. **Is it a whole screen / route / persistent panel that owns its own state and data?**
   → `pages/<feature>/`
   Examples: `pages/profile/profile.tscn`, `pages/settings/settings.tscn`, `pages/chat/chat_panel.tscn`.

2. **Is it a reusable page skeleton / layout primitive (containers, safe-area wrappers, responsive switchers)?**
   → `layouts/`
   Examples: `layouts/safe_margin_container.gd`, `layouts/responsive_container.gd`, `layouts/orientation_container.gd`.

3. **Is it a complex composite UI section: navbar / modal / dialog / popup / panel / settings form / inventory grid / debug overlay?**
   → `components/organisms/<name>/`
   Examples: `components/organisms/navbar/`, `components/organisms/modal/`, `components/organisms/dialogs/`, `components/organisms/notifications/`.

4. **Is it 2–3 atoms composed into a small reusable unit: a search bar, a labeled icon button with dropdown, a list-row item, a card?**
   → `components/molecules/<name>/`
   Examples: `components/molecules/search_bar/`, `components/molecules/button_profile/`, `components/molecules/wearable_item/`, `components/molecules/place_item/`.

5. **Is it a single-purpose control with no children of significance (button variant, single input field, single image wrapper, single slider)?**
   → `components/atoms/<group>/<name>/`
   Examples: `components/atoms/buttons/custom_button/`, `components/atoms/inputs/dcl_line_edit.gd`, `components/atoms/images/async_image/`, `components/atoms/controls/custom_slider/`.

If you can't decide between two tiers, prefer the **lower** tier (atom over molecule, molecule over organism). Pure logic helpers that aren't UI go in `components/utils/` — not in atoms.

## Naming and file layout

Per-component:
```
components/<tier>/<group?>/<component_name>/
├── <component_name>.gd
├── <component_name>.gd.uid       # auto-generated; commit it
└── <component_name>.tscn
```

- Directory name: `snake_case`. Matches the script/scene stem 1:1.
- Class declaration: `class_name PascalCase` in the `.gd` (e.g. `class_name DclLineEdit`).
- The `.gd.uid` stub is required — Godot generates it on import; **commit it** alongside the script.
- A few atoms ship as loose files (no wrapping dir) when there's no associated scene — see `components/atoms/inputs/dcl_line_edit.gd`. Prefer a wrapping dir for new components.

## When to extract a new molecule/atom from a page

Trigger: you're about to copy-paste a Control subtree into two pages, or you notice a third caller of the same composite.

1. Pick the tier with the decision tree above (atom → molecule → organism).
2. Create the new directory and move the scene/script with `git mv` so history is preserved.
3. Update every `res://src/ui/<old>/...` reference (`.tscn`, `.tres`, `.gd`, `.gdshader`) by literal string substitution. Rust callers can also hardcode `res://` paths — `rg 'res://src/ui/<old>' lib/` to catch them.
4. Delete `godot/.godot/global_script_class_cache.cfg` and `godot/.godot/uid_cache.bin`, then regenerate via `cargo run -- import-assets`. Godot caches stale class paths and the validator will fail otherwise.
5. Run `cargo run -- check-gdscript` — must report 0 errors.
6. Run `gdformat godot/ && gdlint godot/` — longer Atomic Design paths often push `preload(...)` past the column budget and need a reflow.
7. If you edited any Rust file with a hardcoded `res://src/ui/...` path, run `cd lib && cargo fmt --all`.

## Path-reference gotchas

- **`uid://` is not enough.** Godot 4 stores both `uid://...` and `path="res://..."` in `[ext_resource]`. The path string is authoritative for builds; rewrite it explicitly.
- **`.import` files have `source_file=`.** When you move a PNG/JPG, the sibling `.import` file's `source_file=` line must be updated by hand (Godot's headless `--import` does not always regenerate it).
- **Rust hardcoded paths exist.** Some Rust files `load("res://src/ui/...")` — e.g. `lib/src/scene_runner/rpc_calls/handle_restricted_actions.rs` references modal/dialog scenes. Grep `lib/` for the old path before declaring the move done.
- **Trailing slash matters.** When doing literal substitution on directory prefixes, append `/` to old and new (`res://src/ui/profile/` → `res://src/ui/pages/profile/`) so `profile_settings/` doesn't accidentally get rewritten as `pages/profile_settings/`.
- **`class_name` is path-independent.** Moving a script does not require renaming its `class_name`. Renames are a separate concern and out of scope for a move.

## Anti-patterns — do not do these

- **Don't add new files at the top of `components/`.** Every new component goes inside `atoms/`, `molecules/`, or `organisms/`. The bare `components/` level is for the tier directories only.
- **Don't add `components/<feature_name>/`.** Feature-grouped folders (`components/profile/`, `components/discover/`) are pages now — they live under `pages/<feature>/`.
- **Don't introduce a duplicate of an existing atom/molecule.** Check `COMPONENT_AUDIT.md` § "Duplication audit" first — there are already known duplicate sets (text inputs, button family, toast variants, modal vs dialog, profile buttons, list-item scaffolds). If your new component would be a 7th button variant, push back and pick from the canonical set instead.
- **Don't mix tiers within a single feature folder.** A page is for state/data; its building blocks live in `organisms/` or `molecules/`. Don't put `pages/profile/profile_button.gd` if `button_profile/` is a real molecule used by other pages too.
- **Don't put pure-logic helpers in `atoms/`.** Atoms are UI controls. Behavior helpers (debouncers, throttlers, formatters) stay in `components/utils/`.

## Verification checklist (before committing a UI change)

- [ ] New files live in the right tier per the decision tree.
- [ ] Directory + script + scene + `.gd.uid` all share the same `snake_case` stem.
- [ ] `class_name` (if any) is `PascalCase`.
- [ ] Every `preload`/`load`/`[ext_resource path=...]` reference resolves; no `res://src/ui/<old>/` strings remain (rg across `godot/` *and* `lib/`).
- [ ] `cargo run -- check-gdscript` → 0 errors.
- [ ] `gdformat godot/` and `gdlint godot/` pass.
- [ ] If a Rust file was touched: `cd lib && cargo fmt --all && cargo clippy -- -D warnings`.
- [ ] For PNG/JPG moves: the sibling `.import` file's `source_file=` matches the new path.

## Why this exists

PR #2021 reorganized 97 directories and rewrote ~194 files of `res://` references to land this structure. Reviewers will reject new code that re-creates the old flat layout (`components/<feature>/<scene>.tscn`) or duplicates atoms that already exist. The audit doc (`godot/src/ui/COMPONENT_AUDIT.md`) is the migration map *and* the catalog of known duplication candidates — consult it as a reference, not just a history doc.
