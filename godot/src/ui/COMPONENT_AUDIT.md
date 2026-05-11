# UI Component Audit

This document accompanies the Atomic Design reorganization of `godot/src/ui/` (issue [#1876](https://github.com/decentraland/godot-explorer/issues/1876)). It captures duplication candidates, misplaced files, and follow-up cleanup items that were **deliberately left out of the reorganization PR** so each change can be reviewed independently.

The reorganization PR moved files into the new tier structure and rewrote `res://` references. It did not change behavior, merge components, rename classes, or delete unused code.

## New structure

```
godot/src/ui/
├── explorer.gd / explorer.tscn        # app shell (top-level entry point)
├── components/
│   ├── atoms/                         # smallest single-purpose controls
│   │   ├── buttons/                   # 9 single-button components
│   │   ├── controls/                  # sliders, pagination, label variants, etc.
│   │   ├── images/                    # texture / image wrappers
│   │   └── inputs/                    # LineEdit / TextEdit wrappers
│   ├── molecules/                     # 2-3 atoms working as a unit
│   └── organisms/                     # complex composite sections
├── layouts/                           # responsive / safe-area wrappers
└── pages/                             # full screens with state and data
```

The full mapping that was applied is in the git history of this PR — every move was performed with `git mv` so file history is preserved.

## Duplication / unification candidates

Each row below is a candidate for a follow-up unification PR. Estimates assume only the noted files change.

### 1. Single-line text inputs

- **Files**: `components/atoms/inputs/dcl_line_edit/`, `components/atoms/inputs/line_edit_custom/`
- **Proposal**: Diff the two. Both extend `LineEdit`. Keep `dcl_line_edit` as the canonical wrapper unless `line_edit_custom` has features (placeholder behavior, focus styling, validation hooks) that `dcl_line_edit` lacks. If the deltas are additive, port them into `dcl_line_edit` and replace `line_edit_custom` instantiations.
- **Before unifying**: list every scene that instances `line_edit_custom.tscn` (it's referenced by `class_name LineEditCustom`, so also grep that) and verify visual/behavior parity in each.
- **Risk**: Low. Both classes are leaf inputs; visual regression possible if themes differ.

### 2. Button family

- **Files** (under `components/atoms/buttons/`): `animated_button`, `button_touch_action`, `calendar_button`, `custom_button`, `custom_touch_button`, `fav_button`, `menu_navbar_button`, `mini_map_button`, `static_button`
- **Plus** molecule-level buttons: `button_profile`, `profile_icon_button`
- **Proposal**: Map each button's usage. Most are likely thin wrappers around `Button` with different styling. The likely canonical set is 3–4 variants: `Primary`, `Secondary`, `Icon`, `Ghost` (plus a touch-optimized variant for mobile that may already be `custom_touch_button`). Define those as themes/styles on a single `DclButton` rather than separate scenes.
- **Before unifying**: build a usage matrix (which button is used where, with what theme override) before touching any files. The wide adoption makes this a multi-PR effort, not a single sweep.
- **Risk**: Medium. Visual regressions across the whole app are easy to miss.

### 3. Toast / notification variants

- **Files** (under `components/organisms/notifications/`): `alert_toast.tscn`, `low_spec_toast.tscn`, plus the base notification scenes
- **Proposal**: Confirm whether `low_spec_toast` is a hardware-detection variant or obsolete. If it's a styling variant of the standard toast, collapse into one `Toast` with a `variant` enum (alert / info / low_spec). Keep the existing trigger APIs.
- **Risk**: Low. Toasts are leaf, short-lived UI.

### 4. Modal vs Dialog systems

- **Files**:
  - `components/organisms/modal/` — `modal.tscn`, `modal_manager.gd`, `travel_modal.gd`
  - `components/organisms/dialogs/` — `dialog_stack.tscn`, `confirm_dialog.tscn`, `nft_dialog.tscn`
- **Proposal**: Two managers coexist. `modal_manager` is used for general modal content (loaded dynamically); `dialog_stack` handles confirmation and NFT prompts as a stack. Decide whether they're actually specializations (modal = full-screen content, dialog = confirmation prompt) or duplicated infrastructure. If specializations, document the boundary; if duplicates, pick one manager and migrate callers.
- **Before unifying**: trace every `modal_manager.show_*()` call and every `dialog_stack.add_child()` call. The behavior contracts (signal flow, dismiss timing, escape handling) must be compared before merging.
- **Risk**: Medium. Modal/dialog logic is wired into many flows (auth, NFT prompts, settings confirms, travel prompts).

### 5. Profile button surfaces

- **Files**:
  - `components/atoms/images/profile_picture/profile_picture.tscn` — bare avatar image
  - `components/molecules/button_profile/navbar_profile_button.tscn` — animated profile button (in navbar)
  - `components/molecules/profile_icon_button/profile_icon_button.tscn` — icon-style profile button
- **Proposal**: Clarify whether these are size/density variants of one concept (small icon, medium nav button, bare image) or three distinct components. If variants, define a `ProfileButton` with size/style props; if distinct, document where each is used and why.
- **Risk**: Low. All three are leaf widgets.

### 6. List item / row scaffolds

- **Files**:
  - `components/molecules/carrousel_page_item/` — discover carousel card
  - `components/molecules/place_item/` — search result place row
  - `components/molecules/wearable_item/` — equipment grid item
  - `components/organisms/notifications/notification_item.tscn` — notification panel row
  - `components/organisms/emote_editor/emote_editor_item.tscn` — emote wheel item
  - `components/organisms/social/social_item/` — friends list row
- **Proposal**: Extract a shared `BaseListItem` molecule with slots for thumbnail, title, subtitle, trailing action. Each existing item becomes a thin wrapper that fills the slots. This is more about reuse than dedup — the items aren't redundant, but they share padding/selection/touch-target patterns that could live in one place.
- **Risk**: Medium. Touches a lot of feature scenes.

## Misplaced files

These were found in `godot/src/ui/components/` but are **not UI components**. They were intentionally **left in place** by the reorganization PR because moving non-UI code is out of scope. They should be relocated in a follow-up:

- `godot/src/ui/components/floating_island_walls.gd` — `class_name FloatingIslandWalls extends Node3D`. Manages 3D collision walls around scene bounds. Referenced from `lib/`-side game logic (`godot/src/logic/scene_fetcher.gd`). Suggested home: `godot/src/decentraland_components/` or `godot/src/logic/`.
- `godot/src/ui/components/invisible_wall.gd` — `class_name InvisibleWall extends StaticBody3D`. Used by `floating_island_walls.gd`. Same suggested home as above.

## Unused / dead components found during the audit

Tagging these as candidates for removal after a usage sweep — **not removed in this PR**:

- `godot/src/ui/components/atoms/images/right_arrow/right_arrow.tscn` — a plain `TextureRect` of an arrow icon. No `res://...right_arrow.tscn` reference found in the codebase. Likely either inlined in callers or dead.

## Things this PR explicitly did NOT do

- Merge or rename any duplicate components (see table above)
- Consolidate the modal/dialog systems
- Rename any `class_name` declarations (paths changed; class registrations stayed identical)
- Change scene composition, node trees, or signal contracts
- Refactor themes / styling
- Delete `floating_island_walls.gd` or `invisible_wall.gd`
- Delete `right_arrow/`
- Touch GDScript or `.tscn` formatting beyond path substitutions

Every follow-up unification listed above should land as its own PR linked back to issue #1876.
