# Design System Mobile (Figma reference)

File: `Jis8xv58qaDNy53OCGLeoh` ("Design System Mobile"). Pages: Colors, Buttons, Icons, Fonts, Tags,
Names, Inputs, Dropdowns, Controls, Modals, Tabs, Headers, NavBar, Menu, Cards_Backpack, Cards_Discover,
Loader, Skeleton, EmptyState, Backgrounds, Spacing & Sizes, Elevation, Motion, States, Safe areas. This is the source every screen's components (buttons, colors,
text fields) ultimately trace back to — check here **before** re-deriving a spec from a specific
screen's node, and before assuming a value is screen-specific when it might be a shared token.
Last full pass (every component set read through `get_design_context` metadata, no screenshots):
**2026-11-10, second round** (Sheet `_Glow` fixed at 125dp, the EmptyState page, `Icon/Base` Edit / Add / Copy,
`Header` Back Landscape, and two sets missed on 11-09: `RarityLabel` on Tags and `Feature` on Cards_Discover) after
**2026-11-10** (`Panel` frame on the Modals page = `Sheet`, `Card-Profile`, Dialog buttons both Fill) after
**2026-11-09** (the designer's changes since 11-06: Ruby Pressed and four rarity Light colours, the
alpha tokens, the Cards_Backpack / Cards_Discover / Skeleton pages, `Header` Close, the `Background`
component as gradients, the Shop and Chat icon families, `Menu/Item` text style). New nodes show up as ids above the previous pass's
highest (`get_metadata` per page, grep the ids); value edits inside old nodes only show by re-reading
the set. **Conflict rule (decided 2026-11-03): the drawn component wins.** When a node's
values contradict its usage description or a spec string, implement the node — the design gets
its approval visually, a description is text the designer can forget to update. Descriptions are
still the source for the intended RN API and for behaviour the nodes cannot show. Re-run it page by page when the designer announces changes; the node ids below are
the component *sets*, so one call per set returns every variant.

`get_variable_defs` does not work headlessly for this file (requires an active desktop-app
selection). Read colours with `get_design_context` on the swatch group: its "styles contained in the
design" list carries the variable values. **Do not trust the swatch label text or node names**: on
2026-11-09 the variables changed while the labels on the Colors page still showed the old hex.

## Colors — cross-checked against `rn/packages/ui-native/src/tokens.ts`

Confirmed exact matches already in `tokens.ts`: `ruby #FF2D55`, `orange #FF7439`, `shadow #161518`,
`pearl #ECEBED`, `snow #FCFCFC`, `gray #CFCDD4`, `silver #A09BA8`, `purple350/450/500/550`, and the
entire `rarityColors` table (common/uncommon/rare/epic/legendary/exotic/mythic/unique, both dark and
light variants) — all verified exact.

Fixed 2026-11-03 to the Colors page swatches: `periwinkle` `#A0ABFF` (was `#9D8CFF`; also exposed as
`colors.link` = Figma `text/link`), `teal` `#22C1C1` (was `#2FCF9A`; Toast icon + Trending card),
`rarityColors.exotic.dark` `#9CD71E` (was `#CAFF73`). Name drift kept on purpose (renaming touches
dozens of call sites): code `lavender` = Figma **Lavender Light** `#E8B9FF` (`action/selected`, focus
rings, selected pills), code `magenta` = Figma **Lavender** `#C640CD`, code `white30`/`white50` =
Figma **Snow 30%/50%** (`rgba(252,252,252,…)`) while `white20`/`white70` are true White alphas.
Added: `rubyDisabled` (Ruby 50%), `ruby200` `#FF5F77` (counter at its limit), `black10`, `black70`,
`white`, `white40`, `snow10`, `snow70`. Still untokenised (no consumer): Almost White `#E5E5E5`,
Yellow `#FFC95B`, Melon `#FFA25A`, Violet `#A524B3`, Cloudy Sky `#716B7C`, Grass `#28AC00`, Red
`#E00000` (only as `menuPaths.delete.fill`), Mint `#34C070`, Light Ruby `#FF859C`, Ruby Peach
`#FFE5EA`, and the **System** scales (Success 100–500 `#C1EECF #65D890 #34CE77 #00B453 #00A146`,
Error `#FFCDD4 #E97177 #FB3B3B #EC303A #CC1D2C` — 300 is `colors.danger`, Warning `#FFCD88 #FFB95B
#FE9C2A #F38025 #EC7122`). The palette bar gradient starts at `#FFBC5B`, not Yellow.

Changed by the designer on 2026-11-09 (aligned with the Desktop/Web libraries): `rubyPressed`
(`action/primary-pressed`, every `Button/Primary` Pressed) `#E1002B` → **`#D80029`**; rarity Light
`uncommon` `#FFD6B2` → `#FFE0B2`, `epic` `#81D1FF` → `#81E1FF`, `legendary` `#DF9CFF` → `#E8B9FF`,
`exotic` `#E4FFB8` → `#DBF0AD`. `notificationTitle.event` keeps `#DF9CFF` (it comes from the
notifications design, not the rarity token). New alpha tokens: `white8`/`white4`, `black8`/`black4`
(Skeleton), `black50` (Avatar-Card pressed), `deepPurple40`/`deepPurple50` (Purple 500 `#350854`,
Discover card without image), `lavender50`; also `shadow80` (Equipped bar), `brand` `#470071`
(`background/brand`) and `lightRainbow` (`Gradients/Light Rainbow`, shared by `EventBadge` Featured
and the selected `AvatarCard`). `space/26` exists only as a variable used by `Menu` (26 = `u(13)`).

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
| Btn/Large (Botones Big) | 30p | Bold, **uppercase** |
| Btn/Medium | 24p | Bold, uppercase |
| Btn/Small | 22p | Bold, uppercase |
| Body 1 | 26p | Regular |
| Body 2 | 26p | Semi |
| Body 3 | 24p | Semi |
| Body 4 | 20p | Regular |
| Body 5 | 20p | Semi |
| Caption 1 | 24p | Regular |
| Caption 2 | 22p | Regular |
| Caption 3 | 18p | Regular |
| Tags | 18p | Semi, uppercase |
| Input (= H5) | 30p | Medium |
| Input/Value | 30p | Regular |
| Input/Suffix | 24p | **Light** (no `Inter-Light` face is bundled; `UsernameField`'s `#1234` suffix stays Regular until it is) |
| Input/Code | 48p | Semi |
| Link | 24p | Medium, underline |
| Tab/Label | 28p | Semi, uppercase |
| Header/Credits | 32p | Medium |

Body 1/2 have lineHeight 34, B4 (the row labelled "Body 3") 30; everything else is 100%. The style
names `B3`/`B4` are swapped relative to the page labels "Body 3"/"Body 4" — go by the values.

Cross-check against `rn/packages/ui-native/src/AppText.tsx` variants (sizes in Figma-px, i.e. `u()` output × 2):
- `title` (`u21`→42p) = **H2**.
- `label`/`caption` (`u12`→24p) = **Caption 1**, but Caption 1 is `Regular` weight while
  `caption`'s own definition uses `fonts.medium` — **mismatch found and fixed** on the Sign In
  divider ("or"): Figma's "or" text is explicitly `Inter:Regular` (confirmed via
  `get_design_context`), so `dividerLabel` needed an explicit `fontFamily: 'Inter-Regular'`
  override rather than inheriting `caption`'s medium weight. Any other screen using `variant="caption"`
  for body-adjacent copy (not an uppercase label) should get the same check.
