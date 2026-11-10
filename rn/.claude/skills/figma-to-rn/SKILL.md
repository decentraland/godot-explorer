---
name: figma-to-rn
description: Use whenever porting a Figma design (a screen, a component, a single tweak) into `rn/src/`. Runs the full fetch → convert → build → pixel-verify → iterate loop with minimal back-and-forth — don't wait to be told to check fontSize, lineHeight, letterSpacing, colors, radii or spacing; check all of them, verify against real screenshots, and only report once the numbers converge.
---

# Figma → React Native (Decentraland entry flow and beyond)

This is the general playbook for turning a Figma node into `rn/src/` code at the precision level
the team expects — not "looks about right," but numbers that match Figma within a pixel or two,
proven with a real screenshot comparison, not eyeballed. Built out over the `feat/entry-flow-parity`
session porting Account Home / Sign In; apply it to any future Figma → RN work in this repo.

**The standing expectation: don't wait to be walked through each check.** Given a Figma node and a
target screen/component, run the whole loop below unprompted — fetch, convert, build, screenshot,
measure, fix, re-measure — and only hand control back once position/size/color/spacing all verify
within ~1-2px. Report a compact table of what was checked and the before/after numbers, not a
running narration of each step.

**For a whole flow (several screens, states, a drawn order) read §8 first and follow it to the letter:
the user does not review against Figma, this process does.**

**This skill is young and is expected to keep improving — that's the point, not a gap to apologize
for.** Several rounds of back-and-forth on one screen are fine; what matters is that each round
teaches the skill something so the *next* screen needs fewer rounds. Every constant here (the
~0.73 cap-to-baseline ratio, the ~1.2× leading-normal multiplier, the specific gap values in the
worked reference) was fit from a small number of examples on one Figma file — treat them as a
starting estimate, re-verify with `get_metadata` per node, and **when a new pattern, pitfall, or
better technique turns up, add it here before moving on**, the same way `figma-to-godot-ui` absorbs
a lesson from every PR that uses it. Don't just fix the one screen and let the insight evaporate —
a technique that only lives in one session's transcript doesn't compound.

## 0. Setup

- **Check `design-system-mobile.md` (next to this file) before re-deriving a color, type-scale, or
  button/input spec from a screen's own node.** It catalogs the "Design System Mobile" Figma file
  (`Jis8xv58qaDNy53OCGLeoh`), fully surveyed page by page: colors cross-checked against
  `rn/packages/ui-native/src/tokens.ts`, the full type scale (H1–H6, Body 1–5, Captions, Botones), `Button/Primary`/
  `Button/Entry` specs, the Text Field component (Default/Focus/Error), and icon family base sizes
  (NavBar/Menu/Base/Logo-Connect). Tags/Names (Discover + in-world nametags) and Safe areas
  (resolved natively by `react-native-safe-area-context`, nothing to port) are noted but not deeply
  surveyed since nothing in this flow needs them yet. Extend that file (not this one) whenever a
  screen surfaces a new finding against it.
- **Confirm which page/node is canonical before trusting a first fetch.** This org's Figma files
  commonly carry more than one version of the same screen at different fidelity — an "ATLAS" page
  with redline/spec annotations, and a "HIFI"/"HIFI-WIP" page with the fully-composed final design
  (same pattern `figma-to-godot-ui` documents for other files). Two nodes that look identical by
  name and screenshot can still differ by a few px in real resolved position — found this on Account
  Home, where a HIFI-linked node (`1738:9473`) had the bottom block's `paddingBottom` ~4px off from
  the ATLAS node (`1905:14886`) used for the first pass. If the user links a *specific* node partway
  through, treat that as the priority source and re-verify against it with §5 rather than assuming
  whatever was already fetched is close enough — don't guess which page is "more right," ask or let
  a linked node settle it.
- Figma MCP: `create_session` once per file, reuse the `sessionId` for every subsequent call.
- **Fetch the whole screen frame once (`get_design_context` with `forceCode: true`), not only the
  block you are fixing.** The frame-level export is where the container rules live (the `Content`
  frame's `pt/pb/px`, `justify-between`, each group's `gap`) and where text colour, weight and copy
  are stated per span. Account Home's second review found nothing wrong in the containers and three
  misses in exactly that layer: the tagline is `white` (the `body` variant is pearl), the legal
  paragraph is snow with **Regular** periwinkle links (the code had Medium links and an inline hex),
  and the copy was "Decentraland's …, Privacy Policy and Content Policy." where Figma says
  "Decentraland’s …, Privacy Policy, and Content Policy." (curly apostrophe, Oxford comma — the
  wrap differed because of it). Copy goes into i18n verbatim from the frame, punctuation included.
