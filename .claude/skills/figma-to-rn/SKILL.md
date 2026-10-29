---
name: figma-to-rn
description: Use when building or fixing a React Native screen or component in `rn/src` from a Figma design, or when a screen "doesn't look like the Figma". Encodes the Figma→RN workflow for this repo - unit conversion (px/2 via `u()`), mapping Figma output onto `rn/src/ds` tokens and components instead of inline values, auto-layout→flexbox, hero/background assets, safe areas, and the side-by-side screenshot check that must pass before the work is done. Trigger on a figma.com URL, "implement/port/match this design", "differs from Figma", entry flow, FTUE, modal or screen polish.
---

# Figma → React Native

Companion to the `figma:figma-design-to-code` skill (load it before `get_design_context`). This one adds what is specific to `rn/`.

## Why screens drift (read once)

The entry flow shipped looking nothing like `Mobile-Entry-Flow` because it was ported from the old Godot lobby, not from Figma, and values were typed by hand. Typical drift:

1. **Wrong source of truth.** Never port from the Godot/GDScript screen or from main's behavior for *visuals*. Figma is the visual source; the old UI is only a behavior reference.
2. **Frames are old versions.** Boards (e.g. the `HIFI` page) hold many copies of a screen name (`ACCOUNT_HOME` ×6, `DCL_SPLASH` ×8). Pick the latest by asking the user, or by the highest node id / lowest-risk frame, and confirm it with a screenshot. Do not mix elements from several versions.
3. **Missing assets.** Hero photos, gradients with blend modes, provider logos and icons are not "decoration": without them the screen reads as a different design.
4. **Approximated components.** A Figma "Buttons" instance has its own radius, height, weight and case. Using `PrimaryButton` without checking those makes every button off.
5. **Copy and structure.** Labels, order of actions and legal line come from Figma, not from existing i18n keys. If the Figma text differs from the i18n catalogue, change the key (see the `i18n` skill), do not keep the old string.

## Units

Figma frames are **720×1600 px = 360×800 dp at 2x**. In code, write the Figma px **divided by 2** inside `u()` (`rn/src/ds/scale.ts`), which scales by `min(width,height)/360`.

| Figma px | code |
|---|---|
| 96 high button | `u(48)` |
| 30 px label | `u(15)` |
| 72 px title | `u(36)` |
| 64 px side padding | `u(32)` |
| 16 px radius | `u(8)` |

Never put raw Figma px in `u()`, and never use unscaled numbers for layout sizes. Hairlines (1–2px) may stay unscaled.

## Workflow

1. `create_session` with the fileKey, then `get_design_context` (`skillNames: "figma-design-to-code"`, `clientFrameworks: "react-native"`) for **one screen frame at a time**. The whole board is far too large for `get_metadata`; use the node id of a single frame. Use `get_variable_defs` for colors and text styles.
2. Download the screenshot (`get_screenshot`) next to the generated code. It is the target.
3. **Map before writing.** For every color, font, radius and component in the Figma output, find the existing one:
   - Colors → `rn/src/ds/tokens.ts` (`colors.ruby`, `snow`, `pearl`…). Figma variable `DCL/Ruby` = `#FF2D55` = `colors.ruby`. If a color has no token, add a token, never an inline hex.
   - Text → `AppText` variants. Figma `Btn: Inter Bold 30 uppercase` has no matching variant unless you add one. Check weight (`Bold` ≠ `semibold`), size, line height and case.
   - Buttons/inputs/pills → `rn/src/ds/*`. Extend the component with a prop when Figma has a variant it lacks. Do not restyle at the call site.
   - Icons → `ds/Icon` and `ds/iconSvgs.ts`. Add missing SVGs there.
4. **Auto-layout → flexbox.** Figma `flex-col justify-between pt-220 pb-165 px-64` maps to a `View` with `flex: 1`, `justifyContent: 'space-between'`, `paddingTop: u(110)`… Keep the *rules* (padding, gap, alignment, fill vs hug), not absolute positions. Do not use fixed spacers (`flex: 3` / `flex: 2`) to imitate a position.
5. **Safe areas.** Figma frames are 1600 high including the status bar. Do not add `insets.top` on top of a header or padding that already reserves it; reserve once, from the frame's own numbers.
6. **Backgrounds.** Full-bleed images use `resizeMode="cover"` and are anchored the way the frame crops them (the Figma frame is wider than the screen on purpose). Gradients and `mix-blend-multiply` overlays: reproduce with a `LinearFill`/overlay, and say so if a blend mode has no RN equivalent.
7. **Assets.** Download every asset from `get_design_context` into `rn/src/assets/` (they expire in 7 days). Never ship Figma URLs.
8. **Compare.** Register the screen in the gallery (`rn/src/flow/gallery.tsx` is the pattern), capture it at 720×1600 with `tools/ui-spec/ci_capture.sh --ids <dir> <id>` (Storybook + Maestro, see "Visual snapshots" in `rn/README.md`), and view it next to the Figma screenshot. List every difference (position, size, weight, color, copy, asset). Fix all of them. Do not declare done on "close enough". CI repeats the capture on every PR (`rn_visual.yml`) and sends it to the mobile board: approve the changed screens there once the new look is the intended one.
9. Check a taller (e.g. 412×915) and a shorter (e.g. 360×640) viewport: content must stay inside the safe area and the layout must follow the Figma rules, not break.

## Hard rules

- No inline hex colors, no magic font sizes: tokens and `AppText` variants.
- No new component for one screen when a `ds/` one can take a prop (`REVIEW.md` §9: non-abstracted repo).
- Behavior stays as it is (flow machine, auth calls). A design port changes presentation unless the user asks otherwise; if Figma shows a different flow (e.g. one provider list vs. create/sign-in split), call it out and ask.
- Any visible string goes through i18n (`i18n` skill).
- Comments: two lines max, only for what the code cannot say. No "was previously…" notes.

## Done checklist

- [ ] Figma frame id and version named in the PR (so the next person can re-check).
- [ ] Every color/text/radius traces to a token or ds component.
- [ ] Every Figma asset is present, local, and at the right place.
- [ ] Side-by-side screenshot reviewed; differences listed and fixed or explicitly accepted.
- [ ] Tested on a tall and a short viewport.
