# Backpack B (manifest seq 140-175)

Figma file `jkkuZWmTjjFlhlqWUQoGF4`, page `20:2779`. 36 screens captured (30 portrait 720x1600, 6 landscape 1600x720); all files exist under `docs/rn-ui-spec/shots/backpack/`. Companion report: `backpack-a.md` (seq 104-139: body/face/clothing pill design, colour editor, emote selection frames). Coordinates are Figma px measured from the screenshots (+-4px). RN `u()` = Figma px / 2 (360dp-wide frame; landscape frames are 720 px = 360dp tall, so the same scale applies).

Capture note: the Figma MCP seat hit its call limit (burst of ~4 calls per ~minute, shared with the other backpack agent); every node was retried and none failed. I did not spend `get_design_context`; tokens below come from the screenshots plus the existing `rn/src/ds` tokens, which already match (grid 3x192 px tiles, 24 px gutter, 24 px gap = `layout`).

## 0. Two design generations in this range (important)

The Figma page mixes two generations of the same screen:

- **Pill generation (older)**: sub-categories are lavender pills under the tab strip (`Chest Legs Feet Hands`, `All Hat Helmets ...`). Seq 144-160 (the EXTRAS/Accessories set, Menu set, Equip01/02, Skeleton, Android/iOS, All 861).
- **Slot-tile generation (newer, has the sheet drag handle)**: sub-categories are 124x124 px square thumbnail tiles with a label under them (the thumbnail is what is worn in that slot, "+" in a dark tile when empty). Seq 162-168, 174-175 (portrait) and 169-173 (landscape). **RN already implements this generation** (`SlotTile`), so treat the slot-tile frames as the source of truth for layout; the pill frames are the source for states the new frames do not draw (empty-state copy, filter menu, skeleton, equip feedback, Extras taxonomy).

## 1. Screen inventory