- `get_metadata` on a node for its resolved tree (ids, names, `x/y/width/height` — **absolute
  ground truth for position and size**, unlike CSS classes which can be ambiguous).
- `get_design_context` on a node for the CSS-ish spec (colors, font, radius, border) — **convert
  it, never paste it**. It targets React+Tailwind; this project is RN + a `u()` scale + the
  existing design system in `rn/packages/ui-native/src/`.
- `get_screenshot` on a node for the reference image, downloaded via `curl -L` (the URL expires in
  7 days — never keep it as a link, fetch and save the PNG immediately).
- Python + Pillow for the verification scripts below. System Python usually lacks Pillow in this
  environment — make a throwaway venv once per session:
  ```bash
  python3 -m venv .venv && .venv/bin/pip install --quiet Pillow
  ```

## 1. The scale convention — get this right first, it invalidates everything else if wrong

`rn/packages/ui-native/src/scale.ts`'s `u(x)` expects **Figma px ÷ 2** — confirmed by the file's own doc comment.
`SCALE = min(min(w,h)/360, max(w,h)/800)` — the Figma frame (360×800dp) is *contained* in the
phone: by width on most phones, by height on one that is proportionally shorter than the frame
(Galaxy S25, 415×900dp: 1.125 by height, not 1.154 by width). `u(size) = size * SCALE`. A Figma
`gap-[46px]` is `u(23)`, a `text-[72px]` is `u(36)`. Never pass a raw Figma number into `u()`,
never eyeball-map to "whichever existing token looks about right."

Why contain and not width: the reviewer lays the Figma frame next to the device **aligned by
height**, and every vertical number is judged that way. With width scaling, on the S25 every
padding, gap and line box came out 2.5% taller than the frame's proportions, which read as "the
top block sits too low and the bottom block too high" even though each value was the exact Figma
px. Scaling by height for margins only (`v()`) fixed the edges and left the stacked buttons ~6dp
off; the DS-wide contain scale closed everything to ≤1.5dp. On a 360×800 viewport (the web
capture) both definitions are 1.0, so CI snapshots do not move.

## 2. Per text node — fontSize, lineHeight, letterSpacing

**fontSize**: `text-[Npx]` in the Tailwind export → `u(N / 2)`. Trust it directly, it's literal.

**lineHeight — the single most common mistake.** `get_design_context` shows `leading-[normal]` for
most text in this file. That is **not** a literal value — it means "use the font's own default
metric," which for Inter is ≈1.2× the font size, **not** 1.0×. Reading `leading-[normal]` as
"`lineHeight == fontSize`" is wrong and produces visibly tighter text than Figma. Two cases:

- **A node references a named Figma text style** (the design-context response appends a line like
  `These styles are contained in the design: ... Btn: Font(family: "Inter", style: Bold, size: 30,
  weight: 700, lineHeight: 100, letterSpacing: 0)`). That `lineHeight: 100` is **literal and
  authoritative** — 100% means equal to font size, exactly. Use it as-is, no back-calculation
  needed. This is how the entry-flow's button labels confirmed `lineHeight == fontSize` really is
  correct there (unlike the title/tagline/disclaimer, which have no such named style and default
  to `leading-normal`).