- `button` (`u15`→30p) is SemiBold and natural case because card/name labels reuse it; the real
  **Btn/\*** style (Bold + uppercase, "Buttons - uppercase always" on the Fonts page) is applied by
  `Button`, `LoginButton` and `LinkButton` themselves (`styles.label`), not by the variant.
- `body` (`u15`→30p) does **not** match any of Body 1–5 (26/26/24/20/20p) — the entry-flow's 30p
  "Legal"/disclaimer text is a one-off larger size for this specific flow, not the general app's
  body scale. Don't assume `variant="body"`'s default always matches a screen's "Body" Figma label;
  check the actual `text-[Npx]` on the node.
- `heading` (`u28`→56p) and the entry-flow's own `flowTitle` (`u36`→72p) don't match **any** row in
  this table — both are oversized hero/display text specific to full-bleed screens, not part of the
  general in-app type scale. Expected, not a bug.

## Buttons page — `Button`, `PillButton`, `IconButton`, `LoginButton`, `LinkButton`

Audited 2026-11-03 against the sets `Button/Primary` 1:17, `Button/Secondary` 2:767, `Button/Entry`
244:492, `Button/Discover` 4:1222, `Button/Filter` 4:1554, `Button/Social` 300:383, `Button/Pill`
10:293, `Button/Reminder` 594:589, `Button/Link` 706:4763, "Width — Hug vs Fill" 505:343. The page's
spec strings ("Height 96px Width Responsive Corner 16px Font 30pt Bold Uppercase Icon 32x32" …) are
older than the nodes: where they disagree the nodes win (Entry says "Icon 32x32", the logos are 36).

| Component | Figma | Code |
|---|---|---|
| Button sizes | Large **fixed 96**, Medium 74, Small 54; px 24 (`space/lg`); gap 8 (`space/xs`) on every variant | `minHeight u(48)`/`u(37)`/`u(27)`, `paddingHorizontal u(12)`, `gap u(4)` |
| Button radius | Large 16 (`radius/xl`); Medium 12 (`radius/lg`); Small 8 (`radius/sm`) | `radii.button` / `medium` / `small` |
| Button icons | leading 32 / 28 / 24 (`icon/button-lg|md|sm`), `Icn/JumpIn` trailing at the same sizes; Reminder icon 54 | `iconSize` 16/14/12 dp, reminder 27 |
| Button label | `Btn/Large|Medium|Small` Inter **Bold** 30/24/22, lh 100%, **uppercase**; Snow, Disabled `text/disabled` Silver 50% | `styles.label` (bold + uppercase) over `AppText variant="button"` |
| Primary | Ruby / Pressed `#E1002B` / Disabled Ruby 50% | `ruby` / `rubyPressed` / `rubyDisabled` |
| Secondary · Outline | Black 40% / Pressed Black 60% / Disabled Black 30%; Outline adds a 2px Snow border, Silver 50% when disabled | `secondary` + `outline`; `dark` is a code-only alias of `secondary` |
| Primary `Text=False` 316:282 | 54×54 Ruby square, radius 8, `Icn/JumpIn` 24 | `IconButton variant="primary"` |
| Reminder 594:589 | control/lg 96, px 24, py 16, gap 8, radius 12; Default purple 40% + **1px Snow border**; Pressed purple 60% (same border, Snow text); Selected Black 20% + 2px Ruby border + Ruby text/check | `tone="reminder"` (`selected` swaps border and colours) |
| Entry 244:492 | 96 tall, radius 16, px 24; Google/Apple/Wallet Snow bg + Shadow text, logo 36 (gap 4, Wallet 10), Pressed White 70%, Disabled White 10% + content at 60%; Mail = Secondary with `Icon/Base` 38 + gap 8 | `LoginButton` |
| Discover 4:1222 | 94×96 radius 16, `Icon/Base` 52; purple 40% / Pressed **60%** / Selected 70% + Lavender Light icon / Disabled 20% + Snow 30% icon | `IconButton variant="filled" big` |
| Filter 4:1554 | Rebuilt 2026-11-06 (781:\*): **54×54 radius 8**, `Icon/Base` 40 (the new `Menu-Dot`); `State` Default / **Tap-Selected** (one state for press and selection: Black 60% + Lavender Light icon) / Disable (Black 30% + Silver 50% icon) × `Background` On/Off (Off = the glyph alone, same colours); no badge state any more | `IconButton variant="filled"` (`selected`, `background={false}`); the `badge` prop is gone |
| Social 300:383 | 54×54 radius 8, 2px border, `Icon/Base` 40; Black 40% / Pressed 60% with the border **and icon** in Ruby (Cancel) or Rare Light (Accept) / Disabled Black 30% + Silver 50% | `IconButton variant="outline" accentColor` |
| Pill 10:293 | Large 60 / Medium 56, pl 14 pr 16 py 8 gap 4, icon 32; Large Default Black 40% + Pearl B2 26/34; Medium Default White 20% + White 70% B4 24/30; **Selected = Lavender Light + Shadow in every size** (States page); badge 22 at −4/−4 | `PillButton` |
| Link 706:4763 | 54 tall, hug, no padding; `Link` Inter Medium 24 underline, `text/link` `#A0ABFF`; Pressed label at 70%; Disabled Silver 50% | `LinkButton` |
| Width | Hug by default (text + 24/side); "Fill container" = `fullWidth` | `alignSelf: 'stretch'` |

