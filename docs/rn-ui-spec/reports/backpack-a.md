# Backpack A (manifest seq 104-139)

Figma file `jkkuZWmTjjFlhlqWUQoGF4`, page `20:2779`. 36 screens captured (35 portrait 720x1600, 1 landscape 1600x720). All files exist under `docs/rn-ui-spec/shots/backpack/`. Companion report: backpack-b (seq 140-175). Coordinates below are Figma px at 720x1600 portrait, measured from the screenshots (approximate, +-4px); RN `u()` = Figma px / 2.

Capture note: the Figma MCP seat hit its call limit repeatedly (shared with the other agent); every node was retried until it succeeded. No node failed.

## 1. Screen inventory

| seq | id (suffix) | name | what it shows | state/variant |
|---|---|---|---|---|
| 104 | 43-918 | Body-Body Shape | Wearables mode, BODY tab, `Shape` pill active; swatch row (rainbow wheel + 6+ skin tones, first tone ringed); 2 body-shape cards (female, male). Male = equipped (orange border, UNEQUIP strip) | default, wheel not selected, preset skin selected |
| 105 | 126-2652 | Body-Body Shape | Same header; sheet replaced by "< Skin color" editor: swatch row, COLOR hue slider, SATURATION slider, BRIGHTNESS slider | color editor (opened from the wheel) |
| 106 | 271-8440 | Body-Body Shape | As 104 but the rainbow wheel is the ringed (selected) swatch, i.e. custom color active | custom color selected |
| 107 | 83-2357 | Emotes | Emotes pill active; 5+ equipped-slot thumbnails row (scrolls horizontally, 6th clipped) above the sheet; 3-col emote grid, all 9 visible tiles equipped (orange border + UNEQUIP) | initial emotes, no slot selected, no name chip |
| 108 | 226-12623 | Wearables (landscape) | LANDSCAPE emotes: pills + filter button top-left, avatar left, vertical emote-slot column (x~600-710, 112px tiles), 3-col emote grid in a dark purple panel (x 720-1440), right rail with profile avatar on top then friends, bell, compass, backpack(active), settings, small profile | landscape layout; frame is named "Wearables" but shows Emotes mode |
| 109 | 126-2391 | Body-Skin-Empty | BODY tab, `Skin` pill active; empty state: shirt icon, "Nothing here yet", "Find new ones in the Marketplace" (Marketplace is a link, lavender underlined) | empty |
| 110 | 156-4350 | Body-Skin | BODY tab, Skin pill, swatches greyed/mauve (dim, disabled look), 1 skeleton-skin card (no equip) | skin swatches disabled/loading; SECOND TAB SET (BODY HEAD ACCESSORIES TORSO LEGS), see section 7 |
| 111 | 169-4714 | Body-Skin | Avatar replaced by skeleton w/ sombrero (preview of an equipped skin collectible); swatch row normal with wheel; 2 skeleton-skin cards, second equipped/UNEQUIP | skin wearable equipped; same alt tab set |
| 112 | 215-8041 | Emotes | Emote slot 1 selected; name chip "Emote Name01" above slot row; grid: item1 UNEQUIP (orange), item2 checkmark strip, rest partially visible with check strips | emotes, slot 1 selected |
| 113 | 215-8436 | Emotes | Slot 2 selected; chip "Emote Name02"; grid: slot1's emote shows check strip, item2 orange UNEQUIP, item 3 none | slot selected holds emote |
| 114 | 215-8842 | Emotes | Slot 2 selected; chip "Emote Name03"; grid item3 has pink/red EQUIP strip | tapped an emote not in the slot: pending equip |
| 115 | 912-19520 | Emotes | Avatar hidden (transition frame), toast "Equipped ✓" replaces the name chip; slot 2 has white (not orange) border | after equip: toast |
| 116 | 916-10419 | Emotes | Avatar plays wave emote (hand raised, hip), chip "Emote Name03", slot 2 selected, item3 UNEQUIP | emote playing |
| 117 | 916-10839 | Emotes | Slot 1 selected, chip "Emote Name01"; grid item1 UNEQUIP; items 3,4.. check strips; slot row is 8 px higher than 112 (header pills shifted left slightly: layout drift) | variant of 112 |
| 118 | 916-11246 | Emotes | Slot 1 is EMPTY ("+" in dark tile) and selected (orange border); no name chip; avatar feet visible; grid has no UNEQUIP on slot-selected tile | empty slot selected |
| 119 | 980-14266 | Emotes | Grid scrolled down: colored collectible tiles (lime, double-figure, blue), UNEQUIP on a tile mid-grid, last slot selected; chip "Emote Name01" | scrolled + collectible emote thumbnails |
| 120 | 984-15749 | Emotes | Slot 2 selected, chip "Emote Name02", item 3 EQUIP strip | variant of 114 |
| 121 | 984-16132 | Emotes | Slot 1 selected, chip "Emote Name02", item 3 UNEQUIP orange | variant |
| 122 | 169-4931 | Face-HairStyle | FACE tab, sub-pills `Hair`(active) `Facial hair` `Eye` `Eyebrows` `Mouth`; avatar camera zoomed to head+shoulders; swatch row (wheel + hair tones); 3-col hair grid, item 1 equipped | face, hair |
| 123 | 182-2523 | Face-FacialHair | `Facial hair` active; same swatches; grid of beards, 4th tile equipped (UNEQUIP) | |
| 124 | 182-2866 | Face-Eye | `Eye` active; eye swatches (dark, brown, tan, gold, grey, ...); eye grid, item 1 equipped | |
| 125 | 182-3242 | Face-Eyebrows | `Eyebrows` active; hair palette; eyebrow grid, item 1 equipped | |
| 126 | 182-3630 | Face-Mouth | `Mouth` active; NO swatch row (mouth has no color); grid larger, first row real, rows 2+ greyed/dim thumbnails (loading/unavailable) | grid with dimmed tiles |
| 127 | 342-12908 | Face-HairStyle-Color | "< Hair color" editor: 6+ swatches, COLOR/SATURATION/BRIGHTNESS sliders | color editor |
| 128 | 342-13098 | Face-FacialHair-Color | "< Facial hair color" | color editor |
| 129 | 342-13288 | Face-Eye-Color | "< Eye color", eye palette | color editor |
| 130 | 342-13478 | Face-Eyebrows-Color | "< Eyebrows color", hair palette | color editor |
| 131 | 226-9279 | Emotes | Emotes, slot row without chip or selected slot, all 9 tiles equipped UNEQUIP orange, slot thumbnails with white border, row y ~ 478-555 (chip row space empty) | emotes "all equipped" |
| 132 | 226-9697 | Emotes | Filter dropdown open (anchored under filter button, right aligned): `All` (checked), `Collectibles` | filter menu, All |
| 133 | 226-10131 | Emotes | Filter dropdown, `Collectibles` checked; menu anchored left of the button (x 238-467) | filter menu, Collectibles |
| 134 | 226-10557 | Emotes | Collectibles filter result: only colored-rarity tiles (lime, purple, blue, orange); red notification dot on filter button; no bottom nav in frame | filtered grid, active-filter dot |
| 135 | 1378-16445 | Emotes | Empty collectibles result: emote-icon, "Nothing here yet", "Find new ones in the Marketplace"; red dot on filter button; no bottom nav | empty |
| 136 | 186-4510 | Clothing-Chest | CLOTHING tab; sub-pills `Chest`(active) `Legs` `Feet` `Hands`; no swatch row; avatar full-body (not zoomed); 3-col grid: jacket (equipped, UNEQUIP), robe, skeleton torso, yellow tee, sweater, brown hoodie... | clothing |
| 137 | 206-5723 | Clothing-Legs | `Legs`; avatar is bigger (frame 1680x1244 in my sheet = same 720x1600) pants grid, item 1 equipped | |
| 138 | 206-5978 | Clothing-Feet | `Feet`; sneakers equipped, boots, sandals ... | |
| 139 | 271-8932 | Clothing-hand | `Hands`; 4 tiles (watch, cube, gloves, gloves), none equipped | few items |

