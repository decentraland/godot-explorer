---
name: figma
description: Use whenever a Figma design is the source of truth for a task in this repo — an issue with a design attached, a review comment pointing at a frame, a new screen, a restyle, a new icon. Covers the one hard prerequisite (the Figma MCP plugin must be installed AND authorized, and only a human can do either), how to read a design, how to decompose it onto the atoms/molecules/organisms/layouts/pages tiers in `godot/src/ui/`, the Figma→Godot property mapping (auto-layout, colors, fonts, StyleBoxes, theme variations), SVG icon export and `.import`/`uid://` rules, and the `Figma:` reference convention that leaves a resolvable link behind. Trigger on a figma.com URL, "the design", "the mock", "per Figma", node-id, frame name, "new icon", `editor_description`, `dcl_theme.tres`, `assets/ui/`.
---

# Decentraland Godot Explorer — Figma designs

Three things go wrong on every design task here, and all are avoidable.

**You cannot see a design without the MCP plugin.** There is no fallback. Searching the repo,
reading a layer name out of an old comment, inferring from a screenshot — none of these are
reading the design, and a confident guess is worse than saying you are blocked. Both installing
and authorizing the plugin need a human with a browser.

**Screenshots are never a source of truth — in either direction.** Not the design's, not the
app's. A rendered image cannot tell you a colour, a size, a spacing or a corner radius: alpha
blends with whatever is behind it, device scaling and PNG compression shift pixels, and a
translucent panel over a bright object reads as an entirely different colour than it is. Both
halves of every comparison must come from data:

| | Read it from | Never from |
|---|---|---|
| The design | `get_design_context` / `get_variable_defs` — the returned values | the returned screenshot's pixels |
| The built UI | the live scene tree (**mobile-dev-debug-tool** `eval`) | `adb screencap`, a device photo, the editor viewport |

Screenshots are for orientation only — checking you are looking at the right frame, or spotting
that something is obviously missing. The moment a number or a colour matters, measure it. This has
already produced a false positive here: a 50%-black pill over an orange cube looked exactly like a
grey plate that had in fact already been removed, and acting on the screenshot would have meant
"fixing" a bug that did not exist.

**Nothing gets recorded afterwards.** The repo holds 12 Figma references and **11 cannot be
resolved** — frame names with no node-id, a node-id with no file, a file named only by its title.
Exactly one, in `joypad.tscn`, carries a real file key. No `figma.com` URL exists anywhere in the
tree. So every design question starts from zero and ends at a human. The last section fixes that
for the component you touch.

The governing rule: **the design is authoritative for appearance; this repo is authoritative for
structure.** Which tier a component lives in, how its text is translated, where its assets go and
what they are named are decided here, not in Figma.

## Step 0 — the plugin, before anything else

Check which Figma tools you actually have, and dispatch:

| What you see | What it means | Do this |
|---|---|---|
| `mcp__figma__*` read tools available | installed and authorized | proceed |
| only `mcp__plugin_figma_figma__authenticate` / `…__complete_authentication` | installed, **not** authorized | run the OAuth flow below |
| no `mcp__*figma*` tools at all | **not installed** | ask the user to install it, then authorize |

**To authorize:** call `mcp__plugin_figma_figma__authenticate`, give the user the URL it returns,
and stop. When they finish, the tools appear on their own. The redirect lands on
`http://localhost:<port>/callback` and the browser usually shows a connection error — that is
expected; ask them to copy the **full URL from the address bar** and pass it to
`mcp__plugin_figma_figma__complete_authentication`.

**To install:** the user runs `/plugin` in Claude Code and adds the Figma plugin, then authorizes
as above.

Ask plainly and early — one message, naming which of the two steps is needed. Do not start
implementing UI "provisionally" while blocked: work built against an imagined design is thrown
away, and it reads to a reviewer as though the design was consulted.

## Step 1 — get the link

- If the task supplied a Figma URL, use it.
- If not, **ask for it** — naming both halves: the file *and* the frame or component. "The design
  for X" is not a link.
- Never assume a node-id belongs to whichever file another comment happens to mention.
  `screenshot_slot.gd:50` records `Figma 26:428` with no file — that is unresolvable, not a hint.
- `skocZRe2lV9IjqV4rF6EYs` (from `joypad.tscn:20`) is the one file key in this repo confirmed
  real. It is the joypad/controls file, and only for the frames named there. Do not treat it as
  the project's design system.

## Step 2 — read the design

The plugin ships its own skills for the mechanics: load **figma-design-to-code** before
`get_design_context`, and **figma-use** before `use_figma`. Those own the API; this skill owns
what happens to the result.

Pull everything in one pass — a second round trip costs another human hand-off. You need: frame
size and the artboard height it was measured against, auto-layout direction / gap / padding, text
styles, fills and strokes **per state**, corner radii, icon slots, component variants with their
property names, and which states exist at all (normal / hover / pressed / disabled / focus).

## Step 3 — decompose onto the tiers