| seq | id (suffix) | name | what it shows | state / variant |
|---|---|---|---|---|
| 140 | 880-18121 | Emotes | Emotes mode, slot 2 selected (ruby border, slightly larger tile), grid: item 1 orange UNEQUIP, item 2 lavender-border + dark strip with a check mark only, rest UNEQUIP | selected slot's own emote shows a check, others UNEQUIP |
| 141 | 880-18182 | Emotes | Toast "Equipped ✓" over the avatar's feet; slot 2 thumbnail greyed (loading/replacing) with white border; grid item 2 plain (not equipped) | equip transition frame 1 |
| 142 | 880-18244 | Emotes | Same toast; slot 2 now ruby-bordered again with a new (standing) thumbnail; item 2 plain, item 3 UNEQUIP | equip transition frame 2 (done) |
| 143 | 880-18306 | Emotes | Slot 2 selected, no toast/name chip, all grid tiles UNEQUIP except item 2 (plain) | steady state after equip |
| 144 | 239-8072 | Accessories-Hat | EXTRAS tab, pill row `All`(active) `Hat Helmets Tiaras Glasses Ma..`; 3-col grid (fox hat, spiral goggles, crystal earrings, flower glasses, tiara, heart glasses ...), nothing equipped | pill generation; "All" sub-pill = everything under EXTRAS |
| 145 | 186-3439 | Accessories-Hat | `Hat` active; empty state (shirt icon in rounded square, "Nothing here yet", "Find new ones in the" + underlined lavender link "Marketplace") | empty |
| 146 | 186-3838 | Accessories-Helmet | `Helmets` active; same empty state | empty |
| 147 | 206-6599 | Accessories-Tiaras | `Tiaras` active; 5 tiaras in a 3-col grid (last row has 2, third cell empty) | partial last row |
| 148 | 215-4551 | Accessories-Glasses | `Glasses` active; glasses grid, 4th row clipped by the nav (scrolls) | scrollable |
| 149 | 215-4822 | Accessories-Mask | pill row scrolled to the right: `Helmets Tiaras Glasses Mask(active) Earrings`; empty state | empty, pill row scrolled |
| 150 | 215-5262 | Accessories-Earrings | `Earrings` active (row scrolled, "Helmets" clipped on the left); earrings grid | scrolled pills |
| 151 | 215-6293 | Menu | CLOTHING tab, `Chest` active; jacket equipped (UNEQUIP), robe, skeleton torso, yellow tee, sweater, hoodie | baseline for the menu set |
| 152 | 215-6540 | Menu Open-All | Filter dropdown open: `All` (checked, darker row, lavender check) / `Collectibles`; anchored right-aligned under the filter button; avatar partially covered | filter menu, All |
| 153 | 215-6808 | Menu Open-Collectibles | Dropdown with `Collectibles` checked; same placement | filter menu, Collectibles |
| 154 | 215-7318 | Menu | Collectibles applied: only 3 rarity-coloured tiles (tee, sweater, hoodie); red dot on the filter button's top-right; menu closed | filtered grid + active-filter dot |
| 155 | 373-9823 | Equip01 | CLOTHING/Chest, hoodie (jacket 1) equipped (UNEQUIP), puffer jacket plain; avatar wears the hoodie | before equip |
| 156 | 373-10073 | Equp02 | Puffer jacket now UNEQUIP, hoodie plain; avatar wears the puffer (white/green) | after equip: same-category item swapped, avatar updates |
| 157 | 727-13070 | Skeleton | CLOTHING/Chest, tab strip + pills visible, grid = 12 dim rounded purple placeholder squares (no images); avatar unchanged | loading / skeleton |
| 158 | 855-14530 | Android | BODY, `Skin` pill active; empty state "Nothing here yet" + plain (non-link) "Find new ones in the Shop" | empty, Android copy |
| 159 | 855-14723 | iOS | Same but only "Nothing here yet" (no hint line); icon+title sit ~24 px lower (vertically centred without the hint) | empty, iOS copy |
| 160 | 861-15341 | All | ALL tab (no handle), grid mixed catalogue; first equipped tile only on row 3 (catalogue order, not equipped-first) | all, older ordering |
| 161 | 1261-16342 | Wearables (landscape) | Full-bleed purple, centred wifi-off icon in rounded square, "Connection lost", "Please check your internet connection and try again." Frame is named "Wearables" but is the error state; no retry button | error, landscape; no header/nav |
| 162 | 1665-13665 | All | Slot-tile generation. Wearables pill active, handle visible (y~709), ALL tab, grid: 6 equipped tiles (UNEQUIP) first, then plain jackets | all, equipped first |
| 163 | 1663-13206 | Body | BODY tab: slot tiles `Shape` (selected, avatar thumb) and `Skin` ("+" dark tile); grid: female shape plain, male shape UNEQUIP | body shape picker |
| 164 | 1658-15000 | Face | FACE: `Hair`(selected) `Facial Hair` `Eye` `Eyebrows` `Mouth`(clipped); row scrolls horizontally; grid below (mock content) | face; thumbnails are placeholder art |
| 165 | 1462-12305 | Clothing | CLOTHING: `Chest`(selected, jacket thumb) `Legs` `Feet` `Hand`(+ empty); grid of chest items | clothing |
| 166 | 1658-15475 | Extra | EXTRAS: `Hat`(selected) `Helmets` `Tiaras` `Glasses` `M..`(clipped); grid | extras |
| 167 | 1462-13153 | Full | Sheet dragged to the top ("full"): handle y~194, tabs y~244, slot row Chest/Legs/Feet/Hand, grid starts y~512 and shows 4+ rows; avatar fully covered; header pills still visible | sheet = full |
| 168 | 1531-19038 | Full avatar | Sheet collapsed to its handle (y~1381); avatar enlarged (head at y~210, feet at y~1000 -> a bigger stage); no tabs/grid | sheet = collapsed |
| 169 | 1658-14380 | All (landscape) | LANDSCAPE: back chevron, `Wearables`(active) `Emotes` pills, filter button top-left; avatar left; ALL..EXTRAS tabs top-right; 3-col grid of 192 px tiles (6 UNEQUIP then plain) | landscape, All |
| 170 | 1665-14477 | Body (landscape) | `Shape`(selected) `Skin` slot tiles (124 px), grid: female plain, male UNEQUIP | landscape body |
| 171 | 1665-14732 | Face (landscape) | `Hair Facial Hair Eye Eyebrows Mouth`, Mouth cut by the screen edge (row scrolls) | landscape face |
| 172 | 1665-14225 | Clothing (landscape) | `Chest Legs Feet Hand`, CLOTHING tab, grid | landscape clothing |
| 173 | 1665-15169 | Extra (landscape) | Slots `Hat Helmets Tiaras Glasses Mask(clipped)` but the **CLOTHING** tab is highlighted (design bug, should be EXTRAS) | landscape extras |
| 174 | 1665-17382 | Body | Default-height sheet, BODY, `Shape` plain, `Skin` selected (ruby/orange border, "+"), empty state "Nothing here yet" / "Find new ones in the Shop" | skin slot empty |
| 175 | 1665-16724 | Full | Sheet at "full" height, tab strip highlights **CLOTHING** while the slot row is `Shape`/`Skin` (design inconsistency), `Skin` selected, same empty state | full + empty |