(Seq 136-139 and the face/clothing screens use the same shell; each differs only in sub-pill, items and swatch row.)

## 2. Flow graph

- Entry: bottom nav backpack icon (nav: compass, backpack, settings, profile avatar; backpack active, icons only) -> Backpack in Wearables mode, tab ALL (default; ALL screens are in backpack-b).
- Header pills toggle Wearables <-> Emotes (no navigation, same shell). Filter button -> dropdown (All / Collectibles); choosing closes menu; button gets a red dot while a non-All filter is active (134/135). Tap outside closes.
- Wearables tabs: ALL | BODY | FACE | CLOTHING | EXTRAS (underline, red/ruby active). BODY -> Shape, Skin. FACE -> Hair, Facial hair, Eye, Eyebrows, Mouth. CLOTHING -> Chest, Legs, Feet, Hands. EXTRAS (backpack-b).
- Color flow: tapping the rainbow wheel in a swatch row (104/106/109.. 122-125) -> "< <Target> color" editor replacing tab bar+grid inside the sheet (105, 127-130). Back chevron returns. Preset swatch tap just sets the color (no navigation).
- Emotes: tapping slot thumbnail selects the slot (orange border), name chip appears with that slot's emote name; tap grid tile: not in slot -> pink EQUIP, tap again equips ("Equipped ✓" toast, avatar plays emote); equipped tile shows UNEQUIP orange (selected slot's emote) or check strip (equipped in another slot). Empty slot ("+") selected -> next tapped emote fills it.
- Back: Figma has no back button; Android back/hardware = leave backpack (RN `onBack` for entry flow). Sheet has a drag handle (collapsed/expanded).
- Marketplace link in empty states -> external marketplace (open browser/deeplink; see legacy iOS gating below).