- **No named style, just `leading-[normal]`**: back-calculate the real line-height from
  `get_metadata`'s resolved height, because that's the only place the literal number lives:
  1. Get the node's resolved `height` and figure out how many lines it wraps to (check the
     screenshot, or character-count-vs-width estimate).
  2. If the node has **no** `text-box-trim` class: `height = lines × lineHeight` exactly (clean
     division — a 2-line title at 174px height is 87px/line, no remainder).
  3. If the node **has** `[text-box-trim:trim-both] [text-box-edge:cap_alphabetic]`: the trim only
     removes the leading above the first line and below the last line, **not** the gap between
     wrapped lines internally. `height = (lines-1) × lineHeight + capToBaseline`, where
     `capToBaseline ≈ 0.7-0.73 × fontSize` for Inter (empirically fit from this file's own nodes —
     re-derive per node with two known data points when precision matters, don't hardcode blindly).
  4. Convert the solved `lineHeight` through `u(figma_px / 2)` same as fontSize.
  5. As a ballpark sanity check only (never a substitute for step 1-4): Inter's `leading-normal` is
     close to 1.2× the font size in this file. Two confirmed examples: a 72px title → 87px real
     line-height (87/72=1.21); a 30px legal/disclaimer text → 36px real line-height (36/30=1.2).

**letterSpacing**: check for a `tracking-[Npx]` class, or a named style's `letterSpacing` field in
the "styles contained in the design" footer. If absent, it's 0 — same as RN's own default, nothing
to set. Every node checked in this file so far (title, body, buttons, legal/disclaimer) is
`letterSpacing: 0` — don't assume it needs a value unless you actually see one.

## 2b. "Uppercase" is a style, not translated text

Figma buttons often carry a `uppercase` CSS class on the label while the underlying string is
natural case (`"Google"`, the span renders `GOOGLE` only because of the class). **Never bake the
transform into the translation catalog** (`FLOW_X: 'PLAY AS GUEST'`) — add `textTransform:
'uppercase'` to the style instead, scoped to the specific label(s) that need it, not to a shared
`AppText` variant used elsewhere in the app (e.g. `variant="button"` is reused by dozens of
unrelated card titles/names across the app that must stay natural case — adding the transform there
would be a silent regression everywhere else). Found on Sign In: the provider button labels
(`Google`/`Wallet Connect`/`Apple`/`Continue with email`) were missing the transform entirely and
rendered in sentence case against Figma's uppercase spec, while Account Home's `PLAY AS GUEST`/
`SIGN IN` happened to read correctly only because an earlier pass had hardcoded the caps into the
catalog string itself (a latent anti-pattern worth revisiting, not copying forward).

## 2c. A "gradient" background may actually be a baked image, not a CSS gradient

Some Figma backgrounds export as an SVG with `<filter>` Gaussian-blur elements (glowing orbs behind
a solid base color) — `react-native-svg` doesn't render those filter primitives, so recreating it
from the raw SVG/path data isn't practical. Don't guess a 2-stop `LinearFill` approximation from the
visible colors either — it reads visibly flatter than the real thing (checked on Sign In: a flat
magenta→purple gradient was clearly less saturated/directional than Figma's actual blurred-orb
background). Instead, `get_screenshot` the *specific background sub-node* (not the whole screen) at
a resolution matching its native Figma size, download it as a PNG, and use it as an `ImageBackground`
— same technique as this file's existing photo backgrounds. This gets the exact rendered colors
including the blur, at the cost of one more bundled asset.

**A baked background that only exists composed inside screen frames** (the avatar stage: brand
solid + `Background/Avatar`, whose glows use `mix-blend-multiply`/`plus-lighter` and SVG blur
filters) cannot be exported from Figma by MCP: `get_screenshot` of the instance flattens it over
the canvas grey (`#1E1E1E` corners), the DS master has no brand base, and every screen frame has
the avatar on top. Rebuild it instead: take the `get_design_context` markup of the background
layers verbatim (Tailwind classes, inline gradient data URIs, the blurred SVG assets), put it in
an HTML file with the Tailwind play CDN, screenshot it with the repo's `playwright-core` through
Chrome (`chromium.launch({ channel: 'chrome' })`, viewport 720×1600) and compare pixel colours
with the frame screenshot at avatar-free spots. Two traps: the generated code keeps the
component's native box (1372×1475) inside the scaled instance box (1604×1724), so wrap it in a CSS
`scale()`; and a `get_metadata` `y` outside the frame (1811 for a 1724-tall instance in a 1600
frame) means the instance is flipped — add `scale(s, -s)`. The stage matched within 2–5 RGB units
everywhere; the ATLAS page's "Background" export is an older, flatter design, not the HIFI one.

## 2d. Icons and other assets — use the design's asset, found in the design system first

Never redraw a glyph the design already provides, even when a look-alike is one path away. For
every icon or image a screen shows: (1) `get_metadata` on the Icons page of the design-system file
(`10:158`) and find the component (`Icon/Base`, `Icon/Menu`, `Icon/NavBar`, `Icon/Settings`,
`Icon/Modal`, `Logo/Connect`); (2) confirm the screen's instance really is that component (the
`get_design_context` of the consuming node names it, and its SVG `d` matches the master's byte for
byte); (3) download the master's SVG with `curl -L` and put the path data in `rn/packages/ui-native/src/iconSvgs.ts`
— `fillPaths` for `Icon/Base` (54×54 monochrome fills, colour via `fill`), `sources` for multi-colour
exports. An icon the design-system file does not have is still never drawn by hand: download it
from the screen's own node in the design file it came from, add it the same way, and flag it as
outside the design system — the designer decides whether it gets promoted into the library or stays
a one-off that isn't worth adding. Found on `Button/Reminder`: `bell`/`check` had been hand-drawn 24×24 strokes while `Icon/Base` had the real
`Type=Notification`/`Type=Check` all along; the rest of `baseStrokes`/`strokePaths` in `Icon.tsx` is
the same debt, to be swapped family by family as screens touch them.