Designer conflicts, re-read 2026-11-03 after the designer's pass: `Button/Reminder` Pressed is now
purple 60% + Snow border + Snow text (code `glassPressed`) and the Medium radius is 12 on Primary and
Secondary alike — both resolved. Still open: `Button/Pill` Large `Icon=False` nodes carry B4 24/30
while the `Icon=True` nodes say 26/34 — node vs node, code keeps 26/34 until the designer settles it;
`Button/Entry`'s spec string says "Icon 32x32" but the logos are 36 and the Mail icon 38 (code: 36/38).
Code-only: `tone="ghost"` (no Figma source, generic 50% dim), `loading` (renders `Spinner`),
`LoginButton fullWidth`. `Button/Filter`'s description still says "60×60, State=Active shows the
notification dot" — the rebuilt set has neither, the nodes win. `Button/Social` Disabled keeps a Snow
glyph (same asset as Default); only the border goes Silver 50%. The reference page `extra/ds-buttons` (`rn/src/gallery/dsButtons.tsx`) draws
this page with the real components; Pressed is seen by pressing (no `pressed` prop, by decision).

## Inputs page — `TextField`, `SearchField`, `CodeSlots` (`rn/packages/ui-native/src/`)

Audited 2026-11-03 against `Input/Field` 562:4820, `Input/Labeled` 562:4890, `Input/UniqueName`
562:4908, `Input/Counter` 562:5035, `Input/CodeSlots` 562:5058, `Input/Search` 562:5083 and
`Input/Label` 562:6682 (on the Dropdowns page). Every error colour on this page is
`status/danger` `#FB3B3B` (`colors.danger`), never Ruby.

| Part | Figma | Code |
|---|---|---|
| Field box | 624 fill × 68 (single line), **p 16 all sides**, gap 10, radius 12, Black 40%; text `Input` Medium 30 Snow; placeholder `text/placeholder` Snow 50% | `minHeight u(34)`, `paddingHorizontal u(8)`, `gap u(5)`, `u(6)`, `black40`, `white50` |
| Focus | 1px Lavender Light border; `Icon/Base` close 40 Snow at the right (Focus/SingleLine grows to 72 because the icon overflows the padding — Focus/Name pins 68; code keeps 68) | `focused`, clear `size={20}` |
| Error | 1px `#FB3B3B`; message under the box Regular 20 `#FB3B3B` | `error`, `hint` + `errorText` |
| Disabled | Black 30% bg (`surface/overlayDisabled`), text and placeholder White 20%, label White 30% — a palette state, not a dim | `boxDisabled`, `textDisabled`, `labelDisabled` |
| Multiline | fixed 216 tall, `items-start`, p 16; counter `B3` Regular 20 Pearl 8 below ("0/300"), `#FB3B3B` at the limit in the Error node | the **box** is `minHeight u(108)` (216 includes p 16; until 11-09 the TextInput got the 108 and the box grew to 126), the input stretches, `counter` (Regular `u(10)` Pearl, `marginTop u(4)`), `counterMax` = `Input/Counter`'s `ruby[200]` `#FF5F77` |
| Name | 68 box + a Regular 20 Pearl row 8 below: helper left, "0/15" right; suffix `#1234` `Input/Suffix` Light 24 in the text colour | `UsernameField` keeps the suffix Regular (no Inter Light face) and the helper above — screen-level follow-up |
| Labeled | `Input` Medium 30 **Snow** label, gap 20 to the field; Disabled label White 30%; `Button=True` adds a `Button/Secondary` Outline Medium (radius 12) after the field, gap 16 | `label` Medium Snow, `marginBottom u(10)`; no `button` prop yet |
| Label/Description | label + `Caption2` Regular 22 Pearl, gap 10, pb 8 | `helperAbove` `u(11)` Pearl, `marginBottom u(5)`, `paddingBottom u(4)` |
| Search 562:5083 | 540×68, **px 10**, radius 12, Black 40%; `Icon/Base` search **56** Snow, gap 8; text Medium 30 Snow, placeholder Snow 50%; Typing/Focus = 1px Lavender Light border + `Icon/Base` close 40 Snow | `SearchField`: one look for every call site (the old `glass` pill and the always-bordered `framed` box are gone; `framed` only widens the gap after the back chevron) |
| CodeSlots 562:5058 | 84×130 per digit, radius 16, Black 40%, 1px Shadow border; Typing = Lavender Light border + Black 70% + a 1px White caret 50 tall; Filled = Pearl border + `Input/Code` Semi Bold 48 Snow; Error = `#FB3B3B` border and digit | `CodeSlots` (hidden `TextInput`, `oneTimeCode`); the gap between slots is not specified in Figma — `u(8)` |
| UniqueName 562:4908 | Dropdown field with `Icon/NameVerified` 32 after the name; Blocked = Gray text, no badge, no chevron; Focus = 1px Lavender Light border; Disabled Black 30% (`surface/overlayDisabled`) + White 20% | `UsernameField` → `Dropdown` with a hand-drawn `ClaimedSeal` — swap for the Figma asset and add Blocked when that screen is next touched |
| Counter 562:5035 | `B3` Regular 20 Pearl, "0/300"; Max = `ruby[200]` `#FF5F77` | `counterLabel` + `counter`/`counterMax` |
| Label/ExternalLink | label + `Icn/NewChat` 32, gap 16, h 40 | not built (no consumer) |

Designer conflicts, re-read 2026-11-03 after the designer's pass — resolved: the standalone
`Input/Counter` is now `B3` 20 like the one inside `Input/Field`; `Input/UniqueName` Disabled is
Black 30% like `Input/Field`; `Input/Search` Focus holds a value ("Search" in Snow), so the clear
button only ever appears with text (what code does). Still open: Error/SingleLine 562:4837 has no
message row although the description promises one (code shows it); `Input/Dropdown` Disabled stays
Black 20% while `Input/Field` is Black 30% (code follows each node); `Input/Labeled Button=True`
keeps the nested field at a fixed 624 instead of fill. Re-read 2026-11-06: the `Name` helper row was
rebuilt (703:\*) as **one** Regular 20 line — helper/message left, counter right (Error: "15/15" left
in `#FB3B3B`, message right) — code still stacks hint then counter (`UsernameField` follow-up).
The entry flow's own inputs (`rn/src/flow/screens.tsx` `TextStep`, pill / White 10%) predate this
component and are the entry-flow branch's job.

## Icons page — every family is imported (`iconSvgs.ts`)

