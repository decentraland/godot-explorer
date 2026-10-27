---
name: figma-to-godot-ui
description: Use when implementing a Figma design as Godot UI in this repo — translating a Figma frame/component into `.gd` + `.tscn` files that match this project's conventions. Covers the Figma MCP workflow (get_design_context, asset download), the design-token translation table (Figma React/Tailwind output → Godot Color()/StyleBoxFlat/theme), the project's reusable chrome patterns (modal backdrop, card, repeated-row), asset simplification heuristics, and the validation loop. Complements `godot-ui-components` (where a file goes) and `i18n` (translation keys) — this skill is about the Figma → `.tscn` translation step itself. Trigger on "implement this Figma", a figma.com URL alongside Godot UI work, or building a new modal/card/component from a design file.
---

# Decentraland Godot Explorer — Figma to Godot UI

This skill is the source of truth for turning a Figma design into working `.gd` + `.tscn` files
in this repo. It's young — first written while implementing the 7-day login streak feature
(issue #3018) — and is meant to absorb lessons from every Figma-to-UI PR after this one. If you
learn something this skill got wrong or missed, **add it here**, don't just fix the one PR.

Read `godot-ui-components` first for *where* a new file goes (atoms/molecules/organisms/pages)
and `i18n` for translation keys — this skill is specifically about step 2: turning a fetched
Figma node into GDScript + scene XML that looks like it was built by this team, not generated.

## Workflow

0. **Confirm which PAGE is canonical before fetching anything.** One Figma file commonly holds
   several pages at different fidelity — a rough mockup page (e.g. "Draft-Proposal") the PM
   sketched early, and the real current source the design team actually iterates on (e.g. an
   "ATLAS" page with explicit redline/spec annotations, and/or a "HIFI-WIP" page with the fully
   composed, current screens). `get_metadata` with no `nodeId` is **not reliable** for listing a
   file's pages (it's returned a single stale/cached page in practice) — instead get the page list
   from wherever the team actually links the file (an issue, a PM comment, a Slack message) and
   fetch each named page's node directly. **Ask which page is canonical before building anything**
   if it isn't obvious — building a whole feature's components against the wrong page means
   redoing structural work (new illustrated showcase elements, different copy throughout,
   different claim-flow entry points), not just tweaking colors. This happened on the streak
   feature: a full first pass was built from "Draft-Proposal," then had to be substantially
   reworked once "ATLAS"/"HIFI-WIP" turned out to be the real source — the trigger was the user
   saying the page name was wrong, not something catchable by any validation step.