## 3. Colors, fills, borders, radii

| Figma export | RN |
|---|---|
| `bg-[#RRGGBB]` | `backgroundColor: '#RRGGBB'` — check `rn/packages/ui-native/src/tokens.ts` first, this repo keeps hex values as named tokens (`colors.ruby`, `colors.snow`, etc.); use the token, don't duplicate the literal |
| `bg-[rgba(r,g,b,a)]` | copy verbatim as the RN rgba string |
| `border-N border-[#hex] border-solid` | `borderWidth: u(N)` (border widths are **not** halved like other dimensions — Figma's own export already gives the literal stroke width, e.g. `border-2` is a literal 2px stroke, not `2 * 2`), `borderColor: '#hex'` |
| `rounded-[Npx]` | `borderRadius: u(N / 2)` — check `rn/packages/ui-native/src/tokens.ts`'s `radii` first (`radii.button = u(8)` already *is* 16px, don't re-derive and hardcode a second copy) |

Confirm every color/radius pairing against the actual token file, not by re-typing the hex — a
button whose fill, border, and radius all trace back to existing tokens (`colors.ruby`,
`colors.snow`, `radii.button`) is both correct and consistent with the rest of the app.

## 4. Layout — spacing, gaps, padding

**Prefer `get_metadata`'s resolved positions over a Tailwind `gap-[Npx]`/`pb-[Npx]` class when they
disagree.** A fixed-height footer (`Footer-Buttons` h 200, `pt 18`, `pb 40`) puts its button at
`y + 18` and leaves 86 below it, not the 40 the class says: measure from the child's `y` and
`height` to the frame edge.** A real contradiction surfaced this session: one screen's exported `pb-[350px]` didn't
match what the metadata's actual child positions implied. Compute gaps directly:
`gap = nextChild.y - (prevChild.y + prevChild.height)`, `padding = frame.height -
(lastChild.y + lastChild.height)`. Metadata is ground truth; the CSS class is a convenience that
can drift from it.

**Fixed edge padding + adaptive middle gap, not a hardcoded middle value.** A layout with two
groups pinned to the top and bottom of a frame (an "ACCOUNT_HOME"-style screen: logo/title/tagline
at the top, buttons/legal at the bottom, lots of empty space between) is
`justifyContent: 'space-between'` with the *edge* paddings taken from Figma, not a fixed middle
`gap` value — a fixed gap computed at one device aspect ratio overflows or underflows at another.

**Watch for double-counted padding** when a shared wrapper (e.g. `FlowFrame`) and a per-screen
container both contribute vertical space. If the per-screen container already carries Figma-exact
margins, the shared wrapper should contribute *only* real safe-area inset, nothing more — a wrapper
that adds its own "safety buffer" on top of a screen that already has a correct margin quietly eats
the budget `space-between` needs and collapses everything toward the center.