Audited 2026-11-06: `Icon/Base` 48:3563 (54, Filled=False/True, 41 glyphs → `basePaths`; `Menu-Dot`
778:179 added on 11-06 = three 7px dots, code `menuDot`, the old hand-drawn `kebab` is now its alias),
`Icon/Menu` 42:3274 (22, 47 glyphs → `menuPaths` + 8 multi-path SVG strings; `Location` 732:4783 added
on 11-06 = `Icon/Base` Location at 25px centred in the 22 box, code `menuLocation` via a `viewBox`), `Icon/NavBar` 17:228
(82, 7 types × Active → `navBarPaths` + the profile SVGs), `Icon/Settings` 606:112 (32, 6 →
`settingsPaths`; Gameplay is hidden in Figma), `Icon/Modal` 562:7607 (90×68, 8 → `modal*Svg`),
`Logo/Connect` 17:386 (64, 5 → `logoPaths` + SVG strings), `Badge/Notification` 501:24 (22),
`Icon/Credits` 681:67 (40, multicolour → `creditsSvg`). `Icon/Base` **Edit 852:124, Add 852:131, Copy
852:137** (added 11-10) → `edit` / `add` / `copy` in `basePaths`, each vector placed with the node's inset;
`copy` used to alias `menuCopy` and now draws the `Icon/Base` glyph (Profile and Settings copy buttons).
Nothing on the page is missing from code;
12 hand-drawn strokes (`plus`, `arrowRight`, `minus`, `bulb`, `hand`, `externalLink`, `eye`,
`eyeOff`, `lock`, `star`, `flame`, `upArrow`), `chevronRight` and `profileRing` are code-only.
Sizes per family are the base sizes; the consuming component sets the drawn size (`Button` 32/28/24,
`IconButton` 52/40, `Header` 60, `Search` 56, `Dropdown` 40, `Tabs` 32, `NavBar` 82/64).

Re-export candidates (nodes edited after the export): `Icon/Base` Share/Like/Dislike (vectors
640:1659–1680), `Icon/NavBar` Social inactive mask 641:903 and the Profile ring 641:2726, `Icon/Modal`
Block (641:830–833). The NavBar description says Active uses "the nav gradient"; the exported active
glyphs are flat `#E8B9FF` and code paints them so.

Not imported (added 2026-11-09, no screen uses them yet): the **Shop** frame 785:253 (`shop/cart`,
`cart-filled`, `offer`, `offer-filled`, `delete`, `view` 40; `credits`, `credits-filled` 38×40;
`sale`, `sale-filled` 40; `land`, `names` 24 — names is "temporary home"; `credits-coin` 24 full
colour) and the **Chat** frame 785:348 (24: `emojis-off/on`, nine `emoji-category/*`,
`microphone-off/on`; 23×24: `call`, `end-call`, `call-blocked`; several are composite groups with
rotations, export each component as one SVG when a screen needs it).

Don't bulk-import assets outside these families; when a screen needs one, it is the Figma asset
itself, never a redrawn approximation (SKILL.md §2d).

## Controls page — `Switch`, `Slider`, `Checkbox` (`rn/packages/ui-native/src/`)

Audited 2026-11-02 against `Control/Switch` 562:9023, `Control/Slider` 562:9048, `Control/Checkbox`
594:126. Every state colour is a palette token; disabled is never an opacity dim on the whole control
(States page rule: background at a lower alpha + Silver 50% text).

| Component | Figma | Code |
|---|---|---|
| Switch Large | track 140×64, pad 4, knob 70×56, pill; Checked Ruby, off Black 40% | `size="lg"`: `u(70)`×`u(32)`, pad `u(2)`, knob `u(35)` × full height |
| Switch Small | 110×56, pad 3.5, knob 60 | `size="sm"`: `u(55)`×`u(28)`, knob `u(30)` |
| Switch disabled | on: whole control 50% + Gray knob at 90%; off: Black 20% track + Pale Black knob | `onDisabled` / `offDisabled` / `knob*Disabled` |
| Slider | track 624×40 Black 40%, `radius.sm` 8 → `u(4)` on track, fill **and** thumb (the exported track SVG is an 8px rounded rect, not a pill), Ruby fill, thumb 32×50 Snow + 1px Pearl border + `Elevation/Down`; disabled: fill Pale Black 80%, thumb Pale Black | `track`/`fill`/`thumb` + `fillDisabled`/`thumbDisabled`; label row is `Input/Label` Value (label Inter Medium 30, value Inter Regular 30, both Snow) |
| Checkbox | Large 48 / Small 30, Black 40% box, 1px Snow border, `rounded-[2px]`; Checked = Ruby border + Ruby fill inset 12.5% **in every size** (the Small checked *node* is Lavender Light and the node wins: `size="sm"` checks in Lavender Light) | `size="lg"` `u(24)` / `"sm"` `u(15)`, `padding: '12.5%'` for the fill, hitSlop to 44dp, label is the touch target too |

## Dropdowns page — `Dropdown` (one component covers `Input/Dropdown` + `DropdownLabeled` + `OptionsList` + `DropdownOption`)

| Part | Figma | Code |
|---|---|---|
| Field | 624×68, `rounded-[12px]`, px 16, gap 10, Black 40%, text Input (Medium 30) Snow, chevron `Icon/Base` 40 | h `u(34)`, radius `u(6)`, px `u(8)`, text `u(15)` medium, chevron `size={20}` |
| States | Focus/Open: 1px Lavender Light border · Disabled: **Black 30%** bg (`surface/overlayDisabled`, the `DropdownLabeled` Disabled instance since 11-06; the standalone `Input/Dropdown` node still says 20%), White 20% text, label White 30% · ReadOnly: no chevron, Gray text · Error: 1px `status/danger` #FB3B3B (the node has no message row; code draws one, States rule) | `fieldOpen` / `fieldDisabled` (`black30`) + `textDisabled` + `labelDisabled` / `readOnly` prop / `error` prop (`colors.danger`) |
| Label | `Input/Label`: Medium 30 Snow, gap 20 to the field | `u(15)` medium, `marginBottom: u(10)` |
| Options list | `surface/menu` #1B002D, 1px border (Down node says `alpha.violet70`, Up node says `border/menu` = Lavender Light 70% — the theme token wins), radius 12, padding 8, gap 8, `Elevation/Down` or `/Up` | `colors.menu`, `colors.lavender70`, `u(6)`/`u(4)`/`u(4)`, `elevation.down|up` |
| Scroll height | 388 = p 8×2 + 5 rows × 68 + 4 gaps × 8 | `listHeight` counts the gaps (5 rows + 4 × `u(4)`) |
| Action=Button / Action=Copy | a `Button/Primary` Medium ("CONNECT", since 11-06; was Secondary Outline) or an `Icon/Menu` copy stretched to the field height after the field, gap 16 | not built (no consumer) |
| Option | 68 tall, px 16, radius 12; Selected = Black 40% bg + Lavender Light text + `Icon/Base` Check 40 | `row`/`rowSelected`/`textSelected`, check `size={20}` |