## 2. Flow graph

- Entry: bottom nav backpack icon (portrait, 4 icons: compass, backpack, settings, profile; backpack filled when active) or Continue/entry-flow footer. In landscape (169-173) there is **no bottom nav and no right rail**; a back chevron (x~133, y~60, 32 px hit) replaces them -> leaves the backpack (back to explorer / previous flow step).
- Header: `Wearables` / `Emotes` pills switch mode in place; filter button -> dropdown (152/153), choosing an option closes it and applies; red dot while filter != All (154); tap outside closes. Filter applies to the current mode's grid.
- Wearables tab strip ALL | BODY | FACE | CLOTHING | EXTRAS; tabs other than ALL show a slot row (tiles); tapping a slot tile selects it (orange border) and filters the grid to that category (BODY: Shape, Skin; FACE: Hair, Facial Hair, Eye, Eyebrows, Mouth; CLOTHING: Chest, Legs, Feet, Hand; EXTRAS: Hat, Helmets, Tiaras, Glasses, Mask, Earrings). Slot row scrolls horizontally when it overflows.
- Grid tap: unequipped -> equip (replaces the same-category item, avatar updates 155 -> 156); equipped (orange border + UNEQUIP strip) -> unequip. Body shape / eyes / mouth / eyebrows cannot be unequipped (RN `REQUIRED`).
- Sheet: drag handle; three heights: collapsed (168), default (162/174, top at ~43% of screen), full (167/175, top at ~11%). Drag up/down snaps; tapping the handle toggles.
- Emotes: tap slot thumbnail = select slot (ruby border); tap grid emote = plays it on the avatar and equips into the selected (or first empty) slot -> "Equipped ✓" toast (141/142); tap an equipped emote = unequip it (UNEQUIP strip).
- Empty category -> empty state; "Marketplace"/"Shop" hint -> platform policy (see 3.6).
- Error: catalog fetch fails -> "Connection lost" (161), retry on tap (RN adds a Try again button).

## 3. Per-screen spec

### 3.1 Shell, portrait (162-168, 174-175 and every pill frame)
- Stage: full-bleed purple, radial glow behind the avatar (centre ~x360, y~42%), floor halo near the bottom of the avatar area. Avatar is Godot-rendered into the transparent hole between the header and the sheet (RN `AvatarHole` -> `backpack_open/place/close`).
- Header row (y 86-146): `Wearables` pill (x160-360, 60 px tall, ruby when active, dark `#24033B` when idle, shirt icon + label 600/~28px?), `Emotes` pill (x384-545), filter button (x612-672, 60x60, radius ~12, dark `#24033B`, filter/sliders icon). Right edge 48 px from the screen edge.
- Sheet: top-radius ~24 px, vertical gradient `#460F6F` -> `#24033B`, handle 200x8 px, 30% white, centred 22 px under the sheet top.
- Tab strip: ALL BODY FACE CLOTHING EXTRAS, uppercase 600 ~28px, inactive `#CFCDD4`-ish, active white with 4 px ruby underline; y~746 (default) / 244 (full). Evenly spread across x48-672.
- Slot row (slot-tile): tile 124x124 (inner image 112 + 6 px frame), gap 32 (pitch ~156), first tile at x48; selected = 4 px orange border (`#FF7439`) with radius 12, unselected = no border; empty slot = dark `#2E0A47`-like tile with centred "+" (silver, thin); label under tile 28 px regular white ("Chest", "Facial Hair"). Row y 306-431 (full) / 809-935 default.
- Grid: 3 columns, tile 192x192, gap 24, gutter 48, radius 12, 2 px white-ish border (equipped: 4 px orange `#FF7439` border plus a 44 px black `#161518` strip along the bottom with "UNEQUIP" 700 ~28px white uppercase). Background of an item = diagonal gradient of its rarity (base: `#B48CD7` -> `#FCFCFC`; common `#73D3D3`..., see `rarityColors`). Last row is padded to 3 columns (147).
- Bottom nav: 4 icons on `#161518`, y 1419-1600 (incl. home-indicator inset).