**RN has no `text-box-trim`.** Every multi-line text node that uses it in Figma renders *taller* in
RN than in Figma, because RN keeps the font's natural leading above/below the block that Figma's
trim strips away. The fix is never to shrink that node's own `lineHeight` (which must stay accurate
to Figma's real per-line metric, see §2) — it's to absorb the excess in the **margin/gap
immediately before** the oversized block, found empirically via §5, not computed analytically (the
exact amount of "untrimmed leftover" isn't predictable from the Tailwind export alone).

**A component shared across two screens needs its anchor value literally identical, not just each
screen individually close to Figma.** With `justifyContent:'space-between'`, an element's distance
from its container's edge is set *only* by that container's `paddingTop`/`paddingBottom` — internal
gaps before it only affect spacing from the *previous* sibling, not its distance from the edge. If
the same component (e.g. a Terms disclaimer footer) appears at the bottom of two different screens
and should read as the *same persisted element* across a screen transition (no jump, no visual
reset), both screens' trailing padding must be the exact same `u()` value — even if each screen's
own Figma frame gives a slightly different nominal number (here: Account Home's Figma source was
`161px`/`u80.5`, Sign In's was `163px`/`u81.5` — close enough that the designer clearly intended
parity, so pick one value and use it on both, then re-verify each screen's own landmarks via §5
since changing a shared padding shifts everything anchored above it too). Don't tune two screens'
bottom paddings independently to "look right" in isolation — check whether they're meant to match
whenever the same component crosses screens.

## 5. Verification — pixel-measure, don't eyeball

Build the screen, then prove the numbers with real screenshots. Two tiers:

**Coarse pass (overall rhythm / sanity)**: normalize both images to a common height and overlay a
gridline every 5%, to make sure landmarks (hat, button block, disclaimer) land in roughly the right
place:
```python
from PIL import Image, ImageDraw
target_h = 1600
fig_r = fig.resize((int(fig.width*target_h/fig.height), target_h))
impl_r = impl.resize((int(impl.width*target_h/impl.height), target_h))
# paste side by side, draw horizontal lines every 5% of target_h, label with the %
```
This is good enough to catch a layout that's badly wrong, but **gridline-eyeballing at this
resolution has ~10-15% measurement error** — not good enough to settle a "is this gap 2-3px off"
question. Don't try to read individual-pixel differences off a gridline screenshot; it produces
inconsistent readings between successive attempts.

**Precise pass (does this specific gap/height match, within a couple px)**: pixel-color
edge-detection along a scanline through a solid-fill landmark (a button, a solid block) — accurate
to ~1px and the method that actually converged every residual gap this session:
```python
from PIL import Image

def find_edges(path, scan_x_frac=0.5, y_search=(0.55, 0.85), is_target=lambda r,g,b: False):
    im = Image.open(path).convert('RGB')
    w, h = im.size
    x = int(w * scan_x_frac)
    y0, y1 = int(h*y_search[0]), int(h*y_search[1])
    px = im.load()
    ys = [y for y in range(y0, y1) if is_target(*px[x, y])]
    return (min(ys), max(ys), h) if ys else None

# example: PLAY AS GUEST is a solid ruby (#FF2D55) fill
is_ruby = lambda r,g,b: r > 200 and g < 120 and 40 < b < 140
top, bot, h = find_edges('impl.png', is_target=is_ruby)
frac_top = top / h
fig_top_px = frac_top * 1600   # convert to Figma's 1600-tall frame for direct comparison
```
Run this on both the Figma reference screenshot and the device screenshot, report both numbers and
the delta, apply **one** targeted fix, rebuild, re-measure. Do not guess a magnitude and move on —
confirm convergence with the same script before calling a gap "fixed."

**Text landmarks: compare line centroids, not band edges.** `scripts/text_rows.sh` (next to this
file, ImageMagick only, no Pillow) masks the light text colours, projects the image to one column
and prints every row band with its **centroid**. Band *edges* move by ±8px between a crisp device
render and a blurry 461px-wide Figma export, and thin ascenders/descenders drop in and out of the
mask; the centroid of a line stays put to ~1px, and the centroid-to-centroid distance between two
lines is the line pitch (`lineHeight`). The method that converged Account Home in one pass:
```bash
# Figma shot scaled to the DEVICE height, device shot as captured: one shared y axis
scripts/text_rows.sh figma.png  --height 2340 --crop 853x2340+100+0
scripts/text_rows.sh device.png                --crop 880x2340+100+0
```
Crop ~100px off each side so the photo background's highlights do not join the mask. For a solid
landmark (a ruby button) `magick img -crop <region> +repage -fuzz 8% -fill black +opaque '#FF2D55'
-fill white -opaque '#FF2D55' -trim -format '%wx%h+%X+%Y' info:` gives its box. Do the reference
scaling **by height** (the reviewer's alignment); scaling by width only agrees when the phone has
the frame's aspect ratio.

**Measuring glyph-only landmarks.** A crisp device render keeps descenders inside the mask while a
blurry Figma export drops them, so a text row's band is taller on the device and its centroid sits
2-3px lower for the same real position: compare the *band top* too, or use a row without
descenders. For an icon-only button (the naming dice), size the `Icon` from the glyph's extent
inside its viewBox against Figma's bounding box (44 of 54 → `size = box / 0.81`) and confirm with
the band height of the glyph on both images, not the button's box.

**Emulator at the Figma frame size (the project standard).** `npm run emulator:figma` pins the
emulator to 720×1600 at 320 dpi, a 360×800dp @2x display: `SCALE` is 1, a screenshot is the same
size as the frame PNG and every number compares 1:1 with no scaling or aspect-ratio argument
(`npm run emulator:reset` restores the phone). Verify there first, then on the real aspect ratio.

**Device loop (no rebuild).** The emulator carries the no-godot mock build (`[CoreSelfTest] no
native core`), hot-reloaded by Metro on 8081; a registered preview opens with
`adb shell am start -a android.intent.action.VIEW -d "dclrn://preview/<id>"` (the normal boot may
land elsewhere, e.g. a recovered mock session skips Account Home) and `adb exec-out screencap -p >
shot.png` captures it. Force-stop the app before reopening a preview whose code changed. The
react-native-web capture (`tools/ui-spec/ci_capture.sh … --ids <id>`) is the CI view of the same
screen; its `?preview=` needs the gallery host to mount `preview-ready`, which a careless merge once
dropped — if every capture times out, diff `rn/src/gallery/GalleryHost.tsx` against `main` first.
The whole flow runs on the mock too: `adb shell am start -n
org.decentraland.godotexplorer/com.godot.game.GodotAppLauncher` and `adb shell input tap x y`
through the steps (a recovered mock session boots into Avatar Create). Entry-flow previews
register as `extra/<name>` ids: `docs/rn-ui-spec/manifest.json` does not know
the Mobile-Entry-Flow file, and the gallery test rejects other prefixes.

## 6. Iteration discipline

- **One change per measured gap.** Don't bundle three guessed adjustments into one edit; each
  landmark gets its own measure → fix → re-measure cycle, otherwise a correction and an overshoot
  cancel out and you can't tell which fix actually worked.
- **Small deltas.** Move by a few px-equivalent at a time, not large jumps — an explicit user
  correction like "you went 2-3px over" or "you moved way too far" is itself a data point: it tells
  you the right step size for the *next* nudge, don't re-guess from scratch.
- **Trust a live visual report over a static analytical model.** If the user is watching a live
  reload and says a position looks off, treat that as ground truth even if your own screenshot
  measurement said it already converged — rebuild and re-measure, don't argue with the math.
- **Report compactly.** A table of node → Figma value → code value → verification result beats a
  blow-by-blow narration. Save the narration for when something doesn't converge and you need to
  explain why.

## 7. Don't re-derive what's already a shared component

Before building a "new" button/card/radius from scratch, check `rn/packages/ui-native/src/` — `Button`
already has `big` (96px / 16px radius, the entry-flow CTA) and the `outline` tone (black 40% + 2px
snow border; a frame that draws it at 60% is an instance override to flag, not a DS change), and
`Spinner` is Figma's `Loader/Spinner`; `AppText` variants carry the type scale (`heading` u28, `body` u15,
`button` u15, `caption` u12 — check these against a node's `text-[Npx]` before assuming a screen
needs a bespoke font size). Extend an existing DS component additively (a new tone/variant) rather
than hand-rolling a one-off style when the shape already exists elsewhere in the app.

## 8. Flow audit protocol — "the Figma, not the person watching, finds the differences"

The user's standing rule (2026-11-10): **no step of Figma → implementation may depend on them noticing.**
A flow is done only when every frame of it, in every state, was compared against the *current* Figma
and the differences are either fixed or listed as decisions. Everything below comes from mistakes made
on `feat/entry-flow-parity`; each one cost a round trip the user should never have paid.

### 8.1 Source of truth
- The **current Figma** is the spec. Not the existing code, not the first RN port (it was a vague
  approximation of the old Godot lobby), not another screen. Code that exists is *unaudited* until a
  measured comparison says otherwise: never write "implemented" about a screen you did not measure
  (an inventory that says "exists" was read by the user as "matches the design").
- Inherited screens get the same audit as new ones. Opening the app on a device and finding a screen that is
  not the Figma (Welcome Back was the old Godot one) is an audit failure, not a surprise.
- **Name code 1:1 with the document.** Step ids and screen components carry the frame names
  (`auth_otp_start` ↔ `AuthOtpStart` ↔ frame `AUTH_OTP_START`; states of one frame are props, not new
  steps). A reviewer must be able to check a screen against its frame by name alone.

### 8.2 Read the flow before the pixels
1. List the frames of the flow in drawing order (`scripts/figma-reads.md` §1): the page repeats the flow once per
   path (guest, Google, email, returning user), left to right, with `Line N` arrows and decision groups
   ("Correct code", "New user"). The drawn order **is** the flow; do not reuse an older flow from the code.
2. Read every `TEXT` note on the page near the frames: they are behaviour specs ("keyboard pushes the text field
   and button up", "show the numeric keyboard, allow native paste", "background no gradient", "resend timeout 2 min",
   "hide credits here"). Each one is a requirement with a test on a device.
3. Make a checklist: one line per frame **and per state** (empty, focused with keyboard, valid, invalid, loading,
   error, disabled, pressed) and per variant ("Loading..." vs "Opening browser..."). A frame that only differs by
   an OS screenshot of the keyboard is still a state to implement (focus ring, caret, lift).
4. Several copies of a frame exist. Diff them node by node (`figma-reads.md` §2) to find the live one; copies that
   are equal prove nothing about the code. Re-read after the designer "cleans up": ids disappear, frames move,
   sizes change (the splash logo went 145 → 142 → 130/140 between readings). Re-read at the start of each
   session and again before saying done.
5. Anything ambiguous or that looks like a design slip (a copy of another modal used as a frame, sample text
   like "Text / 0/15", a double space, a link missing its underline) is implemented as drawn **or** the nearest
   sensible reading, and goes in the report under **Decisions**. Never drop it silently, never ask the user to
   discover it.

### 8.3 Compare against the real renderer, every state
- Export the frame at full resolution (`download_assets`, scale 1, png: 720×1600). The 461-px screenshot is only for
  looking.
- Capture the implementation on **Android native** (emulator or phone, debug build + Metro, preview deep link
  `dclrn://preview/<gallery id>`; release builds have no gallery) and on web (`tools/ui-spec/ci_capture.sh`,
  what CI snapshots). Web alone hid 4–5 px native offsets and the device-only behaviours (keyboard, line breaking).
  When web and native disagree, find the cause (usually a text without an explicit `lineHeight`) instead of tuning
  for one of them.
- Measure with `scripts/measure.py` (bounding boxes and deltas per region) and sample colours with
  `magick img -format '%[pixel:p{x,y}]' info:`. Target ≤ 2 px and exact colours. Expected non-bugs: the status bar
  pushes safe-area-aware elements (back arrow) ~10 px down on a device; text that Figma fills with sample words.
- Drive the **real flow** on the device too, not only previews: a state only reachable by interaction
  (keyboard lift, blur validation, the screen after a back, a session the app remembers) is exactly where
  previews lie.
- Per-range text styling matters: `getRange*` for colour, weight and decoration of each run. One frame underlines links,
  its sibling does not.

### 8.4 Per-screen checklist (do all of it, unprompted)
Text: size, weight, line height, colour, case, decoration per run, line breaks · fills, strokes (weight and colour
per state), radii, shadows · every icon/logo at its drawn box, from the design's SVG (a PNG with padding looked 22 %
smaller) · spacing and positions · background kind (glow / plain / photo / image: the notes say) · safe areas ·
keyboard behaviour · copy in en/es/pt_BR · gallery entry per state, with the Figma node id in `notes` · tall and short viewport.

