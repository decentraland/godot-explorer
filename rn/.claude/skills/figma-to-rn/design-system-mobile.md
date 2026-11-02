# Design System Mobile (Figma reference)

File: `Jis8xv58qaDNy53OCGLeoh` ("Design System Mobile"). Pages: Colors, Buttons, Fonts, Icons, Tags,
Names, Inputs, Loader, Safe areas. This is the source every screen's components (buttons, colors,
text fields) ultimately trace back to — check here **before** re-deriving a spec from a specific
screen's node, and before assuming a value is screen-specific when it might be a shared token.

`get_variable_defs` does not work headlessly for this file (requires an active desktop-app
selection) — use `get_metadata` instead; color names/hex are readable directly from node *names*
(e.g. `"Ruby #FF2D55"`), no need to call `get_design_context` on every swatch.

## Colors — cross-checked against `rn/packages/ui-native/src/tokens.ts`

Confirmed exact matches already in `tokens.ts`: `ruby #FF2D55`, `orange #FF7439`, `shadow #161518`,
`pearl #ECEBED`, `snow #FCFCFC`, `gray #CFCDD4`, `silver #A09BA8`, `purple350/450/500/550`, and the
entire `rarityColors` table (common/uncommon/rare/epic/legendary/exotic/mythic/unique, both dark and
light variants) — all verified exact.

**Found drift**: `colors.periwinkle` in `tokens.ts` is `#9D8CFF`, but Figma's actual "Periwinkle"
swatch is `#A0ABFF` — the *correct* value is already used ad hoc in `notificationTitle.social` and
was hand-typed (not token-referenced) for the entry-flow disclaimer's link color. Don't silently
"fix" the shared `periwinkle` token without checking its other call sites first (icon colors, HUD
elements may intentionally differ) — flag it, confirm impact, then fix in one pass.

Not yet reconciled: `colors.lavender` (`#E8B9FF`) doesn't match any swatch found so far (Figma's
"Lavender" in the Secondary section is `#C640CD`, closer to `colors.magenta`) — needs a closer look
if a screen ever needs it.

Full purple scale (50→550): `#F3ECF9 #DCCBEC #C5AADF #AE8AD2 #966AC5 #7F48B7 #691FA9 #57178B #460F6F
#350854 #24033B`. Full vivid-pink scale (50→550): `#F8BDC8 #FDA0AF #FF8194 #FF5F77 #FF2D55 #DF2447
#C01C39 #A1132C #840B20 #680415 #4D000B`.

## Typography scale (Fonts page)

The scale's own label states the line-height convention directly: **"Font size / Inter: P1.2 o
1.25"** — i.e. Figma's own spec confirms the ≈1.2× `leading-normal` heuristic from §2 of the main
skill file, not just something inferred from entry-flow examples.

| Name | Size | Weight |
|---|---|---|
| H1 | 48p | Bold |
| H2 | 42p | Semi |
| H3 | 32p | Bold |
| H4 | 28p | Semi |
| H5 | 30p | Medium |
| H6 / Subheaders | 24p | Bold |
| Botones (Big) | 30p | Bold |
| Botones (Medium) | 24p | Bold |
| Body 1 | 26p | Regular |
| Body 2 | 26p | Semi |
| Body 3 | 24p | Semi |
| Body 4 | 20p | Regular |
| Body 5 | 20p | Semi |
| Caption 1 | 24p | Regular |
| Caption 2 | 22p | Regular |
| Caption 3 | 18p | Regular |
| Tags | 18p | Semi |

Cross-check against `rn/packages/ui-native/src/AppText.tsx` variants (sizes in Figma-px, i.e. `u()` output × 2):
- `title` (`u21`→42p) = **H2**.
- `label`/`caption` (`u12`→24p) = **Caption 1**, but Caption 1 is `Regular` weight while
  `caption`'s own definition uses `fonts.medium` — **mismatch found and fixed** on the Sign In
  divider ("or"): Figma's "or" text is explicitly `Inter:Regular` (confirmed via
  `get_design_context`), so `dividerLabel` needed an explicit `fontFamily: 'Inter-Regular'`
  override rather than inheriting `caption`'s medium weight. Any other screen using `variant="caption"`
  for body-adjacent copy (not an uppercase label) should get the same check.
- `button` (`u15`→30p) = **Botones (Big)**, Bold — matches the "Btn" named style found repeatedly
  on entry-flow screens (`lineHeight: 100`, `letterSpacing: 0`).
- `body` (`u15`→30p) does **not** match any of Body 1–5 (26/26/24/20/20p) — the entry-flow's 30p
  "Legal"/disclaimer text is a one-off larger size for this specific flow, not the general app's
  body scale. Don't assume `variant="body"`'s default always matches a screen's "Body" Figma label;
  check the actual `text-[Npx]` on the node.
- `heading` (`u28`→56p) and the entry-flow's own `flowTitle` (`u36`→72p) don't match **any** row in
  this table — both are oversized hero/display text specific to full-bleed screens, not part of the
  general in-app type scale. Expected, not a bug.