## 3. Per-screen specification

### 3.0 Shell (all portrait screens)
- Background: purple radial glow behind the avatar, centred ~(360,330); deep purple gradient (#350854 base). Godot renders the avatar through a hole; RN draws background (RN: `StageBackground`, `AvatarHole`).
- Header: y ~96-136 (centre y~115). Pills `Wearables` (shirt icon) / `Emotes` (dancer icon), active = ruby fill (#FF2D55) white text; inactive = dark glass (#24033B/60% ish). Filter icon button 56x56 at right (x~478-523 in 560-scale = x 612-672), dark rounded square. Header row is centred-left: pills start x~143, gap ~24; filter button right margin ~48.
- Avatar stage: y 0-~690 (portrait Wearables/BODY/CLOTHING). Face tab: avatar camera zoomed so head+shoulders fill (122-130). Body/clothing/emotes: full body, feet at y~640.
- Sheet: top edge y~690 (wearables), rounded top corners ~28px, gradient #460F6F -> #24033B, no visible handle in Figma. In Emotes mode the sheet top starts lower (y~813) because the 10-slot emote row (y 674-789, tile 112, gap 14, selected = 4px orange border) + name chip (centred, y 595-650, text e.g. "Emote Name01", dark glass pill, white text 22px) sit above it.
- Bottom nav: y 1420-1600 (180px), #161518 solid, 4 icons centred at x 150/290/430/565 (compass, backpack(purple filled=active), settings, profile avatar 72px circle).
- Tabs row: y ~746 baseline; labels uppercase 26px semi-bold: ALL x~52, BODY ~176, FACE ~295, CLOTHING ~440, EXTRAS ~598; active = white + 4px ruby underline, inactive = lavender 70%. Left margin 52.
- Sub-pills: y 835 centre, height ~52, radius full; active = light lavender fill (#E8B9FF-ish) dark text; inactive = translucent purple (#691FA9 40%) light text; font 22px medium; gap 16.
- Swatch row (Body shape, Skin, Hair, Facial hair, Eye, Eyebrows): y centre ~940, circles 80px, gap 28, first = rainbow wheel, then palette; selected = 3px ruby/pink ring with 4px gap; row scrolls horizontally (last circle clipped at the right edge).
- Item grid: 3 columns, card 193x193 (radius 8px, 2px #fff border at 80%; equipped = 4px orange #FF7439 border + bottom strip "UNEQUIP" 34px #161518 white bold 22px). Column x 49, 265, 481; row pitch ~216 (gap ~24). Card background: diagonal gradient #B48CD7 -> #FCFCFC (base) or rarity gradient (collectible, see rarityColors in `rn/src/ds/tokens.ts`). Grid scrolls under the nav.
- Grid starts y ~1022 under swatch row (wearables with swatches), ~1000 under sub-pills without swatches (Mouth, clothing), ~850 emotes.

### 3.1 Body-Shape (104, 106) / color editor (105)
- Items: 2 base body shapes (BaseFemale, BaseMale) from the peer lambda collection; equip = `set_body_shape` (cannot be unequipped; the legacy hides UNEQUIP? Figma shows UNEQUIP on the equipped one, legacy `_can_unequip` blocks body_shape; follow legacy: no-op).
- Swatches: rainbow wheel, then skin palette. Selected ring follows avatar skin color; wheel selected when the color is not in the palette (106).
- Editor (105): header "< Skin color" (back chevron 32px at x 52, title 30px medium white at x~110); swatch row y~600 (circles 80px, no wheel); three labelled sliders (labels COLOR, SATURATION, BRIGHTNESS 16px caps, lavender at y~675/780/883 of 560-scale; i.e. y~970/1120/1270) tracks 6px-tall pill rounded 40px high gradient, width x 48-672; thumb 56x56 rounded square with white border (hue thumb filled with current hue color, sat/brightness thumb grey). Slider colors: hue rainbow, saturation grey->orange, brightness black->white.
- Data: `set_color {target:'skin',color}` throttled (legacy rebuilds avatar on each color).

### 3.2 Skin states (109, 110, 111)
- 109 empty: sub-pill Skin; centre y~1100: 120px outline shirt icon, "Nothing here yet" 40px semi-bold white, "Find new ones in the " + underlined "Marketplace" link 28px lavender. Shows when skin wearables list is empty (note: skin colors list is separate from skin wearables? Figma Skin pill shows both swatches and skin wearables; 109 shows NO swatch row).
- 110: swatches desaturated (mauve) = disabled while a skin wearable is equipped or loading; card = unequipped skeleton skin.
- 111: skeleton avatar rendered; swatches normal again; equipped card second.
- Open question: swatch row visibility differs across 109/110/111 (see section 7).

### 3.3 Emotes (107, 112-121, 131-135, landscape 108)
- Slot row: 10 slots (RN `EMOTE_SLOTS`=10); thumbnail 112px; empty = dark tile with "+" (118); selected = 4px orange; after equip frame flashes white (115).
- Name chip: shows name of selected slot's emote; hidden when empty slot selected (118). Animates up from below 0.3s (legacy `emote_name.gd`). "Equipped ✓" toast replaces chip for 0.5s on equip.
- Grid tile strips: UNEQUIP (dark strip, orange border) = this emote is in the SELECTED slot; check strip (grey strip with white check) = in another slot; EQUIP (ruby/pink strip, orange border) = tapped candidate not in selected slot; no strip = not equipped.
- Tapping a tile also previews the animation on the avatar (116).
- Collectible emotes show rarity-coloured tile backgrounds (lime=exotic? lavender=?, blue) with a ghost double-figure thumbnail for multi-avatar emotes (119/134).
- Empty result (135): emote-dancer icon, "Nothing here yet", "Find new ones in the Marketplace".
- Data: base emotes list (`rn/src/backpack/emotes.ts`, 21 base) + owned emotes (peer lambdas, not in RN yet), `play_emote`, `set_emote_slot {slot,urn}`.
- Landscape 108 (see 4): slots vertical, grid right of it; see gaps.

### 3.4 Filter menu (132, 133)
- Dropdown: 165x~140 (x 238-467 on 560-scale -> ~160x~190 px real), dark #24033B/near black fill, 1px lavender 45% border, radius ~16. Rows `All`, `Collectibles` height ~90 (full-res), selected row darker highlight + check (white) at right. In 132 it's right-aligned under the button (x~1740-1965 of the 2000-wide sheet = right side); in 133 it's left-aligned at x 238-467 under the Emotes pill. Inconsistent anchor (see 7). RN anchors right.
- State: active filter => red dot (12px, #FF2D55) on button top-right corner (134/135 only; 132/133 show none while open).

### 3.5 Face (122-126) and color editors (127-130)
- Camera: zoomed on head. Legacy per-category camera focus (head, head_base, head_base_facial, torso, hands, legs, feet) in `avatar_preview.gd:focus_camera_on`. Figma shows head zoom for all five face sub-tabs (legacy would use "head" for hair, "head_base_facial" for facial hair, "head_base" for eyes/eyebrows/mouth). Clothing screens (136-139) are full-body in Figma (legacy zooms to torso/legs/feet/hands): Figma differs.
- Sub-pills: `Hair`, `Facial hair`, `Eye`, `Eyebrows`, `Mouth` (note label `Facial hair` lowercase h; RN has `Facial Hair`). 
- Palettes: hair row (black, dark brown, brown, light brown, tan, red, ...), eye row (near-black, brown, tan, gold, grey, light blue, ...). Facial hair, hair and eyebrows share the hair palette (RN maps color 'hair' to all three; Figma: each has its own editor titled Hair color / Facial hair color / Eyebrows color; all three edit the same value in RN since `ColorTarget` is skin|hair|eyes; legacy has per-category hair/eyebrow/facial colors? see 5).
- Mouth: no swatch row; later rows dimmed (126), likely placeholder/skeleton while thumbnails load.

### 3.6 Clothing (136-139)
- Sub-pills `Chest`, `Legs`, `Feet`, `Hands` (RN matches). No swatch row. Grid starts ~y 880. Item names not shown; only thumbnails (and rarity gradient for collectibles). 139: only 4 items, rest empty (no empty-state message).

## 4. Components / tokens

Colors (match `rn/src/ds/tokens.ts`): ruby #FF2D55 (active pill/tab underline/EQUIP), orange #FF7439 (equipped border), shadow #161518 (strip, nav), lavender #E8B9FF (inactive tab/links), purple350 #691FA9, purple450 #460F6F, purple500 #350854, purple550 #24033B, cardLight #B48CD7 + snow #FCFCFC, rarity gradients in `rarityColors`.
Type: Inter. Tab 26px/700 uppercase; pill 22; card strip 22 bold uppercase; empty title 40 semi-bold; link 28 underline; sheet title 30; slider labels 16 caps.
Existing in `rn/src/ds`: Pill, IconButton, Tabs, SlotTile, ItemCard (+SkeletonCard), SwatchRow, HsvSliders, ColorPicker, BottomNav, GradientView, Icon, PrimaryButton. Not existing: ToastChip/NameChip, EmptyState (icon+title+link), text sub-pill row (SlotTile is thumbnail-based), vertical slot rail (landscape), CheckStrip/EquipStrip variants of ItemCard caption, notification dot.

## 5. Legacy counterpart (Godot)

`godot/src/ui/pages/backpack/`: `backpack.gd` (1267 lines; filters, equip/unequip, marketplace section, colors, new-item tags), `backpack_portrait.tscn`, `backpack_landscape.tscn`, `backpack_responsive.gd` (swaps layout by orientation), `avatar_preview.gd` (preview, drag/rotate, pinch zoom, per-category camera focus), `emote_name.gd/.tscn` (chip + "Equipped" toast, 0.3s slide tween), `no_items_message.gd/.tscn` (`BACKPACK_NOTHING_HERE_YET`), `backpack_button.gd`.
Behaviour the Figma does not show but legacy has:
- Collectible filter checkbox (`only collectibles`) and `_load_filtered_data`; unequip blocked for required categories (`_can_unequip`); body shape swap resets/keeps colors.
- Per-category camera auto-focus and pinch-zoom/pan in the preview (`can_drag`); avatar loading indicator (`_set_avatar_loading`) and change VFX (`avatar_change_vfx.png`).
- "NEW" tags on freshly arrived items and refresh on notifications (`_on_item_arrived`, `newtag_*`, owned-wearables refresh `_async_refresh_owned_wearables`); marketplace preview-equip (`_async_marketplace_preview_equip`) and the iOS marketplace-section gating (`_setup_ios_marketplace_section`, guest-upgrade check). The Marketplace link in the empty state must respect that gating (hide/disable on iOS for non-eligible accounts).
- Blacklisted wearables (`_on_blacklist_changed` + deploy timer); profile deploy debounced on changes; "back to explorer" button visibility.
- Emote equip callback `_on_emote_equipped`, emote names via `emote_name.gd`.

## 6. RN status and gaps (`rn/src/backpack`, portrait only)

Exists: BackpackScreen (portrait, Sheet with drag-collapse + 0.455 height ratio), header pills + filter button + FilterMenu (All/Collectibles), Tabs ALL/BODY/FACE/CLOTHING/EXTRAS, slots per tab (`taxonomy.ts`), ItemGrid/ItemCard with equip/unequip + rarity, ColorPanel with SwatchRow+HsvSliders and 120ms throttle, EmoteSlotsRow + EmotesGrid (base emotes only, slot targeting), ConnectionLost state, SkeletonCard loading, catalog = base-avatars lambda only.

Exact gaps vs seq 104-139:
1. Landscape (108): no layout (RN comment "portrait only"). Needs: left pills+filter, vertical 112px slot rail, 3-col grid panel x720-1440, right rail with profile avatar + friends/bell/compass/backpack/settings. Orientation is available from `GodotBridge.Orientation`.
2. Sub-slots are `SlotTile` thumbnail tiles in RN; Figma uses TEXT PILLS (Shape/Skin, Hair/Facial hair/Eye/Eyebrows/Mouth, Chest/Legs/Feet/Hands). Switch to `Pill`-style row; rename `Facial Hair` -> `Facial hair`.
3. Swatch row inline: Figma shows the swatch row (wheel + palette) directly above the grid for Shape?/Skin/Hair/Facial hair/Eye/Eyebrows; RN shows a single "Color" button row (ColorSwatchButton) that opens the panel. Need inline SwatchRow whose wheel opens ColorPanel; mouth/clothing none; Skin pill shows swatches only (no grid when no skin wearables).
4. Color editor titles: "Hair color", "Facial hair color", "Eyebrows color" each; RN has only `COLOR_TITLES` skin/hair/eyes and eyebrows/facial hair reuse `hair` (changing eyebrows color also changes hair/facial hair). Needs new targets in bridge (`AvatarColorTarget` = skin|hair|eyes) or confirm they share (open question). Editor header y~600, sliders labels "COLOR/SATURATION/BRIGHTNESS" present in ds.
5. Empty state: RN shows a bare caption "Nothing here yet"; Figma has icon (shirt for wearables, dancer for emotes) + "Find new ones in the Marketplace" with link; none for emotes.
6. Emote UI: name chip ("<Emote name>") and "Equipped ✓" toast absent; emote slot row lacks selected-name binding, white flash, and selected-empty state without chip; tile caption states only `Unequip` (need EQUIP pink strip for candidate, check strip for equipped-elsewhere, UNEQUIP only for selected slot's emote). Emote tap should select/preview first, equip second tap (Figma 114/120) - RN equips immediately and auto-advances slot.
7. Emote catalog: base emotes only; collectible/owned emotes (colored tiles, multi-avatar thumbnails, 119/134) require owned-emotes fetch and `collectible` flag; Collectibles filter on emotes returns nothing.
8. Filter: red active dot on button (134/135) missing (RN has `filterActive` styling only; verify dot). Menu anchor differs between 132 and 133.
9. Face camera zoom: no bridge command for camera focus (`GodotBridge` ClientMessage has none) -> need `focus_camera {category|'head'|'overall'}`; Figma = head zoom for all face pills, full body for clothing.
10. Skin: skin wearables list + swatches coexist (109-111); RN treats Skin as colour-only slot. Disabled swatch state (110) missing.
11. Mouth grid dim/skeleton rows (126) are probably loading; RN has SkeletonCard.
12. Sheet: Figma shows no drag handle visible and sheet top ~y690 (0.43 of 1600); RN ratio 0.455 + 28 handle; emotes mode sheet top ~813 (0.508) is pushed by the slot row, RN overlays the slot row separately.
13. Clothing tabs: avatar full-body in Figma vs legacy zoom; 139 has no empty-state.

## 7. Open questions / inconsistencies

- 110/111 use tab labels BODY | HEAD | ACCESSORIES | TORSO | LEGS (Skin case) instead of ALL|BODY|FACE|CLOTHING|EXTRAS: an old taxonomy variant; treat as stale design (use the 5-tab set), confirm with design.
- Swatch-row visibility on Skin: 109 none (empty), 110 dim, 111 normal. Define: swatches always shown when Skin pill active; dimmed only while loading.
- Filter menu anchored right (132) vs left (133); 117/131 header pills sit ~10px left of other frames (layout drift).
- "UNEQUIP" on body shape (104) contradicts legacy `_can_unequip` (required category).
- Emote tap semantics (select then EQUIP then equip, vs RN immediate equip); is `EQUIP` shown only when a slot is selected?
- Filter "Collectibles" meaning for emotes/wearables (rarity != base) and where the red dot comes from (new items? active filter?). Legacy red dot = new arrivals (`_on_new_notifications`); Figma 134/135 show it with Collectibles active.
- Hair vs facial hair vs eyebrows colors: shared or separate (bridge supports skin/hair/eyes only).
- Name for Mouth/Eyebrows/Eyes: label strings are `Eye` (singular) vs `Eyebrows`.
- Landscape frame 108 is named "Wearables" but shows Emotes; no landscape Wearables/color-editor frames in this range (check backpack-b).