The old `variant="settings"` (tighter paddings) is gone: the Figma field already is the tight one.
`Input/Label` Description (helper, Caption2 Regular 22 Pearl) renders under the field at `u(11)`.

## Modals page — `Dialog`

`Modal/Dialog` 562:7666 + `Modal/Scrim` 591:118. Portrait card 520 wide (pt 64, pb 60, px 56, content
gap 54), Landscape 720 (pt 54, pb 48, px 54, gap 48); `surface/modal` #4C147C (`colors.modal`) with
the two glows (`surface/modal-glow`, `RadialFill` with dp radii); radius 16 → `u(8)`; title
`Modal/Title` Bold 38 → `u(19)` Snow centred; body `Modal/Body` Regular 26 / lh 36 → `u(13)`/`u(18)`
Snow; text gap 16; CTAs gap 32 (40 landscape) = `Button/Secondary` + `Button/Primary`, both filling
(every button gets `flex: 1` + `fullWidth`). `Icon/Modal` (90×68) sits centred over the top edge
at `top: -34px`. Scrim: `surface/scrim` = Black 40% + a 12px background blur (`Scrim/Blur`); the description no
longer mentions a no-blur value (it used to say 60%). RN has no blur library, so code keeps the
Black 60% (`colors.black60`) fallback — a code-only decision to re-check with the designer.
Orientation comes from `useWindowDimensions`; the old `rounded`/`width` props are gone.
Re-read 2026-11-06: the glows are no longer blurred ellipses but **two radial gradients in the card
fill**, Mythic `#FF4BED`: top-right centred on the corner, rx 210 / ry 190, 60% → 0; bottom centred
20px below the bottom edge, rx 343 (Portrait) / 475 (Landscape) / ry 140, 62% → 0; both with the same
six-stop falloff (×1, .669, .399, .192, .055, 0) — `glow()`/`FALLOFF` in `Dialog.tsx`, radii passed as
`rxDp`/`ryDp`. `Link=True` = `B3` Regular 20 underline **`text/link` `#A0ABFF`** (`colors.link`; was
`#32CEFF`, the `sky` token is gone), two lines max, gap 10 under the body (the body and the link share a group: `bodyGroup` gap `u(5)`,
fixed 11-09 — it was the text gap plus a margin, 13dp) →
`Dialog link={{ label, onPress }}`. Node quirks: 562:7744 (Portrait, no icon) is 514 wide inside its
520 frame (keep 520); the Landscape Input variant 562:7727 (720×514, content gap 36) is not modelled
either, the field is `children`. Checkbox and Input blocks stay the caller's
`children` (`Checkbox size="sm"` 30×30 radius 2; `TextField`). The Input variants (562:7756 with
label 520×458, 562:7771 with icon 520×414; frames fixed by the designer on 2026-11-03) tighten the
content gap to 38 and the text gap to 24 (with label) / 32 — not modelled, the caller passes the
field as `children`. "Cancel" there is a standard `Button/Secondary` Black 40% and the confirm
`Button/Primary` starts Disabled.
**Buttons re-read 11-10**: every variant now has both CTAs `Fill` (`flex-1`, `Btn/Medium` 74 / radius
12), so `Dialog` gives each button an equal share — the old hug + fill made the row change with the
labels.
`Modal-Background` 671:4363 (Portrait 520×466 with the 458 card inset / Landscape 720×404) is the
same card with the same glows and no content — not built.

### Panel frame (Modals page) — `Sheet`, `SheetFooter`, `BottomSheet`, added 2026-11-10

`Sheet` 841:1057 (720 wide): body = Purple 500 with `_Glow` 858:200 — Purple 450 → 500 over a **fixed 250 px
(125dp)** at the top so it does not stretch with the height (was 450 → 500 at 34% until the second 11-10
round; code: `SheetGlow`, shared by `Sheet` and `BottomSheet`), top radii `radius/2xl` 24, px 48,
pb 30, gap 40, `Elevation/Up`; `Handle=On` pt 16 + `_Sheet/Handle` 841:1036 (200×8, Snow 30%,
`radius/xs`), `Handle=Off` pt 40; `Height` Hug (Content min 200) / Fixed (Content fills and scrolls) /
Collapsed (54 = the handle alone). `Sheet/Footer` 841:1039: 200 tall Purple 500, pt 18, px 48, a 96 button
row (the description says 20dp of bottom space; the node has 46 px left over — code: `minHeight u(100)`
and `max(u(20), bottomInset)`). Examples 841:1058: Profile, Backpack (Fixed), Backpack collapsed, Entry
flow (Handle=Off + footer: `Button/Secondary` hug + `Button/Primary` fill, Large, gap 18) and Discover
detail (over `Modal/Scrim`). Code: `Sheet` (static, `handle` / `height` / `footer`) and `SheetFooter`;
`BottomSheet` (draggable) draws the same body and handle itself and exports `SHEET_HANDLE_HEIGHT`
(`u(32)`); its old `background` / `handleWidth` / `handlePadding` props are gone. Backpack peek = 27dp
(Collapsed), the Discover detail scrim = Black 60% like `Dialog`.

## Tabs page — `Tabs`

`Tab/Underline` 618:130: 56 tall, pt 8, pb 16, px 4, gap 4; label `Tab/Label` Semi Bold 28 uppercase,
Snow when selected (+ 6px Ruby underline), Gray otherwise; `Notification` = 16px `Badge/Notification`
at top 5 / right -6; `Icon` = an `Icon/Settings` 32 glyph before the label. Code: `u(28)` tall,
underline `u(3)` absolute at the bottom, dot `u(8)`, `icon` 16dp. A number badge keeps the inline
count pill (no Figma source — a code-only extension).

## Headers page — `Header`