## Buttons page — component spec strings (read directly off the page, no need to re-derive)

`Button/Primary` (large) text label literally states: **"Height 96px Width Responsive Corner 16px
Font 30pt Bold Uppercase Icon 32x32"** — i.e. `minHeight: u(48)`, `borderRadius: u(8)`
(= `radii.button`, already correct), `fontSize: u(15)` Bold, **uppercase is a component-level style
property**, not baked into translated strings (see the main skill's §2b — Account Home's
`FLOW_ACCOUNT_GUEST: 'PLAY AS GUEST'` hardcoded-caps pattern should eventually be replaced with a
`textTransform: 'uppercase'` style + natural-case translation string, matching how Sign In's
provider buttons were fixed).

`Button/Entry` (the Google/Apple/Wallet/Mail provider buttons used on Sign In) is a **separate**
component family from `Button/Primary` — don't assume its icon size (confirmed 36×36 via the
specific screen instances, not 32×32 like generic `Button/Primary` with an icon) carries over to
other icon-bearing buttons; check the specific component family's own symbols.

States present for every button family: `Default` / `Pressed` / `Disabled` (and `Google`/`Apple`/
`Wallet`/`Mail` as `Type` for `Button/Entry`) — `rn/packages/ui-native/src/PrimaryButton.tsx`'s `disabled`/`pressed`
styles already cover the first two; pressed/disabled *colors* per type haven't been cross-checked
against these symbols yet.

Width model: the library's base components are "Hug contents" (width = text + 24px/side padding);
screens stretch them to "Fill container" explicitly (confirmed via `w-full` on the actual screen
instances) — matches how `rn/src/flow/screens.tsx` already lays these out (full-width Pressable in
a block container), no change needed there.

### Button/Secondary — has an explicit Figma-authored usage note

Node `2:767`'s Figma description (visible via `get_design_context`, written by the design team in
Spanish) literally says: *"Botón secundario sobre overlay negro 40%. Outline=True agrega borde de
2px. React Native: `<Button variant="secondary" | "outline" size="lg|md|sm" leadingIcon={...} />`"*
— i.e. there are exactly two variants, both `bg-[rgba(0,0,0,0.4)]` (`colors.black40`),
`rounded-[16px]` (`radii.button`): **`Outline=False`** has no border, **`Outline=True`** adds
`border-2` `#FCFCFC` (`colors.snow`). Both are `Btn/Large` (Bold, 30px, uppercase, `lineHeight:100`
literal, `letterSpacing:0`).

Cross-checked against `rn/packages/ui-native/src/PrimaryButton.tsx`'s `ButtonTone` union:
- `outline` tone already matches `Outline=True` (border color `colors.snow` ✓) — the one deviation,
  `rgba(0,0,0,0.6)` instead of the library's default `0.4`, is a **confirmed screen-specific
  override** (verified exact against Account Home's and Sign In's own screen nodes earlier in this
  survey), not a bug — leave it.
- `secondary` tone was wrong on both axes (`colors.white10` + `1px` `colors.white30` border matches
  no real Figma variant) — **fixed** to `{ backgroundColor: colors.black40 }`, matching
  `Outline=False`. Only two call sites (`src/discover/MessageState.tsx`,
  `src/backpack/ConnectionLost.tsx`) — fixed without a full visual pass on those two screens since
  the new value traces exactly to the documented spec; worth a quick look next time either screen
  is touched.