1. **Load `figma-design-to-code`** (the Figma MCP server's own skill resource) before calling
   `get_design_context` — it's enforced by the tool itself. Pass
   `skillNames: "resource:figma-design-to-code"` and `clientFrameworks: "godot"`,
   `clientLanguages: "gdscript"`.
2. **Fetch the richest single node that covers a full state**, not every sub-frame. A modal
   with 4 copy variants usually has one Figma frame with the complete body (hero + title +
   content + CTA + footer) — fetch *that* once, and drive the other copy variants from your
   GDScript state machine instead of re-fetching near-duplicate frames. (For the streak modal,
   node `10:1030` — the "Modal" content frame — had everything; the 3 other top-level
   "Welcome-Streak"/"Day-01"/"Complete" frames were the same body with different text, confirmed
   via `get_metadata` text diffs rather than 4 full `get_design_context` calls.)
3. **Read the response as a spec, not as source.** `get_design_context` returns React+Tailwind.
   Never adapt that syntax — extract from it: exact px sizes/spacing, hex colors (also listed
   separately in the response's "styles contained in the design" line), font family+weight+size,
   and asset URLs. Everything below is how those map onto this project.
4. **Check `godot-ui-components`' decision tree and `COMPONENT_AUDIT.md`** before creating
   anything — a "new" card or button is very often `SecondaryButton` / an existing atom already.
5. **Build the `.tscn` by hand** (see Patterns below) — don't try to script-generate node trees;
   this codebase's `.tscn` files are hand-authored and reviewed as such.
6. **Validate** (see Verification below) before considering it done.

## Design-token translation table

| Figma (Tailwind/CSS from `get_design_context`) | This project |
|---|---|
| `text-[#RRGGBB]`, a hex in the "styles contained" list | `Color(r, g, b, a)` with each channel `/255`, e.g. `#DF9CFF` → `Color(0.8745098, 0.6117647, 1, 1)` |
| `font-['Inter:Black']`, `Inter:Bold`, `Inter:Extra_Bold`, `Inter:Regular`, `Inter:Medium` | `res://assets/themes/fonts/inter/Inter-Black.ttf`, `Inter-Bold.ttf` (or `inter_700.ttf` — both exist, `reward_modal.tscn` uses the `inter_*` numeric names, newer scenes tend to use the `Inter-*` named files; either is fine, don't mix within one scene), `Inter-ExtraBold.ttf`, `inter_400.ttf`/`Inter-Regular.ttf`, `Inter-Medium.ttf` |
| `text-[Npx]` | `theme_override_font_sizes/font_size = N` — Figma mobile frames in this file are ~722px wide, which is close enough to this project's actual portrait safe-width (420-ish, same ballpark as other full-width mobile pages) that most px values can be used close to 1:1; sanity-check against a sibling component's existing sizes if a value looks too large. |
| A plain `Label` with a font/size/color (i.e. not a `RichTextLabel`, not dynamically recolored by multiple unrelated callers) | **A `LabelSettings` resource, not scattered `theme_override_fonts`/`theme_override_font_sizes`/`theme_override_colors`.** Give it its own standalone `.tres` (`[gd_resource type="LabelSettings" format=3]` with its own `[ext_resource]` for the font, see `label_settings_title.tres`) next to the component, assign via `label_settings = ExtResource(...)` on the node. Two+ labels sharing identical typography (e.g. a caption and a footer both Inter-Regular-16-pearl) should point at the *same* `.tres` — that's the reuse win, don't fork near-duplicates. **If a script needs to change the label's color per state at runtime** (a day-pill digit that recolors per day state), `label_settings` is a *shared* resource reference — mutating `label.label_settings.font_color` directly recolors every other node using that same `.tres`. Duplicate it once per instance first: `label.label_settings = label.label_settings.duplicate()` in `_ready()`, then mutate the duplicate. A `theme_override_colors/font_color` override is silently **ignored** once `label_settings` is set (Label checks `label_settings` first), so don't mix the two approaches on one node. |
| Mixed-weight text collapsed into a single `Label` (no `RichTextLabel`) — e.g. Figma's "D1" is a Regular "D" + a Bold "1" span | **Use the base/majority weight, not the heaviest one present.** Check which weight applies to the *parent* element (the un-spanned default) vs. which only wraps a short inline override — the override is usually the minority of the string. Picking the heaviest weight "to be safe" is wrong as often as it's right; read the actual nesting. (Caught in review: `streak_day_pill`'s "D1" label was first built Inter-Bold throughout; Figma's base text class was actually `Inter:Regular`, with only the digit span bolded — corrected to Inter-Regular.) |
| A plain `Button`/CTA with no special class | **Nothing extra** — `dcl_theme.tres`'s default `Button` style already renders as the brand red (`Color(1, 0.1804, 0.3333, 1)` ≈ Figma `#FF2D55`). Only set `theme_type_variation` for a *different* look. |
| A visually "secondary/outlined/dark" button | `theme_type_variation = &"SecondaryButton"` (there's also `SecondaryLittle`, `SecondaryOutlinedButton`, `SecondaryOutlinedSmall` for smaller/outlined variants — check `dcl_theme.tres`'s variation list before assuming `SecondaryButton` is the right one for a given size). |
| `rounded-[Npx]` | `corner_radius_top_left/top_right/bottom_right/bottom_left = N` on a `StyleBoxFlat` |
| `border border-[#hex] border-solid` | `border_width_left/top/right/bottom = 1` (or given width) + `border_color = Color(...)` |
| `shadow-[0px_Npx_Mpx_0px_rgba(...)]`, glow, blur, blend-mode layers (`mix-blend-color-dodge`, `mix-blend-hard-light`, stacked blurred borders) | **Deprioritize these — get `bg_color`/`border_color`/`border_width`/`corner_radius` exactly right first, every time, those are almost always explicit Tailwind classes even when the fill isn't.** Glow/blur/blend effects are real visual polish but are not reproducible 1:1 by `StyleBoxFlat` (single soft shadow only, no blend modes, no stacked borders) — skip them in a first pass rather than guessing an approximation, and say so in a comment. If a reviewer wants the glow, that's a deliberate follow-up, not something to fake with invented values. |
| `linear-gradient(...)` background | **`StyleBoxFlat` has no gradient.** Approximate with a solid `bg_color` at the gradient's midpoint color/alpha (average the two stops). Note this as a known simplification in a comment; a `GradientTexture2D`-backed `StyleBoxTexture` is the real fix if a reviewer wants the gradient, but it's more ceremony than a first pass needs. |
| A flat rounded-rectangle exported as its own asset — an outline box, a solid/translucent tinted square, a bordered "frame" that's nothing but fill + border + corner radius + maybe a drop shadow, with no illustrated content inside it | **Don't download it — it's a node, not an asset.** A `Panel`/`PanelContainer` with a `StyleBoxFlat` — `bg_color`, `border_width_*` + `border_color`, `corner_radius_*` are the four properties that matter most and are worth getting exact every time; `shadow_color`/`shadow_size` exist too but are lower priority (see the row above) — reproduces it exactly, stays crisp at any size, and — critically — can be recolored per state (e.g. a different `border_color` per day-pill state) by swapping one resource instead of shipping N near-identical SVGs. Reach for a downloaded asset only once the shape carries real illustrated content (a glyph, a multi-path icon, a gradient fill, baked-in glow/particles). The test: "would a `StyleBoxFlat` alone look identical to this export?" — not "is it rectangular". `streak_day_pill`'s `FUTURE` state was first built as a downloaded `icon_future.svg` (literally just an outlined rounded square) and corrected to a `StyleBoxFlat` on a `PanelContainer` once reviewed — see the worked example below. |
| Multiple stacked decorative SVG layers (sparkle/particle/ray groups) | **Don't reproduce every layer.** Figma exports from a design tool often bake one visual effect into 10-30 tiny absolutely-positioned SVGs (see any "DaysStreak" reward-day icon in the streak design — stars, flares, blend-mode glows...). Check whether the *asset itself* is already a complete, self-contained graphic for the state you need (it usually is — e.g. the "Current-Completed" pill variant exports a single `imgGenerativeObject` SVG that already contains the flame-in-a-circle look). Use that one asset as a single `TextureRect`/icon instead of rebuilding the layer stack. This is the single biggest time sink if you get it wrong — look for the one-asset shortcut first. |
| Mixed bold/regular text in one paragraph (e.g. "**Jump in 7 days** in a row, light up...") | A `RichTextLabel` with `bbcode_enabled = true` and inline `[b][color=#hex]...[/color][/b]` in the `text`. Keep it a single translation key (see `i18n` skill) — don't split into two Label nodes; BBCode tags pass through the translation catalogue untouched per the `i18n` skill's "BBCode is markup" rule. |

## Reusable chrome patterns (copy these, don't reinvent)

**Full-screen celebratory modal** (`reward_modal.tscn` is the canonical example):
```
<Root> (ColorRect, anchors_preset=15, semi-transparent color, your script)
├─ Blur (ColorRect, anchors_preset=15, same canvas_item blur ShaderMaterial, copy verbatim)
└─ VBoxContainer (anchors_preset=15, alignment=1 center)
   └─ PanelContainer (custom_minimum_size, StyleBoxFlat card: bg_color≈Color(0.0863,0.0118,0.2196,1), border, corner_radius=16)
      └─ MarginContainer → VBoxContainer   # your actual content
```
Connect `gui_input` on the root to a `_on_gui_input` that closes on tap-outside (guard it while a
network request is in flight, see `reward_modal.gd`'s `_claiming` flag), and `pressed` on your
primary button to its handler — both as `[connection]` blocks at the bottom of the `.tscn`, not
code-side `.connect()` calls (matches every existing modal in this repo).

**Small embedded card** (`guest_upgrade_card.tscn` is the canonical example):
```
<Root> (MarginContainer, lateral margins e.g. 48/0/48/0 — this is what gives Discover-feed
        cards their consistent side gutter)
└─ <SameName> (PanelContainer — yes, same node name as the root; that's this codebase's
               existing convention for this shape, don't rename it for "clarity")
   └─ MarginContainer (inner padding, e.g. 32/22/32/24)
      └─ VBoxContainer → your labels/button, HSeparator+StyleBoxEmpty as spacers between them
```
If your card's "panel" chrome (border+gradient+shadow) is itself a Figma *component* reused
elsewhere (like the streak feature's "Streak panel"), don't wrap it in a second card — make the
reusable molecule's root **be** the `PanelContainer` with that styling, and have embedding
contexts (a modal, a feed widget) place a plain instance of it directly. Wrapping a component
that already has a card in another card is the most common way this pattern goes wrong.

**Repeated row of runtime-built items** (`pagination.tscn`, or `streak_day_row.gd` for a richer
example with per-item state instead of just selected/unselected):
- The row scene is just the container (`HBoxContainer`/`PanelContainer>HBoxContainer`) with the
  script attached and an `@export var item_scene: PackedScene` — **no static children**.
- The script has a `populate()`/`render()` method that instantiates N children on first call
  (guard with `if not _items.is_empty(): return` so repeated calls don't rebuild) and a
  per-item `set_state()`/`select()` method the row calls per child afterward.
- Each repeated item is its own atom/molecule scene+script, instantiated via
  `item_scene.instantiate()`, never built as raw nodes in the parent script.

**Every `PanelContainer` gets its own standalone `.tres`, full stop — single-state or not.**
Godot's default `PanelContainer` style is a plain engine-gray box with no border/radius; if a
scene leaves `theme_override_styles/panel` unset "because this panel never changes," it renders
as that default the moment anyone opens the scene, not as whatever Figma specified. Before calling
a `.tscn` done, grep it for `type="PanelContainer"` and confirm every single match has a
`theme_override_styles/panel` pointing at a real resource with Figma's `bg_color`/`border_color`/
`border_width_*`/`corner_radius_*` — one-off single-state card chrome (a modal's backdrop panel, a
small reward-thumbnail frame) is not exempt just because it's "simple" or "only used once"; it
still gets its own `.tres` (e.g. `panel_background.tres`, `reward_card_background.tres`) next to
the component, same as a multi-state one. This includes states whose icon/content already has a
baked-in glow or its own visual weight (a flame, a highlighted "today" indicator) — **don't hide
the box for "this one already looks fine without a frame."** Two problems follow from skipping it:
the state loses the frame every sibling state has, breaking visual rhythm in a row of repeated
items, and — because container layout only accounts for *visible* siblings in some setups — a
hidden box can make the remaining visible content (the icon) read as inconsistently sized against
states that do keep their box. Give every state its own `style_<state>.tres`, always visible, even
when the content on top is already a complete illustration.

**When hand-authoring a full-rect anchored child** (`anchors_preset = 15` to fill a parent),
set `offset_left/top/right/bottom = 0.0` explicitly alongside the anchor properties, and give the
child its own matching `custom_minimum_size` if the parent's size isn't guaranteed by a `Container`
auto-layout. `anchors_preset` is a value the *editor* uses to also zero the offsets when you click
an anchor preset button — writing the `.tscn` by hand doesn't get that side effect for free, and
an unset offset doesn't reliably default to `0` the way the editor's preset button guarantees.

**A node whose `StyleBoxFlat` changes per state** (e.g. `streak_day_pill.gd`'s `%Box`, swapped
via `set_state()`): build **one standalone `.tres` per state** next to the component
(`style_<state>.tres` — `[gd_resource type="StyleBoxFlat" format=3]` + one `[resource]` block,
same shape as `close_button_normal.tres`), reference each as an `@export var style_<state>:
StyleBoxFlat` on the script, and wire them in the `.tscn` as `ext_resource`s. **Also set
`theme_override_styles/panel` on the node itself to whichever state is the visual rest/default**
(not just assign the `@export` vars and leave the node's own style empty) — a style only applied
through a runtime `add_theme_stylebox_override()` call means the node renders unstyled the moment
anyone opens the scene standalone or previews it before the owning script's `set_state()` has run
once. Standalone `.tres` files over inline `[sub_resource]` here specifically because they need
to be independently tweakable from the FileSystem dock/Inspector without digging through the
scene's node tree first — this is worth the extra files even for a state that's only used by one
component.

## Asset handling

- **Node over asset for flat rounded-rectangles.** Before downloading *any* export, ask whether
  it's just a filled/bordered/rounded box with nothing illustrated inside — if so it's a
  `StyleBoxFlat`, not a file (see the translation table above). This is the single most common
  unnecessary download in a Figma handoff: icon sheets are full of "empty state" boxes that are
  pure geometry.
- **Reuse before downloading.** If the Figma design uses an obvious placeholder/stock image
  (a stock photo, a generic reward icon) and this repo already has a similar placeholder asset
  for the same *purpose* (e.g. `reward_modal`'s `dog-reward.png` as "the generic reward art"),
  reference the existing file instead of downloading Figma's version. Don't duplicate assets that
  serve the same role.
- **Where new assets go**: colocate with the component that owns them
  (`res://src/ui/components/<tier>/<name>/asset.svg`), unless the asset is generic/shared across
  many components, in which case it goes in `res://assets/ui/`. Match whichever convention the
  *nearest sibling component* uses.
- **Download via the asset URLs `get_design_context`/`get_screenshot` return** (`curl -L -o ...`
  the `https://www.figma.com/api/mcp/asset/...` URL — it expires in 7 days, so don't commit it as
  a link, fetch it now). Never hand-draw or re-derive an asset's path data.
- **`.import` files are automatic.** Don't hand-author them — just reference the new
  `res://.../file.png` from a `.tscn`'s `ext_resource` and run `cargo run -- import-assets`
  (after clearing the class cache, see Verification); Godot generates the `.import` + `uid://`
  on that pass.
- **Standalone `StyleBoxFlat` `.tres`** when a card/pill style is shared by more than one scene
  (see `close_button_normal.tres` for the minimal format: `[gd_resource type="StyleBoxFlat"
  format=3]` + one `[resource]` block). Keep one-off styles as inline `[sub_resource]` blocks in
  the single `.tscn` that uses them instead.

## When a baked asset set supersedes a StyleBoxFlat approach

Sometimes real per-state exports arrive *after* a first pass already built the "node over asset"
version (a `StyleBoxFlat` box + a small illustrated icon on top) — e.g. the designer/user drops in
a full set of baked PNGs (box + icon + glow composited into one image per state) once they're
ready. When that happens, **switch cleanly — delete the `PanelContainer`/`StyleBoxFlat` layer
entirely**, don't layer the new baked asset on top of the old box. One `TextureRect` whose texture
swaps per state replaces both the box and the icon. Check the new asset's *name* for granularity
the old hand-rolled version didn't have — e.g. separate `-current`/`-complete` exports for a
reward slot mean "today is this reward day" and "already past it, unclaimed" are meant to look
different, not share one "unclaimed" state.

**Mismatched canvas sizes across a state set are a real bug, not a rounding quirk.** A "plain/
empty" export is often trimmed tight to its content, while states with a glow/shadow are exported
on a larger canvas to fit the glow's bleed — displayed at a shared node size, the tight one reads
smaller than its siblings. First check whether the core content sizes already match (see below):
if so, this is a pure padding fix, not a rescale:
```sh
magick identify -format "canvas: %wx%h\n" file.png
magick file.png -alpha extract -threshold 60% -trim -format "core: %wx%h+%X+%Y\n" info:   # core, ignores glow
magick file.png -alpha extract -threshold 5%  -trim -format "full: %wx%h+%X+%Y\n" info:   # full bleed incl. glow
magick file.png -gravity center -background none -extent <target_w>x<target_h> file.png   # pad in place
```

**Two separate groups (e.g. "day" pills vs "reward" pills) can each be internally consistent —
same canvas, same core size within the group — and *still* visually mismatch when shown in the
same fixed-size control, if one group's core-to-canvas ratio differs from the other's.** Padding
alone doesn't fix this: padding only grows the canvas around an unchanged core, so if group A's
core fills more of its canvas than group B's does, A still reads larger after both are scaled
(independently) into the same control — matching canvas size is necessary but not sufficient.
Diagnose by measuring *both* canvas and core (with margin = `(canvas − core) / 2`) across every
file in both groups:
```sh
for f in *.png; do
  echo "--- $f ---"
  magick identify -format "canvas: %wx%h\n" "$f"
  magick "$f" -alpha extract -threshold 60% -trim -format "core: %wx%h offset+%X+%Y\n" info:
done
```

**Do not assume the fix is "make both groups' core the same size."** That was tried first on the
streak pills and was wrong — ask (or check the Figma redline/spec annotations) whether the groups
are supposed to read as the *same* size or as *deliberately different* sizes first, before picking
a target. A spec note like `"56 x2"` means "56pt in the design, exported at 2x" → the real pixel
target is `56 * 2 = 112`; a sibling group's note `"46 x2"` → `92`, a *smaller* target, on purpose
(a reward slot reading bigger than a plain day slot was the actual intent here). Once you have the
real per-group target in px, pick the shared canvas as the **larger** target with zero margin (the
group whose core should fill the display control edge-to-edge gets canvas == target, i.e. no
padding at all) — every other group gets the *same* canvas but keeps its own smaller target core,
centered, with real margin. This is what actually makes "neither group looks deformed relative to
the other" true: both canvases are pixel-identical, so `STRETCH_KEEP_ASPECT_CENTERED` scales both
by the same factor into the control, and the *ratio* between the two groups' core sizes is
preserved exactly as specified — not forced to 1:1.
```sh
# target core A = 112 (fills canvas, zero margin) → canvas = 112
# target core B = 92  (same canvas, real margin)   → margin = (112-92)/2 = 10 per side
magick file_A.png -filter Lanczos -resize <scaled>x<scaled> -gravity center -background none -extent 112x112 out_A.png  # scale so core hits 112, then crop flush
magick file_B.png -gravity center -background none -extent 112x112 out_B.png  # core already 92: just re-canvas, no rescale
```
**`-extent` without `-background none` fills new canvas area with opaque white, not
transparency** — silently defeats any later alpha-based measurement (a `-alpha extract -threshold`
trim on the result reports the *entire* canvas as "content", since the padding is now 100% opaque
too). Always pass `-background none` on every `-extent` call on an RGBA asset; if a trim/measure
step afterward reports suspiciously-full-canvas core sizes, this is the first thing to check.
Verify by re-running the measurement loop on the output and confirming core sizes/margins now land
within a couple px of each other across *both* groups — a couple px of residual variance from
compounding two resize passes is fine, don't chase pixel-perfect equality.

If canvas or core sizes don't match *within* one group, that's a real export inconsistency to
flag, not something to paper over by force-resizing — ask before rescaling actual content.

## i18n while authoring the scene

Decide **per label** whether it's auto-translating (static per-state copy, raw key as `text`) or
mode-2/`auto_translate_mode = 2` (dynamic, text assigned from GDScript) — see the `i18n` skill for
the full rule. In practice for a Figma-driven component: anything whose copy depends on a
GDScript state machine (title/subtitle/CTA text that changes per modal state) is mode-2 and set
via `TranslationKey.new("KEY").text()`/`.format(...)` in the script; anything that's the same
string regardless of state (a footer caption, a static label) is a plain auto-translating node
with the raw key typed directly into the `.tscn`'s `text =`.

**The `NOTIFICATION_TRANSLATION_CHANGED` re-translate hook the `i18n` skill requires can fire
before `_ready()`.** A mode-2 node's re-render function typically touches several `@onready`
vars (one per label/child it updates) — if `_notification()` calls that render function
unconditionally, an early `NOTIFICATION_TRANSLATION_CHANGED` (Godot can send it the moment the
node enters the tree, ahead of `@onready` assignment) crashes with `Invalid call: Nonexistent
function '<fn>' in base 'Nil'` on whichever `@onready` var the render function touches first —
this will NOT be caught by `check-gdscript` (it's a runtime ordering bug, not a parse error), only
by actually running the scene. Guard it:
```gdscript
func _notification(what: int) -> void:
	if what == NOTIFICATION_TRANSLATION_CHANGED and is_node_ready():
		_render()
```
`is_node_ready()` is the built-in, idiomatic check — don't hand-roll a `_ready_fired` bool.

## Verification (every time, before calling it done)

```sh
rm -f godot/.godot/global_script_class_cache.cfg godot/.godot/uid_cache.bin
cargo run -- import-assets      # registers new class_names, imports new assets, rebuilds .translation
cargo run -- check-gdscript     # 0 errors — also the only way to catch a typo'd %UniqueName
gdformat godot/ && gdlint godot/
python3 tools/i18n/extract_strings.py --check && python3 tools/i18n/format_csv.py --check
```
`check-gdscript` loading your new `.tscn` without a "Cannot open file" error is the strongest
signal the node tree actually matches every `@onready var ... = %UniqueName` in the script — if
you rename a node in the `.tscn`, this is what catches the drift, not a visual check.

**This only validates structure, not pixels.** Nothing here confirms the scene actually *looks*
right — run the client (`cargo run -- run`) and navigate to it, or ask whoever's doing the device
pass to compare against the Figma screenshot. Say so explicitly if you skipped that step; don't
claim visual parity you haven't checked.

## Known simplifications in the first pass (streak feature, issue #3018) — revisit here

- Gradients are flattened to a solid midpoint color (`streak_panel_background.tres`). Fine for
  v1; a real gradient would need `StyleBoxTexture` + a baked/generated gradient texture.
  *(Note added during the implementation PR — update/remove once design signs off on the flat
  version or someone implements the real gradient.)*
- Reward-day pill icons use a single flat illustrated asset per state (flame/check/gift) rather
  than the full glow/blur/stars/particle layer stack Figma's component sheet defines for them.
  Visually close, not pixel-identical — fine per this PR's "I'll touch it up" review pass. The
  pill's plain rounded-rect *box* behind those icons (`FUTURE`/`DONE`/`REWARD_LOCKED` states) is
  **not** simplified this way — it was first built as a downloaded SVG and corrected to a
  `StyleBoxFlat` on a `PanelContainer` (see the translation table's "node over asset" rule above)
  once reviewed. Keep that distinction: simplify *illustrated* content, never substitute a node
  for *geometric* content. Second correction on the same box: the first `StyleBoxFlat` pass
  *invented* `bg_color`/`shadow_*` values instead of reading them off the real node (verify by
  fetching a `get_screenshot` of the exact per-state node, not just skimming the Tailwind dump —
  `DONE`'s box turned out to be a solid filled purple square, not the transparent-bordered box the
  first pass assumed). When a state's fill comes from a decorative background image/blend-mode
  glow rather than a flat `bg-[...]` class (no CSS color to lift), per the user: **deprioritize
  reproducing the glow/blur/blend effect itself, but still set a real flat `bg_color` and the
  exact `border_color`/`border_width`/`corner_radius`** — pick the fill from the project's
  existing palette (e.g. the brand purple `#691FA9` already used elsewhere) rather than leaving it
  at a default/guessed value. Border color, width and corner radius are almost always given
  explicitly as Tailwind classes (`border-2 border-[#hex] rounded-[Npx]`) even when the fill
  isn't — get those exact, every time.
- Multi-weight "D1" pill label (regular "D" + bold digit) is rendered as a single bold run
  instead of two inline weights, since Godot `Label` can't mix font weights without a
  `RichTextLabel`. Acceptable for a 2-character label; switch to `RichTextLabel` + BBCode if a
  reviewer wants the exact mixed weight.

## Open questions for next time this skill gets used

- We don't yet have a repeatable way to turn a Figma `linear-gradient` into a `GradientTexture2D`
  — if the next design leans harder on gradients than streak did, that's worth solving properly
  instead of re-flattening every time.
- No established convention yet for *how much* of a decorative Figma animation (confetti,
  particle bursts) is worth reproducing with `CPUParticles2D`/`AnimationPlayer` vs skipping —
  `reward_modal.tscn` has a real example (rays rotation + confetti burst) worth studying before
  deciding case-by-case.