`Header` 624:57. Portrait 720×176: pt 40, pb 24 (30 with Back), px 48; title H1 Bold 48 → `u(24)`;
Back = `Icon/Base` 60 → `size={30}` + gap 8; Actions gap 32 → `u(16)`: Search 60, `Badge/Credits`
(h 60, px 18, gap 10, Black 40% pill, `Icon/Credits` 40 + Medium 32 amount), Ticket 54 (Back only).
SearchOpen: Back + `Input/Search` filling (gap 24, pb 20, **bottom-aligned**: the 60 icon against
the 68 field; code centred it until 11-06) — rendered with the DS `SearchField`, now the
`Input/Search` look (see Inputs).
**`Type=Close`** 817:3410 (added 11-09): only an `Icon/Base` Close 60 at the left, pt 40, pb 20,
px 48 → `Header onClose` (label `APP_CLOSE`). Every Portrait variant is a fixed 176 tall
(`items-end`); code hugs its content (≈62dp) because the screens add the status-bar inset on top —
left as is, re-check against a full screen frame before forcing `u(88)`.
**`Type=Back, Layout=Landscape`** 851:11541 (added 11-10): only `Icon/Base` Back 50, 528×50, `items-end`
→ landscape `Header onBack` renders the Back icon alone (`size={25}`). No screen uses `Header` yet.
Landscape 528×74: title Bold 44 → `u(22)`; TitleButton = `Button/Secondary` Outline → `tone="outline"`
(border `border/thin` 2px = 1dp). Orientation from `useWindowDimensions`. The description lists a
Portrait "Title" type the set does not have (code renders it anyway). `Badge/Credits` 681:148
confirmed: 107×60, px 18, gap 10, `surface/overlay` pill, `Icon/Credits` 40 ("do not recolor") +
`Header/Credits` Medium 32 Snow; the Ticket (54) only appears with Credits in Figma, code shows it
whenever `onBack && onTicket`.

## NavBar page — `BottomNav`