### 8.5 Report shape (what the user reads)
Per screen: frame id → what was compared (states) → measured deltas → fixes → **Decisions** (ambiguities, slips, behaviours
chosen). Lead with what does not match. Do not write "coincide" without numbers behind it.

### 8.6 Pitfalls found (each cost time)
- Release APKs have no gallery/deep link; use `npx react-native run-android --main-activity com.godot.game.GodotAppLauncher
  --extra-params -PdclNoGodot=true` + Metro + `adb reverse tcp:8081 tcp:8081` (uninstall a differently signed build first).
- A `--no-godot` Android build that crashes on `libdclgodot.so`: delete `rn/android/build/generated/autolinking`.
- `download_assets` rejects ids inside instances (`I123:4;5:6`): use the component's `mainComponent.id`.
- Android's line breaker treats `a@b.com.` as an address and may drop the full stop alone on the next line; detect
  with `onTextLayout` and move the address down instead (the frame wraps them together).
- `AppText` does not forward arbitrary props (`onTextLayout`): use `Text` when you need them.
- `prettier --write <dir>` reformats files you did not touch; `git add <dir>` takes stray untracked dirs along. Stage files
  by name and look at `git diff --cached --stat`.
- Renaming files/ids: do it with a script restricted to the flow files (`'account'` is also a settings section).
- A frame's text that differs from i18n changes the key (all three languages), not the call site.