- `dark` tone (`{ backgroundColor: colors.black40 }`, used e.g. by Avatar Create's "EDIT" button)
  was *already* a correct match for `Outline=False` before this fix — `secondary` and `dark` are
  now visually identical. Not merged/deduplicated (that's a naming decision, not a color-accuracy
  one) — flagging so a future pass doesn't rediscover this as "two tones that do the same thing"
  without context.

### Open finding: `PrimaryButton` may be missing `textTransform: 'uppercase'` app-wide

Every button family's named text style (`Btn`/`Btn/Large`) is `Bold` + `uppercase` at the
**component** level in Figma — not something each caller's translation string should bake in. But
`rn/packages/ui-native/src/PrimaryButton.tsx`'s label (`<AppText variant="button">`) has no `textTransform` at all,
and nothing in `AppText`'s shared `button` variant sets one either (can't add it there — that
variant is reused by unrelated card/name labels elsewhere that must stay natural case, see the main
skill's §2b). The result is **inconsistent casing across the app depending on whether a given
translation string happens to be hardcoded uppercase**: confirmed two near-identical "try again"
strings, `BACKPACK_TRY_AGAIN: 'Try again'` vs `COMMON_TRY_AGAIN: 'TRY AGAIN'`, both rendered through
the same `PrimaryButton`. `PrimaryButton` is used broadly (`Dialog.tsx`, `EmptyState.tsx`,
`MessageState.tsx`, `ConnectionLost.tsx`, every entry-flow screen) — **not fixed**, since adding
`textTransform: 'uppercase'` to `PrimaryButton`'s own label would be a visible behavior change
across every screen that uses it, most of which haven't been visually re-verified in this pass.
Surface this to the user/team before touching it; if approved, the fix is a single `textTransform`
line on `PrimaryButton`'s label plus reverting any translation strings that hardcoded caps as a
workaround (`FLOW_ACCOUNT_GUEST`, `COMMON_TRY_AGAIN`, etc.) back to natural case.

## Inputs page — Text Field component (applied to `rn/src/flow/screens.tsx`'s `TextStep`)

Real spec, confirmed via `get_design_context` on `Default`/`Focus`/`Error` state instances of the
single-line `Input-Field`:

| State | Style |
|---|---|
| Default | `bg-[rgba(0,0,0,0.4)]` (= `colors.black40`), `rounded-[12px]` (= `u(6)`), `p-[16px]` (= `u(8)`), no border |
| Focus | + `border` `1px` `#E8B9FF` — this is literally `DCL/Rarities/Legendary Light`, i.e. **the same hex as `colors.lavender`**, just reused as a focus-ring color. Resolves the earlier "unreconciled" `lavender` token — it's correct, just semantically double-purposed. |
| Error | + `border` `1px` `#FF2D55` (`colors.ruby`) |
| Text | `Inter:Medium` `30px` (= `u(15)`) = the **H5** type-scale row, `lineHeight: 100` literal, matches `fontFamily: 'Inter-Medium'` already used |
| Placeholder | `rgba(252,252,252,0.5)` — added as `colors.white50` (didn't exist before) |

Was previously wrong on every axis: `radii.pill` (should be `u(6)`, not a pill), `colors.white10`
background (should be `colors.black40`), `fontSize: u(16)` (should be `u(15)`), a permanent visible
border regardless of state (should be transparent/no border by default, colored border only on
focus/error). Fixed: `input`/`inputFocused`/`inputError` styles in `screens.tsx`, wired through
`TextInput`'s `onFocus`/`onBlur` plus the existing `error` prop. A multiline/long variant also
exists (`h-[216px]`, counter text `0/300` in `Inter:Regular` `20px` `#ECEBED`) — not yet needed by
any current screen, but same `black40`/`rounded-12` base applies if one shows up.

## Icons page — categories and base sizes (not individually imported)

Four component families, each its own base size — **don't assume one family's icon size carries
over to another** (already bit us once: `Button/Entry`'s provider icons are 36×36, not the generic
32×32 quoted in `Button/Primary`'s spec string):

| Family | Base size | Covers |
|---|---|---|
| `Icon/NavBar` | 82×82 (`u41`) | bottom tab bar: Explore/Backpack/Social/Settings/Alerts/Profile/Chat, each Active/Default |
| `Icon/Menu` | 22×22 (`u11`) | context-menu glyphs (Block/Report/Kick/Ban/Copy/Translate/…, ~35 icons) |
| `Icon/Base` | 54×54 (`u27`) | general UI icons (Back/Close/Check/Chevron/Search/Filter/Email/Wallet/Chat/Emote/…, ~40, each `Filled=False/True`). Home in code: `fillPaths` in `rn/packages/ui-native/src/iconSvgs.ts` (`bell`, `check` so far); `Icon.tsx`'s `baseStrokes`/`strokePaths` are hand-drawn stand-ins for this family, replace with the real asset whenever a screen touches one (SKILL.md §2d) |
| `Logo/Connect` | 64×64 (`u32`) | brand marks: Discord, X, Apple, Google, WalletConnect — the *master* asset size; the actual Sign In screen instance scales these down to 36×36, which is the number that matters for implementation (see the main skill's §2b/Buttons section), not this 64×64 library default |

Don't bulk-import every icon as an SVG asset — pull one on demand when a screen actually needs it,
same as every other asset-handling rule in this skill. But when you do pull one, it is the Figma
asset itself, never a redrawn approximation (SKILL.md §2d).

## Tags / Names pages — Discover/in-world UI, not entry-flow

`Tags` page: `Tag/Category` (Art/Social/Game/Shop/Education/Music/Fashion/Sports/Crypto/Casino/
PointOfInterest/Business), `Tag/EventTime` (Live/StartingSoon/InMinutes/InHours/InDays/Date),
`Tag/Event` (Trending/Featured) — all Discover-feed event chips. `Names` page: `Nametag/Claimed` +
`Nametag/Unclaimed` (Default/Speaking states), `Icon/NameVerified`, device-type glyphs — in-world
avatar nametags. Neither is relevant to the entry-flow screens this skill was built against; survey
properly (state colors, exact radii) only when a Discover or in-world nametag task actually needs
it.

## Loader page — empty

No content on this page as of this survey. Nothing to extract.

## Safe areas page — reference only, not actionable in RN

Documents per-platform/orientation margins (Android vs iOS, portrait vs landscape — status bar,
home indicator, notch/Dynamic-Island heights as literal px callouts like `102` labeled "Safe area"
for iOS). This is designer-facing reference for how much margin to leave in mockups — in RN,
`react-native-safe-area-context`'s `useSafeAreaInsets()` (already used by `FlowFrame`) resolves the
real per-device inset natively at runtime, so there's no fixed number here to copy into code.