### 3.2 Emotes (140-143, backpack-a 107-121 for the rest)
- Slot row sits **above** the sheet at y~681-800: 10 tiles of 112x112, gap 16, first at x48, horizontally scrollable (5 visible + a clipped 6th). Tile = white-ish gradient with silhouette/thumbnail; border 2 px white 80%; selected tile = ruby (`#FF2D55`) 3-4 px border and ~125 px (slightly larger). Empty slot = dark tile with "+". While an equip is in flight the slot thumbnail is greyed (141).
- Name chip: dark rounded rectangle (`#161518`, radius ~12, ~195x44) centred at y~615-670 with the emote name (backpack-a: "Emote Name01") while a slot is selected; replaced for ~0.5 s by "Equipped ✓" (text 500 ~24px white + check icon) after equip, then back to the name (legacy `emote_name.gd`: name slides up 0.3 s, equipped notification slides up, holds 0.5 s, slides down).
- Grid in the sheet: same 192 tiles. States per tile: plain (not equipped), equipped in another slot (orange border + UNEQUIP), equipped in the selected slot (lavender/white 4 px border + dark strip with ✓ only, 140).
- Sheet content starts directly with the grid (no tab strip); sheet default height; nav visible.

### 3.3 Filter menu (152-154)
- Panel x344-672 (328 wide), y162-347, radius 12, fill `#24033B`-like very dark purple with 1 px lavender 45% border and shadow; two rows 84 px tall, text 500 32px? (16dp) lavender/white; the selected row has a darker `rgba(0,0,0,.35)` background and a lavender check on the right. Options exactly: `All`, `Collectibles`.
- Collectibles = items with a rarity (non-base). Red dot (~16 px, `#FF2D55`) on the filter button top-right while filter != All; the button itself is unchanged.

### 3.4 Item grid states
- Loading skeleton (157): same grid with 12 tiles of translucent purple (`rgba(105,31,169,.28)`, radius 12, no border) filling the sheet; tab strip and slot row stay visible.
- Empty (145/146/149/158/159/174/175): centred block in the free area of the sheet: 96x96 rounded-square icon (shirt silhouette, white outline 3 px), title "Nothing here yet" (700 ~32px white), hint line(s) 400 ~24px. Variants: (a) `Find new ones in the` / underlined lavender link `Marketplace` (145-149); (b) `Find new ones in the Shop` plain (158, 174, 175); (c) title only (159 iOS). Landscape empty state is not drawn.
- Partial last row 147: two tiles + empty cell (keep column width).
- Error 161: see 3.7.

### 3.5 Landscape layout (169-173, 1600x720)
- Background identical gradient; avatar column on the left (avatar centred x~420, head y~150, feet y~650; floor halo y~620), no sheet, no handle, no bottom nav, no right rail.
- Top-left: back chevron (x~120-145, y~40-80), `Wearables` pill (x238-427, y33-93, 60 px), `Emotes` pill (x447-601), filter button (x639-699, y33-93, 60x60).
- Right panel: x868-1492 content column (right inset 108 px = notch/safe area on the Figma phone; use safe-area insets), **no panel background** (gradient shows through). Tab strip at y~57 (underline y~85), spread x873-1487. Slot row y120-244 (tile 124, pitch 156, label y~271; overflows off-screen -> scroll). Grid starts y~325 (below slots) or y~120 (ALL tab), tile 192, column pitch 216 (x868/1084/1300), row pitch 216; content scrolls vertically.
- Emotes in landscape is in backpack-a 108 (vertical emote-slot column + 3-col grid in a dark panel + right nav rail); no backpack-b frame adds to it.

### 3.6 Platform copy for empty state (Android/iOS frames 158/159)
Legacy (`no_items_message.gd`): rich state ("Find new ones in the <link>Marketplace</link>", opens `DclUrls.marketplace_browse(section)` via `MarketplaceTracker.open_and_track`) only when `not Iap.is_available() and StorePolicy.can_show_external_purchase_links()`; everywhere else just the label `BACKPACK_NOTHING_HERE_YET`. Figma shows Android with "Find new ones in the Shop" (no link styling) and iOS with title only. RN should drive this from the same two flags; link vs plain text on Android and the word Marketplace vs Shop are open questions (section 7).

### 3.7 Connection lost (161)
Full-screen state (replaces the whole backpack, no header, no nav in the frame): icon 140x140 px (rounded square 8 px radius, outline `#CFCDD4`, wifi-off + "-  -" base line) centred at y~270, title "Connection lost" (700, ~44 px, `#E3D3F0`) y~418, body "Please check your internet connection and try again." (400 ~32 px, 80% opacity, centred, wraps to 2 lines at ~430 px) y~475-520. No button in Figma; RN adds "Try again" (keep).