## Worked reference (Account Home, HIFI frame `1738:9473` in `H1Alvbe7sh0e0IyLckewMI`)

Final values after the second review (2026-11-03, contain scale, measured on the Galaxy S25 with
`scripts/text_rows.sh` against the frame scaled to the device height — every landmark within
1.5dp). **The frame's box math was right all along**: every "corrected by eye" value in the earlier
table below was compensating for width scaling on a phone that is shorter than the frame, plus
band-edge measurements that drifted. Take the frame's numbers, scale them with `u()`, and verify
with centroids before touching anything.

| Node | Figma | RN (`u()`) |
|---|---|---|
| Content `pt` / `pb` / `px` | 220 / 165 / 64 | `paddingTop: u(110)`, `paddingBottom: u(82.5)`, `FlowFrame` `u(32)` |
| Logo, Top gap | 110, 36 | `u(55)`, title `marginTop: u(18)` |
| Title | Inter Bold 72, 2 lines = 174 → 87/line, snow | `heading`, `fontSize: u(36)`, `lineHeight: u(43.5)` |
| Text gap, tagline | 20; Inter Regular 40, 96 → 48/line, **white**, width 534 | `marginTop: u(10)`, `u(20)`/`u(24)`, `color: colors.white`, `maxWidth: u(267)` |
| Bottom gap, Buttons gap | 48, 48 | `accountActions.gap: u(24)`, `accountButtons.gap: u(24)` |
| Buttons | 96 high, radius 16, px 24, Bold 30 uppercase | `<Button big fullWidth>` (`controlHeight.lg`, `radius.xl`) |
| Legal | Inter Regular 30 snow, 3 lines = 36/line, links Regular periwinkle underlined | `body` + `lineHeight: u(18)`, `color: colors.snow`; links `colors.link` |