The page was called "Menu" until 2026-11-06; "Menu" is now the floating context menu (next section).
`NavBar` 635:211 Portrait 720×180 (fixed; the padding sums to 154, the rest is the home-indicator
area the code adds as `bottomInset`): pt 32, pb 40 (+ safe-area), px 64, gap 56, items `NavBar/Item`
82 (`u(41)`), background `surface/navbar` rgba(26,9,37,0.9) (`colors.navbar`) with `NavBar/Portrait`
(blur 16 + shadow 0/-4/4 **Black 25%** = `colors.black25`; since the designer's 2026-11-03 pass the
Portrait fill is `surface/navbar` as well — `colors.navbar` in both orientations). Active = the gradient `Icon/NavBar` + the "Selected indicator"
on the bar edge: a 74×2 `#E8B9FF` line whose ends fade (stops 10%/50%/90%) over a 56×103 ellipse
(Landscape 75×42, on the rail's *inner* edge) centred 4.5px outside the edge, `#DF9CFF`→`#865E99` 0%
fading into the bar, blur 10, 60%, mix-blend lighten, clipped to the bar. In code the blur is a
radial falloff (`SelectedIndicator` in `BottomNav.tsx`): react-native-svg's `FeGaussianBlur` on
Android is a RenderScript blur capped at 25 device px, far below Figma's 10px σ at 3×. Landscape = a 100-wide
left rail, py 40, gap 37, 64px items, blur 16. Items in Figma: Explore/Backpack/Settings/Profile
(portrait); the Landscape rail shows **six** (Explore, Social, Alerts, Backpack, Settings, Profile —
649px of content in a 640 rail, so it clips). The app has four destinations; `NavKey` stays four
until Social/Alerts exist as screens.

## Menu page — `Menu` (`Menu.tsx`), added 2026-11-06

`Menu` 736:32 + `Menu/Item` 736:31. Container 320 wide (`u(160)`, was 319 until 11-09), px 24, py 16, gap `space/26`, radius 12,
`surface/menu` #1B002D (`colors.menu`), 1px `border/menu` (`colors.lavender70`, `u(0.5)`), **`Elevation/Menu`** =
0/8, blur 12, Shadow 40% (`elevation.menu`, only defined on this page — the Elevation page still lists
Down/Up). Row 64 tall filling the width, px 16, gap 10, radius 12; optional leading icon in a 32 slot
(the node draws `Icon/Base` Location at 36.4 px overflowing the slot, the description says
`Icon/Menu`; code draws any `IconName` at 18.2 in a 16dp slot), label text style **`Menu/Item`**
Inter Medium 28 Snow (`AppText variant="menuItem"`), optional trailing `Icon/Base` Check 40; Selected = Black 40%
bg + Lavender Light text/icons (code also uses it while pressed — there is no Pressed node). API:
`<Menu items={[{ key, label, icon?, selected?, check? }]} onSelect />`; the caller positions and
dismisses it. Consumer: `rn/src/menu/KebabMenu.tsx` (Discover's three-dots popover, now `menuDot`);
preview `extra/menu`.

## Cards_Backpack page — `ItemCard`, `RarityBackground`, `AvatarCard`, added 2026-11-09

`Rarity-Backgrounds` 785:970 (192, radius 12, 2px Snow border = 1dp): Purple 350 base + two radial
glows, no blur — the rarity Light colour at (8%, 4%) r 78% and the base colour at (70%, 68%) r 100%
(Exotic: Light twice). Default = Card Light `#B48CD7` + Snow / Gray glows at 90%; Disable = Silver +
Gray / Silver, 80%; Empty = Black 40% with a black border and a Snow 30% plus (16×2dp). Code:
`RarityBackground` in `ItemCard.tsx`.
`Cards-Wearables` 785:1018 (192 → `ItemCard`, 96dp, `elevation.down`): image 176×177 from the top
(inset 4dp, 7.5dp from the bottom, 80% when Disable). `state`: `default`; Pressed (only from
default) = background inset 2.5dp + a 1.5dp Lavender Light ring radius 8; `equipped` = 1.5dp
Lavender ring 2.5dp **outside** the card + a Shadow 80% bar with the 18dp check; `equip` /
`unequip` / `detail` = 2dp Orange ring outside + the `_Backpack-ButtonCard` bar (20dp, inset 1dp,
bottom radii 5, `Btn/Medium` Bold 12 uppercase; Equip Ruby, Unequip/Detail Shadow) when
`actionLabel` is given; `selected` (emote wheel) = 5dp Orange ring, radius 9. `empty` slot with
Default / Pressed / Selected (Black 60% + ring) / Disable (60%, plus 20%). `price` = Premium credits
pill (Black 60%, h 18, px 6, gap 3, `Icon/Credits` 13 + Medium 12); `newLabel` = Ruby `New-Tag`
(Bold 9 uppercase, radius 2) top-right. The old `equipped`/`caption`/`strip`/`outline` props are
gone; the backpack grids map their strips onto `state`.
`Avatar-Card` 790:2789 (160×360 → `AvatarCard`, radius 16, `elevation.down`): Default Black 40%,
Pressed Black 50% + 1px Lavender Light, Selected Black 70% + 4px **Light Rainbow** ring (SVG stroke,
CSS 147.6°); the avatar image `contain` with 10.14% top/bottom margins. Not used by a screen on this
branch (the entry flow's avatar picker is on `feat/entry-flow-parity`).
`Card-Profile` 825:7523 (added 11-10, 150 wide → `ProfileCard`, `u(75)`): Black 40% card, radius 8;
`Type=Wearable|Emote` = the `Cards-Wearables` tile scaled to 150 (code: `ItemCard` at 75dp), `Type=Name`
= a 150 square radius 10, base `#604CB0` + violet `#BC93FF` (27%, 27%) and peach `#F4B57B` (85%, 117%)
eight-stop glows, 1.56px Snow border, `shop/names` 28 (`Icon` `shopNames`) over Bold 24 name and
Regular 18 suffix (`DCL.ETH`). Info: p 10, gap 6, title Semi Bold 16 Snow (two lines, ellipsis),
`Badge/Credits` (coin 20 + Medium 16, gap 5, py 4); the Name info is fixed 92 with space-between.
The profile's equipped strip (`EquippedRow`) uses it: the old rarity tag and category badge are not in
the node and are gone. The price has no data source yet, so code shows the badge only when `price` is set.

## Cards_Discover page — `DiscoverCard`, `DiscoverCardSkeleton`, `DetailRow`, added 2026-11-09

`Cards-Discovery` 790:3141: Places 294 wide, image 156 (`DISCOVER_CARD.place` = 147 / 78dp);
Event 406, image 240 (203 / 120dp); image radius 8, 0.6px Lavender Light border (`u(0.3)`). Places
content pt 14 pb 18 gap 12: title `Pill/Large` SemiBold 26 one line, then Players / Like (`Icon/Base`
28 + `Caption1` Regular 24 Snow, gap 4, groups 16 apart). Event: the image holds the tag row
(pt 16 px 16 pb 12, `Tag/EventTime` left, `Tag/Event` right → `tags` slot), content pt 20 pb 24
gap 12: title H4 SemiBold 28 one line, "by" Lavender Light + creator Snow (Regular 24, gap 4).
`ImageNotFound` (image empty or failing): Places Deep Purple 50% + Lavender 50% border, Event Deep
Purple 40% + Lavender border; `Icon/Base` imageOff 70 / 80 in Lavender Light 70%. Both app card sets
(`src/discover/Cards.tsx`, `src/menu/Cards.tsx`) render it; their tag rows pass `inline`.
`Card-Discover-Skeleton` 806:88: the card's size from `Skeleton` Dark Wave — image + two bars
(Places 28/28 px gap 16, Event 34/29 gap 12) → `DiscoverCardSkeleton`, used by `LoadMoreCard` and
the in-game Discover panel's loading rail.
`Detail` 790:3188 → `DetailRow type value`: `Icon/Base` 32 + `Input/Value` Regular 30 Snow, gap 4,
p 8; Players / Likes / Coords / World (truncates at 148) / Date / Duration. No screen uses it yet.
`Feature` 819:3463 (missed on 11-09) = the Discover hero carousel (`src/discover/FeatureBanners.tsx`):
Selected 624×350 → 312×175dp, radius 8 (`u(4)`), 1px Lavender Light border, a Shadow 0 → 90% gradient over
the bottom 169 px, Name-Button row px 32 py 30 gap 32 with `Pill/Large` SemiBold 26 Snow (2 lines) and a
`Button/Primary` JumpIn 54 (radius 8, icon 24). Code follows those since 11-10. `State=Default` (the
neighbours) is 530×300 with no text; Discover-FeatureBanners (`Tz8ekjVNmv2OpkmfZb8KwU` 1767:39473) lays
them out 8dp from the settled card, vertically centred, the settled one on the 24dp gutter. Code: the
slots stay 312 + gap 8 and `SnapCarousel` hands each card its scroll `position`; `FeatureBanners` scales
side cards to 0.85, pulls them in by the lost width and fades the gradient and text out. `Image Not Found` 821:3478 = the Places ImageNotFound image
alone (already what `DiscoverCard` draws).

## Skeleton page — `Skeleton`, added 2026-11-09

`Skeleton` 790:1783 (200×40, radius 4 → default `radius` 2): Theme Dark = White 8%, Light = Black
8%. Wave = a half-width band (transparent → 8% at 48.4% → transparent) sweeping from −50% to 100% in
1 s ease-out; Pulse = 8% → 4% and back, 0.75 s each way. Code: `animation` / `theme` props
(defaults wave / dark); it was a 7% block with a 12% sheen at 1.2 s linear.

## EmptyState page — `EmptyState`, added 2026-11-10

`EmptyState` 866:372: `Icon` True/False × `Size` Large (624 wide → `u(312)`) / Small (480 → `u(240)`), gap 24.
Icon badge 120 (`u(60)`), 4px Purple 100 `#DCCBEC` border (`colors.purple100`), `radius/2xl`, the icon 100
(`size={50}`, Purple 100; swap = any `Icon/Base` or `Icon/NavBar`). Text Purple 100 centred: Large H2
SemiBold 42 + B1 Regular 26/34, gap 10; Small H4 SemiBold 28 + Caption1 Regular 24, gap 6. Code:
`EmptyState icon? title body? size='lg|sm'`; `actionLabel`/`onAction` stays as a code-only retry button
(Profile load error) — the node has none. Examples 866:373 sit on `background/brand`.

## Spacing & Sizes page — `space`, `radius`, `controlHeight`, `iconSize`, `border`, `minTouchTarget`

Page header: "Values in Figma units (2x). In React Native they are divided by 2: always use the
package tokens (space, radius, controlHeight, iconSize), never the raw number." Added to `tokens.ts`
on 2026-11-03 (`radii` stays as the older alias set): `space` none/3xs/2xs/xs/sm/md/lg/xl/2xl =
0/4/6/8/12/16/24/32/48; `radius` xs/sm/md/lg/xl/2xl/pill = 4/8/10/12/16/24/999; `controlHeight`
lg/md/sm/pillLg/pillMd/tag/badge = 96/74/54/60/56/42/32; `iconSize` buttonLg/Md/Sm/iconButton/pill/tag
= 32/28/24/40/32/26 (dp values, not `u()`-scaled: icons are drawn sizes); `border.thin` = 2px = 1dp
("button and input outline" — outline buttons were 2dp); `minTouchTarget` 88 = 44dp (extend
smaller controls with `hitSlop`). `Button` and `IconButton` consume them; the rest of the package
still carries the same numbers inline and can migrate file by file.

## Elevation page — `elevation.down` / `elevation.up` / `elevation.menu` in `tokens.ts`

One shadow, Shadow #161518 at 50%, blur 4, offset 0/±4: Down for things below their trigger
(dropdown list, menu), Up for things above it or at the bottom of the screen (dropdown opening up,
bars, sheets). The Menu page adds a third effect style, `Elevation/Menu` (0/8, blur 12, 40%), not
listed on this page; its "In React Native" paragraph still describes the old iOS `shadow*` / Android
`elevation` split, code uses `boxShadow`. RN: `boxShadow` (new architecture, both platforms) with the values ÷ 2 — offset
0/±`u(2)`, blur `u(2)`, `#16151880`; the old `shadow*` props are iOS-only and Android `elevation`
drew nothing visible on the slider thumb. Shadows only draw on a solid background. `Blur/Background` (blur 4 on
Black 40%) needs a blur library; without one the surface is Black 60% with no blur.

## Motion page — `motion` in `tokens.ts`

Durations `instant` 100 / `fast` 150 / `base` 200 / `slow` 300 ms; easings `standard`
cubic-bezier(0.2, 0, 0, 1), `enter` (0, 0, 0, 1), `exit` (0.3, 0, 1, 1) → `motion.duration` /
`motion.easing` (added 11-09, no consumer yet). Pattern table: Toast "dismisses after 3 s" →
`Toast` default 3000 ms (was 2500). Not implemented, flagged: Modal "backdrop fade + scale 0.96 → 1"
(`Dialog` only fades), Pressed "scale 0.97" (no pressable scales), "Reduce motion: everything becomes
a fade at fast" (the `Loader` description asks the spinner to keep rotating slower, which code does).

## States page — the rule every component follows

Each state uses a Color-collection token, never a hex: Default (`action/primary`,
`surface/overlay`, `surface/subtle-strong`) → Pressed (`action/primary-pressed`,
`surface/overlay-pressed`: one step darker of the same colour, handled by `Pressable`, never a prop) →
Focus (1px Lavender Light border while the keyboard is open) → Selected (Lavender Light bg + Shadow
text, or Lavender Light text + check in lists) → Disabled (Black 30% `surface/overlay-disabled` +
Silver 50% text, no touch, not announced as actionable) → Error (red border + message, never colour alone). Mobile only: no
hover, no keyboard focus. Resolved on the page: Pill Selected is Lavender Light in every size;
Input/Field, CodeSlots and Dropdown errors use `status/danger`; `Button/Filter` has a Pressed;
`Button/Discover` Pressed is one step darker (purple 60%). The Disabled rule was rewritten to Black
30% (`surface/overlay-disabled`) on 2026-11-03 and now matches every Disabled node (`black30`).

## Tags page — `EventTimeTag`, `EventBadge` (`Tags.tsx`); `Tag/Category` not built

`Tag/EventTime` 35:761: label `Tag/Label` Semi Bold 20 uppercase, px 8, py 4, `rounded-[4px]`;
Live = Ruby/Snow + the Players box (Shadow bg, h 30, gap 2, Green #30CD00 dot 10, `Icn/PlayersOutline`
22, `Tag/Count` Medium 20); StartingSoon = Snow bg, Ruby text + Ruby border; InMinutes/InHours/
InDays/Date = Snow bg, Shadow text + border (0.68px in Figma; 1dp hairline in code). `Tag/Event`
35:785: h 32, same padding/radius, Snow B5 label (not uppercase) on a gradient — Trending CSS
`-73.2°` #22C1C1 → #34C070, Featured CSS `147.6°` #FFBC5B → #FF2D55 50% → #C640CD. `LinearFill`'s
`angle` is 0 = top→bottom, so a CSS angle θ becomes `180 − θ` (253 / 32); the old 17 / 148 ran the
Featured gradient bottom-left→top-right. `Tag/Category` 35:591 (h 42, White 10% pill, 26px MapPins icon + Tags Semi
Bold 18 uppercase) needs the 12 MapPins icons, which are not an `Icon/*` family — left for the
Discover pass; `src/discover/CategoryChips.tsx` is the current stand-in.
`RarityLabel` 825:6933 (added 11-10, missed on 11-09) → `RarityLabel rarity label`: SemiBold 16 uppercase
lh 1.25 in `rarityColors[r].dark` (Base: Gray) on the same colour at 20% (Base: Pearl 20%), px 8 py 4,
radius 6 (`u(3)`; the node reads a 6px `radius-xs` from another library, not this file's 4). The caller
passes the translated name. No screen uses it yet (the profile strip dropped its tag on 11-10).

## Names page — in-world nametags, not built

`Nametag` 562:3419 (Unclaimed/Claimed/Foundation), `Nametag/SpeakingFeedback`, `DCLTag`, plus the
`Icon/NameVerified`, `Icon/NameLogo`, `Icon/DeviceMobile|Desktop` glyphs (imported into the icon
tables). Nametags are drawn over avatars in the world; survey the states and radii when that screen
is ported.

## Backgrounds page — `BrandBackground`

`Background` 676:6 (Plain/Glow × Portrait/Landscape) is no longer a baked image (11-09):
`background/brand` `#470071` (`colors.brand`) plus, for Glow, radial gradients with no blur —
Legendary top-left (portrait stops .4985 → 0, landscape .4999 → 0) and, in portrait only, Mythic
bottom-right (.2911 → 0). `BrandBackground glow` draws them with Figma's own `gradientTransform`
scaled to the view (they are rotated ellipses). The description says Plain is for loading /
Engine-Load. No screen uses it yet; Settings and Edit Profile now read `colors.brand` for their own
gradients. `Background/Avatar` 677:2 is still a PNG export.

## Loader page — `Spinner`; `Loader/Logo` not built

`Loader/Spinner` 562:8817: 150×150, a 16px Black 40% ring **centred at radius 53** (outer ≈123, not
the full box) with a Ruby arc from 141° to 300° (**159°**; both fixed 11-09, code had 75 and 135°) that rotates
360° every 1s, linear, infinite; slower under Reduce Motion; `ActivityIndicator` in `action/primary`
is the documented fallback. Code: `Spinner` (`size` in dp, stroke keeps 16/150), used by
`Button loading`. `Loader/Logo` 562:8818 is a 154×154 raster of the Decentraland logo with a pulse
at `motion.duration.slow` (no value defined on the Motion page, still PROPOSAL); `LoadingCover`
keeps its own `logo.png` until design exports the vector.

## Safe areas page — reference only, not actionable in RN

Documents per-platform/orientation margins (Android vs iOS, portrait vs landscape — status bar,
home indicator, notch/Dynamic-Island heights as literal px callouts like `102` labeled "Safe area"
for iOS). This is designer-facing reference for how much margin to leave in mockups — in RN,
`react-native-safe-area-context`'s `useSafeAreaInsets()` (already used by `FlowFrame`) resolves the
real per-device inset natively at runtime, so there's no fixed number here to copy into code.