A frame is not one component. `godot/src/ui/` is organised by Atomic Design (PR #2021 / issue
[#1876](https://github.com/decentraland/godot-explorer/issues/1876)):

| Tier | Holds | Usually in Figma | Example |
|---|---|---|---|
| `components/atoms/` | smallest single-purpose controls, split `buttons/` `inputs/` `images/` `controls/` | one component + its variants | `atoms/buttons/fav_button/` |
| `components/molecules/` | 2–3 atoms working as a unit | a small repeated group | `molecules/place_item/` |
| `components/organisms/` | complex composite sections | a panel or section of a frame | `organisms/modal/` |
| `layouts/` | responsive / safe-area wrappers — **scripts only, no `.tscn`** | constraints and resizing rules | `layouts/safe_margin_container.gd` |
| `pages/` | full screens with state and data, grouped by feature | the whole screen frame | `pages/discover/` |

- A screen frame is normally **one page**; its sections become organisms, its repeated rows
  molecules, its controls atoms. Split the design on paper before writing a scene.
- On a tie, **prefer the lower tier**.
- Read `godot/src/ui/COMPONENT_AUDIT.md` before creating anything. Nine button atoms already
  exist and a single `DclButton` is the proposed unification — do not become the tenth.
- Naming: directory, `.gd`, `.tscn` and `.gd.uid` share one `snake_case` stem; `class_name` is
  `PascalCase`.

The **godot-ui-components** skill owns the full decision tree, the extraction procedure and the
`res://` path-rewriting gotchas. Defer to it rather than re-deriving them.

## Step 4 — map the properties

| Figma | Godot, in this repo |
|---|---|
| auto-layout vertical / horizontal | `VBoxContainer` / `HBoxContainer`; gap → `theme_override_constants/separation` |
| auto-layout padding | `MarginContainer`, or `SafeMarginContainer` at screen edges |
| fill container / hug contents | `size_flags_horizontal = 3` / `custom_minimum_size` |
| absolute position on a fixed frame | `godot/src/ui/layouts/figma_margins.gd` — scales margins measured off a 720px-tall artboard |
| fill / stroke colour | a raw `Color(r,g,b,a)` in **normalized floats** — there is no token layer |
| corner radius, border, background | a `StyleBoxFlat` `.tres` under `godot/assets/themes/` (~90 exist — reuse before adding) |
| text style | Inter Bold / Medium / Regular in `godot/assets/themes/fonts/inter/`; size via `<Type>/font_sizes/font_size` |
| component variant | `theme_type_variation` (e.g. the `BlackButton/…` keys in the theme) |
| any label text | a translation key — see the **i18n** skill |

Two things that trip people up here:

**Colours have no indirection.** They exist only as literals inside `theme.tres` /
`dcl_theme.tres` and inline in scenes. A Figma hex must be converted exactly —
`#FCFCFC` → `Color(0.988235, 0.988235, 0.988235, 1)`. Never eyeball it, and never introduce a
"palette" singleton as a side effect of one design task.

**"Which theme" is a real question.** `theme.tres` is the project-wide default
(`project.godot` → `theme/custom`) and is used by 21 scenes; `dcl_theme.tres` is used by 44.
Match the surface you are editing rather than assuming the default.

Safe-area edges use `SafeMarginContainer` with `godot/src/ui/layouts/hud_margins.tres`, never
hand-tuned offsets — a design measured on one device is not a layout.

## Step 5 — icons and assets

- Export a **square SVG** at the intended logical size (24 / 32 / 48 viewBox). SVG is the
  default. PNG is for raster art only: PNGs import as VRAM-compressed `CompressedTexture2D`,
  which is wrong for small crisp icons.
- Shared icons → `godot/assets/ui/<feature>/`. Single-use → a co-located `icons/` directory
  beside the page. Both patterns exist and both are fine.
- `.svg.import` produces a `DPITexture`; the only meaningful knob is `base_scale`. It is split
  roughly 50/50 between `1.0` and `2.0` across 236 files with no convention, so **set it
  deliberately**: `2.0` when the art was authored at 1× and needs HiDPI headroom.
- **Run `cargo run -- import-assets` before referencing the file.** `.tscn` references are
  `uid://`-keyed and the uid does not exist until Godot has imported it.
- Theme-bound control icons (checkbox, switch, radio) belong in the theme as
  `<Type>/icons/<state> = ExtResource(...)`, not as a per-node override.
- Anything under `godot/assets/no-export/` is stripped from builds (`export_presets.cfg`).

## Step 6 — leave the reference behind

This is the half that is always skipped, and it is why 11 of 12 existing references are dead.
Node-ids read `3:869` in the Figma UI but `3-869` in a URL; record the URL form so it can be
pasted straight back.

```
Figma: <ComponentName> — https://www.figma.com/design/<fileKey>/<slug>?node-id=<node-id>
```

- `.tscn` → `editor_description` on the node it describes. `joypad.tscn` already does this, per
  node, and is the model to copy.
- `.gd` → a `##` docstring line near the top.
- One line per component; a variant gets its own line naming the variant property, e.g.
  `Property 1=Pressed`.
- If you only know part of it, **record what you have and say what is missing** — "file key
  unknown" is useful; a bare frame name pretending to be a reference is not.

## Step 7 — prove it matches, property by property

Building from the design is not the same as matching it, and "it looks right" is not evidence. The
last step of every design task is a table comparing **the numbers in Figma** against **the numbers
you measured in the running app** — one row per property, filled in from real readings, not from
what you intended to set.

Read the built values off the **live scene tree**, never off a screenshot. A screenshot cannot tell
you a colour: a 50%-black panel over a bright orange object reads as light grey, and a plate you
already removed looks like it is still there. That exact misreading nearly produced a "fix" for a
bug that did not exist. Use the **mobile-dev-debug-tool** skill:

```bash
scripts/unified.sh eval 'var n = <the node>
var sb = n.get_theme_stylebox("panel")
return {
  "height": n.size.y,
  "bg": str(sb.bg_color) if sb is StyleBoxFlat else "none",
  "text_color": str(n.get_node("%Label").get_theme_color("font_color")),
  "font_size": n.get_node("%Label").label_settings.font_size,
  "icon_size": str(n.get_node("%Icon").size),
  "gap": n.get_node("%Label").global_position.x - (n.get_node("%Icon").global_position.x + n.get_node("%Icon").size.x),
}'
```

Then write it out, and mark every row:

| Property | Figma | Measured | |
|---|---|---|---|
| Text colour | `#DFD0FF` | `(1, 1, 1, 1)` | ❌ miss |
| Icon size | 44×44 | `(44, 44)` | ✅ |
| Gap icon→text | 8 | 16 | ⚠️ deferred — pill restyle |
| Height | 60 | 52 | ⚠️ deferred — pill restyle |

Three outcomes only: **match**, **deliberate deferral** (named, with the reason, agreed with whoever
asked), or **miss** (fix it or report it). A row you cannot fill in is a row you have not checked —
"probably fine" is a miss.

Cover at minimum: every colour (text **and** icon **and** background — they drift independently),
font size, icon size, corner radius, height, padding, and the gap between elements. Colours are the
most common miss because they are invisible in a diff and plausible on screen.

**When you change one visual property, check its siblings.** Recolouring a keycap letter and leaving
the body text next to it white shipped a pill whose two halves matched neither the design nor each
other. Ask what else plays the same role in that component, and read those values too.

## Anti-patterns — do not do these

- **Don't guess a design you cannot open.** No plugin means ask for it, not improvise. A screen
  built from an imagined mock gets thrown away and misleads the reviewer about what was checked.
- **Don't record a frame name with no node-id and no file key.** That is exactly the state 11 of
  the 12 existing references are in, and none of them can be followed.
- **Don't bake the mock's text as a literal.** Every player-visible string is a key — see the
  **i18n** skill. `set_title`/`set_body` take a `TranslationKey` and a literal is a parse error.
- **Don't add another button variant** without reading `COMPONENT_AUDIT.md` first.
- **Don't eyeball colours.** Convert 0–255 to normalized floats exactly.
- **Don't verify from a screenshot.** Translucency over scene content, device scaling and
  compression all lie about colour and size. Measure the live tree (Step 7).
- **Don't call it done without the comparison table.** "Looks right" has shipped a white label
  beside a lavender icon, a 30px icon where the design said 44, and a grey plate nobody designed.
- **Don't hand-edit `.import` files.** Change the source asset and re-import.
- **Don't copy the `wearable_categories/` svg+png duplication.** It is legacy, not a pattern.
- **Don't pad with spaces or newlines to match a mock's spacing.** `text = "     METAMASK"`
  shipped once; a translated label re-centres and the padding stops lining up. Use container
  separation or `custom_minimum_size`.

## Verification checklist

```sh
cargo run -- import-assets                       # a new asset gets its uid
cargo run -- check-gdscript
gdformat godot/ && gdlint godot/
python3 tools/i18n/extract_strings.py --check    # every new label is keyed
```

- [ ] Plugin was actually used — the design was read, not inferred.
- [ ] Component placed by the tier decision tree, and `COMPONENT_AUDIT.md` checked for an
      existing atom.
- [ ] Colours converted exactly; no new palette abstraction introduced.
- [ ] **Step 7 comparison table filled in from measured values**, every row marked match /
      deferred / miss, and no row left blank.
- [ ] New asset imported before being referenced; `base_scale` chosen deliberately.
- [ ] Every string is a key in all three catalogues.
- [ ] A `Figma:` reference with file key and node-id left on the component.
- [ ] Looked at on a device in **both orientations** — CI renders nothing, and designs land wrong
      at the safe-area edges most of all. Use the **mobile-dev-debug-tool** skill to inspect the
      live tree.

## Reference

- `godot/src/ui/COMPONENT_AUDIT.md` — component catalogue, tier migration map, known duplication
- `godot/src/ui/layouts/figma_margins.gd` — design-px → device-px margin scaling
- `godot/src/ui/layouts/safe_margin_container.gd`, `layouts/hud_margins.tres` — safe-area insets
- `godot/assets/themes/theme.tres`, `dcl_theme.tres` — icons, colours, fonts, type variations
- `godot/assets/themes/*.tres` — the ~90 StyleBox siblings
- `godot/src/ui/components/organisms/joypad/joypad.tscn` — the only complete `Figma:` references