The earlier table, kept as the record of what went wrong:

| Node | Figma | RN (`u()`) | How it was found |
|---|---|---|---|
| Title fontSize | 72px | `u(36)` | literal `text-[72px]` |
| Title lineHeight | 87px (2 lines, no trim: `174÷2`) | `u(43.5)` | `get_metadata` height ÷ lines |
| Tagline fontSize | 40px | `u(20)` | literal |
| Tagline lineHeight | 48px (trimmed: `77=1×L+29.3`) | `u(24)` | back-calculated, trim formula |
| Legal fontSize | 30px | `u(15)` | literal, matches `AppText variant="body"` default |
| Legal lineHeight | 36px (trimmed: `94=2×L+22`) | `u(18)` | back-calculated, trim formula |
| Button label lineHeight | 100% (named "Btn" style) | no override needed | literal from named-style metadata, not leading-normal |
| Button fill/border/radius | `#FF2D55` / `rgba(0,0,0,.6)`+2px `#FCFCFC` / 16px | `colors.ruby` / `outline` tone / `radii.button` | existing DS tokens, confirmed exact |
| Button height | 93-96px | `minHeight: u(46)` | pixel-edge-detection vs metadata's nominal 96 |
| Gap title→tagline | ~40px visual (not the naive `194-174=20` box math) | `marginTop: u(4)` | pixel-edge-detection (baseline-to-cap-top), not box subtraction |
| Gap buttons→disclaimer | 47px nominal, but RN's untrimmed 3-line disclaimer runs ~7u taller than Figma's trimmed version | `accountActions.gap: u(18.5)` | nominal value as a start, corrected via pixel-edge-detection until PLAY AS GUEST's measured top matched Figma's within <1px |
| Screen `paddingBottom` (anchors the shared disclaimer) | `u(84)` box-math on the HIFI node, but the *visible* disclaimer text sits ~7px higher than that box edge once rendered (RN's untrimmed line-boxes leave extra leading below the last line that Figma's trim doesn't have) | `accountContainer.paddingBottom: u(77)`, **must equal** `signInScroll.paddingBottom` on Sign In too (see §4's shared-anchor note) | started from HIFI's box-math value, then corrected by eye against the live app (user: "disclaimer's slightly too high, lower it ~7px") |

The gap/padding rows are the important lesson: **the nominal Figma box-math number is a starting
point, not the final answer**, whenever RN can't replicate a Figma-only CSS feature (`text-box-trim`,
`leading-normal`) exactly — and that gap, between the *box edge* (which `get_metadata` describes
exactly) and the *visible glyphs* (which depend on how the platform's text renderer distributes
leading within an oversized line-box), isn't something the Figma export can tell you up front. Close
every number with §5's pixel measurement, and when the landmark is text rather than a scannable
solid color, trust a direct visual report over the box math.

**Decoupling two landmarks anchored to the same bottom padding.** When a `justifyContent:
'space-between'` block has a fixed-position element (e.g. a button, already verified correct) above
a looser one (e.g. text whose visible glyphs don't line up with its box edge), moving the *lower*
element without disturbing the *upper* one means changing two values together in opposite
directions by the same amount: decrease `paddingBottom` by `X` (slides the whole block's bottom
edge — and the lower element with it — down by `X`) and increase the gap immediately above the
lower element by the same `X` (grows the block's total height by `X`, which exactly cancels the
`paddingBottom` decrease for everything *above* that gap, keeping the upper element's position
fixed). Verify with §5 that the upper landmark's measured position is unchanged before trusting the
lower one moved correctly.