## 4. Tokens and reusable components

All required tokens already exist in `rn/src/ds/tokens.ts` (`ruby #FF2D55`, `orange #FF7439`, `shadow #161518`, `lavender #E8B9FF`, `purple450/500/550`, `cardLight #B48CD7`, `rarityColors`, `layout` 24/12/3). Components in `rn/src/ds`: `Pill`, `Tabs`, `SlotTile`, `ItemCard` (+ `SkeletonCard`), `IconButton`, `BottomNav`, `GradientView/LinearFill/RadialFill`, `ColorPicker`, `AppText`. Missing in `ds`: a badge-dot on `IconButton`, a toast/name-chip component, an empty-state block (icon + title + hint), a `Chip`-style check-strip variant of `ItemCard`, a dropdown/menu primitive (RN's `FilterMenu` is backpack-local, fine).

## 5. Legacy counterpart (`godot/src/ui/pages/backpack`)

- `backpack_responsive.gd` swaps `backpack_portrait.tscn` / `backpack_landscape.tscn` by viewport aspect (x<y -> portrait). **Landscape is a first-class layout in legacy**; the RN comment "portrait only" is an assumption that Figma contradicts.
- `backpack.gd` (1267 lines) behaviours Figma does not draw: loads owned wearables from the lambda (`WearableRequest.async_request_all_wearables`) plus recent-owned fast marketplace API and base wearables; NEW badge (`set_new_badge`, per-wallet persisted counts, `newtag_*`); marketplace "recommended for you" section on iOS (IAP) at the top of the grid and preview-equip (`_async_marketplace_preview_equip`); blacklist change -> redeploy timer; `item_arrived` live inventory refresh after a web purchase; credits balance button (hidden in the entry flow, `hide_navbar`); `Button_BackToExplorer` visibility; `can_drag` avatar rotation/zoom (`disable_avatar_zoom`); `avatar_preview.focus_camera_on(category)` zooms the camera per category (head for FACE, full body for CLOTHING, as seen in backpack-a 122/136 but not in this range's frames); avatar-change VFX (`avatar_change_vfx.png`); unequip rule `_can_unequip` excludes body_shape, eyes, mouth (RN also blocks eyebrows).
- `emote_name.gd/.tscn`: the name chip + "Equipped" slide animation (`BACKPACK_EQUIPPED`).
- `no_items_message.gd/.tscn`: empty state described in 3.6. Locale keys: `BACKPACK_ALL`, `BACKPACK_BODY`, `BACKPACK_FACE`, `BACKPACK_CLOTHING`, `BACKPACK_EXTRAS`, `BACKPACK_COLLECTIBLES`, `BACKPACK_COLOR_LABEL` ("{category} color"), `BACKPACK_EQUIPPED`, `BACKPACK_NOTHING_HERE_YET`, `BACKPACK_FIND_NEW_ONES_IN_THE_MARKETPLACE` in `godot/locale/*.csv` (RN strings are hard-coded English).
- `backpack_button.gd`, `avatar_preview.gd` (1028 lines: the 3D preview, zoom/rotate input, emote playback) are engine-side and stay in Godot.

## 6. RN status and gap list (`rn/src/backpack`, ~1600 lines)

Exists: portrait shell (`BackpackScreen`), header pills + filter (`BackpackHeader`, `FilterMenu`), two-state `Sheet`, tabs + slot tiles + grid (`WearablesPanel`, `ItemGrid`), emote slot row + grid (`EmotesPanel`), colour panel/swatch (`ColorPanel`), connection-lost (`ConnectionLost`), skeleton cards, base-avatars catalogue + emote thumbnails, Godot bridge (`equip/unequip/set_body_shape/set_color/play_emote/set_emote_slot`, `backpack_open/place/close`).

Gaps vs seq 140-175 (exact):
1. **Landscape layout is missing** (169-173, 161). Needs a `Landscape` branch in `BackpackScreen` (it only renders `Portrait`): left avatar hole, header row with back chevron + pills + filter, right panel without sheet or nav (tabs, slot row, 3-col grid), rotation-safe `AvatarHole`. `SheetContent` already has a `bottomInset` prop meant for this.
2. **Sheet has 2 states, Figma has 3.** `Sheet` snaps `expanded` (`SHEET_RATIO` 0.455 of the window) / `collapsed` (handle only). Add `full` (top ~11% of screen, avatar covered, 167/175). Default 0.455 of the full window is the right ballpark (Figma default sheet top is ~43% of the screen, nav included).
3. **Emote feedback absent**: no name chip, no "Equipped ✓" toast, no greyed slot while swapping (140-143). `EmoteSlotsRow` selected border is orange (`slotSelected`), Figma uses ruby with a slightly larger tile; the selected slot's own emote in the grid should show the dark ✓ strip instead of UNEQUIP (140).
4. **Empty state is just a caption** ("Nothing here yet" in `WearablesPanel`); Figma has the shirt-in-square icon, title and hint with platform variants (3.4/3.6). The emotes grid has no empty state at all (Collectibles with none: backpack-a 135).
5. **Filter**: `filterActive` is also true while the menu is open and the dot is not drawn (`IconButton` only changes the background). Add a 16 px ruby dot on the button's top-right, only for `filter !== 'all'`.
6. **Skin slot semantics**: RN's `skin` slot is colour-only (opens `ColorPanel`). Figma 174/175 shows `Skin` as a wearable-category slot with its own grid / empty state (skin collectibles), and the older frames (backpack-a 104-111) have the colour row + wheel above the grid. RN has no `skin` wearable category in `taxonomy.ts` and no colour row in the slot-tile layout of Figma (the new frames draw none) -> see open question 1.
7. **Catalogue is base avatars only** (`WEARABLES_URL` = `collectionId=urn:decentraland:off-chain:base-avatars`): owned collectibles, rarity tiles, the Collectibles filter result (154), NEW badge and Marketplace/Shop deep links are not wired. `collectible: !id.startsWith(BASE_PREFIX)` can never be true today.
8. **Tab/slot taxonomy details**: Figma slot labels are `Hand` (RN `Hands`) and `Facial Hair`/`Eye`/`Eyebrows`/`Mouth` (match); the Extras row order Hat, Helmets, Tiaras, Glasses, Mask, Earrings matches. The pill frames' extra `All` sub-pill under EXTRAS (144) has no slot-tile equivalent (RN shows the first slot selected by default; decide whether EXTRAS opens on `All`).
9. **"All" ordering**: RN lists equipped first (matches 162); 160 lists in catalogue order (older frame).
10. **Back button**: Figma has none in portrait (RN adds one for the entry flow, fine) and a chevron in landscape.
11. **Strings are hard-coded**; legacy uses locale keys (`BACKPACK_*`), RN has no i18n layer yet.
12. **Slot row scroll affordance**: Figma clips the 5th/6th tile at the screen edge (no fade/scrollbar); RN's horizontal `ScrollView` already does that.
13. Skeleton: RN renders 6 cells; Figma 12 (fills the sheet). Cosmetic.

## 7. Open questions / inconsistencies

1. Where does colour editing live in the slot-tile generation? None of 162-175 draws the swatch row or the colour wheel, yet backpack-a (104-130) does (pill generation). Decide whether the swatch row stays above the grid for Skin/Hair/Eye/Eyebrows/Facial Hair (RN currently shows a "Color" swatch button row, an RN invention).
2. 173 highlights CLOTHING while showing Extras slots; 175 highlights CLOTHING while showing Shape/Skin; 160 uses a different catalogue ordering; 164/165/166/171-173 reuse mock thumbnails (jacket/pants/hair) for unrelated slots; don't copy the thumbnails, use the equipped item's thumbnail.
3. Frame names are unreliable (161 "Wearables" = connection lost; 156 "Equp02"; 144 "Accessories-Hat" shows `All`).
4. "Marketplace" link vs "Shop" plain text vs nothing (iOS) for the empty state: confirm Android policy (legacy gates the link on `StorePolicy.can_show_external_purchase_links()`).
5. The 4th slot is called `Hand` here, `Hands` in the pill frames and RN.
6. Does the avatar camera zoom per category (legacy `focus_camera_on`, backpack-a 122/137) or only change with the sheet height (168 shows a bigger avatar when collapsed)? Frames 162-166 all show the same full-body avatar.
7. "Equipped ✓" toast: only drawn for emotes; wearables show no toast (155 -> 156 just swaps the borders). Confirm that wearables get none.
8. Landscape right inset (108 px) is a notch/cutout allowance, not a design margin; use safe-area insets.
9. Which of the two selected-emote borders is canonical (ruby slot border in 140-143 vs orange in backpack-a 112-121)? RN uses orange.
