# discover-d report (manifest seq 86-103, source discover)

Scope: continuation of discover-c.md (seq 75-85). All 18 nodes were captured on disk and viewed (16 screens + 2 strips). No discover entries exist outside seq 13-103. get_design_context was NOT called: numbers below are measured off the 720x1600 screenshots (Figma px; fileKey Tz8ekjVNmv2OpkmfZb8KwU). Base tokens (gradient bg, gutters, cards, sheet, JUMP IN, bottom nav) are in discover-c.md section 3/4 and are not repeated.

## 1. Screen inventory

| seq | id (suffix) | name | what it shows | state/variant |
|---|---|---|---|---|
| 86 | 1582-20203 | Discover-Connection lost-02 | Back arrow + "Discover" title, dimmed search icon, centered offline illustration, "Connection lost" + "Please check your internet connection and try again." No bottom nav, no retry button | error (full-screen) |
| 87 | 1767-39473 | Discover-FeatureBanners | New top of Discover: feature-banner snap carousel (1st banner active, peeks both sides), dots (1st active), then "Friends 5" rail, "Events" rail, bottom nav | loaded, tab root, banner idx 0 |
| 88 | 1944-34968 | Discover-FeatureBanners-Detail | 87 dimmed/blurred behind a Place sheet ("Meta Residence Tower - Apartments in Decentral...", heart, by Name Creator, hero, stats, share/like/dislike/JUMP IN) | modal, place sheet over banners |
| 89 | 2005-21896 | Discover-FeatureBanners-Detail | Same sheet in skeleton form (bars for title x2 + creator, hero block, 3 stat blocks), action buttons already real | modal, loading sheet |
| 90 | 1952-21149 | Discover-FeatureBanners-Button | Pixel-identical to 87 (red arrow-in-box CTA on banner highlighted as the thing under spec) | loaded, same as 87 |
| 91 | 1783-41275 | Loading | The legacy Godot landscape loading screen ("LAND" tip, "LOADING 20%", "(10/76 resources at 0.00mb/s)", MOBILE ALPHA badge) pasted rotated 90 deg into a 720x1600 frame | world loading, landscape content shown sideways (see loading-hud.md) |
| 92 | 1693-20759 | Discover+Friends | Banner "Tower of Madness" (2nd of 3 dots) with a plain red "JUMP IN" button instead of arrow icon; Friends 5 rail; "Top mobile" rail with black placeholder tiles | loaded; banner variant B; placeholder rail |
| 93 | 1693-21242 | Discover-Story | Full-bleed vertical story/video of a place (avatar in scene), back button top-left, title "Antrom RPG (Dice Masters) SDK7", "by Lazaro", full-width JUMP IN, bottom nav still visible | story viewer |
| 94 | 1700-23498 | Discover-Story | Same, place "Rage Parkour" by "Creator Name" (mouse cursor visible = recorded desktop clip) | story viewer, variant (short title) |
| 95 | 2107-25669 | Discover+Categories01 | Banner (idx 0), category chips row (SOCIAL, MUSIC, GAME, ART, FA...), "Popular" rail, "Events" rail | loaded |
| 96 | 2110-26055 | Category | "Game" category page: back arrow, title "Game", search icon, 2-column grid of place cards, bottom nav | list/grid, scrolled partially |
| 97 | 2159-27663 | Category-Search | Same grid but header replaced by back arrow + search field ("Search"), OS keyboard open | search active, empty query |
| 98 | 2123-26835 | Discover-all carousels (720x3368) | Full scroll of Discover: banner, Jump back in, Popular, Friends online, New in Decentraland, categories, Events, Favorites, My Places | loaded, full length |
| 99 | 2190-22565 | Discover+Categories01 | Halloween-themed Discover: bats art, dark-purple bg, "HALLOWEEKENDS" banner, Jump back in (Genesis Plaza, WonderMine), Popular, Friends online | loaded, seasonal theme |
| 100 | 2214-22767 | Discover+Background | Seasonal background layer alone: dark purple vignette radial gradient with 4 bats | asset (full-screen bg) |
| 101 | 2214-23123 | Discover+Overlay | Same layer in near-black greys with the same bats (a dark overlay variant) | asset (overlay/dim) |
| 102 | 1783-41282 | Banner Feature (strip 7727x130) | Solid mint-green strip (#32E3B9-ish), no content, 1px lighter border | annotation swatch, no UI |
| 103 | 2130-27268 | "." (strip 7727x130) | Solid amber strip (#FFC85C-ish), no content | annotation swatch, no UI |

Note: 90 and 87 are identical; 88/89 differ only in sheet loading state. Strips 102/103 are empty color bars (likely Figma section labels/annotation bars, the "Banner Feature" one is green); nothing to implement.

## 2. Flow graph

- Bottom-nav compass -> Discover root (87/95/98). Search icon -> search mode (97 shows it only for a category; the root search is the existing one). Back arrow on root variant only appears when pushed from the world (discover-c).
- Banner tap (87) -> opens the featured place sheet (88, shows skeleton 89 first, then loaded). Red arrow-in-box on the banner (87, text "Experience Name 2 lines example elipsis name") = open sheet/jump affordance. Banner variant B (92) has a explicit "JUMP IN" button directly on the banner: either teleports straight in or opens the sheet (open question).
- Swipe banner horizontally: snap carousel, dots below indicate position (3 dots visible: active = light lilac, inactive = dark purple). Neighbours peek 16px at each edge (card 48..672 = 624 wide, gap 16).
- Friends avatar (87/92/98) -> friend sheet (84, discover-c).
- Category chip (95/98) -> Category page (96: title = category name, e.g. "Game"). Back arrow -> Discover. Search icon (96) -> category-scoped search (97, field replaces title, keyboard opens, grid filters). Back in 97 closes the search (returns to 96) -- assumed.
- Place card in grid -> place sheet (82). 
- Story (93/94): entry not shown (probably a tile/banner long-press or "stories" rail); back button top-left -> Discover; JUMP IN -> world. Bottom nav remains.
- Connection lost (86) <- any rail failing on first load (85 -> 86). Recovery path not designed (no retry button); legacy/RN ConnectionLost pattern (rn/src/backpack/ConnectionLost.tsx) should reuse and add a retry.
- Seasonal theme (99-101): background layers swapped by remote config / date.

## 3. Per-screen spec (720x1600 portrait, Figma px; unchanged tokens in discover-c.md)

### 3.1 Feature banners carousel (87, 90, 92, 95, 99)
- Position: top of scroll content below header. Header title "Discover" at y~123, search icon x~643,y~121 (here the Discover root has no back arrow).
- Card: x 48..672 (624 wide), y 215..563 (348 tall, aspect ~1.79), radius ~12, 1px light border (white ~30%). Left and right neighbours peek (x 0..32 and 688..720, 24px lower offset: neighbours are vertically inset ~24px, i.e. centred-card scale effect, y 240..538).
- Image fills card. Bottom gradient scrim (black -> transparent, ~45% height) for text legibility.
- Title: bottom-left at x=80,y~488 (padding 32), semibold ~26px white, max 2 lines, ellipsis ("Experience Name 2 lines example elipsis name"). Right of it, CTA square 56x56 at x 584..640, y 476..532, radius ~8, fill coral #FF2D55, white arrow-in-box icon (same as JUMP IN icon).
- Variant 92: no CTA square; instead a wide red button "JUMP IN" (x 441..655, y 463..511, 214x48, radius 8, bold ~20px uppercase white), banner has no title (title is baked into the artwork "TOWER OF MADNESS"). Card in 92 is y 192..541 (starts 23px higher than 87 because the header gap differs: inconsistent).
- Dots: centered at x=336/360/384 (24px pitch), y=581/586, 10px dots; active #E4C1F9-ish light lilac, inactive #3a0a5e dark. Dot pitch/colour from screenshot only.
- Data: needs a "feature banners" source. Not in the public places API; likely a backend/remote-config list ({image, title, target: place|world|event|url, cta}). Legacy: `godot/src/ui/components/molecules/snap_carousel/snap_carousel.gd` + `snap_carousel_card.gd`, used by `%PlacesFeatured` in discover.gd (cards = highlighted places, `card_tapped` -> `_on_featured_card_tapped`). So the existing source is Places `highlighted` (RN already loads "Featured" from `only_highlighted=true`); the Figma banner is the new visual for the same Featured rail with the title overlay and CTA.
- Behaviour: snap-to-card paging, optional auto-advance (legacy snap_carousel; confirm), dot sync with scroll, loading state = skeleton block 624x348 (not drawn).

### 3.2 Banner detail sheet (88, 89)
- Same as discover-c place sheet (82). 88 identical copy ("Meta Residence Tower - Apartments in Decentral...", "by Name Creator", stats "000" / "100%" / "-000,-000", buttons share / thumb up / thumb down / JUMP IN with arrow icon). Sheet top y=678, handle 200x8 at y~695, hero 624x366 at y 927..1293, stats row y~1347 with 1px dividers at x=203 and x=424, button row y 1418..1513 (95 tall): three 94x94 squares (x 48, 156, 264; fill #43106f-ish) + JUMP IN x 376..672.
- 89 (skeleton): title bars 624x34 (y 755), 624x34 (y 807) and 229x31 (y 862), hero block 624x366 (fill ~#451d62), three stat blocks 120x40 at x 48 / 299 / 550 (y 1326); action buttons rendered at full opacity. Skeleton fill rgba white ~8%, radius 4 (bars) / 12 (hero). Backdrop blur is stronger and darker than 88 (heavier dim).
- Sheet opens with skeleton while place details/votes load.

### 3.3 Friends rail (87, 92, 98)
- Section title "Friends 5" (title white semibold ~32px at x=48, count in lilac #D79BFF-ish). 98 calls it "Friends online" without count; 84 sheet calls it "Online Friends". Three names for one section.
- Item column ~244px (87: avatar 150, x 84..232; 92/98: avatar 160, x 96..256, column ~278 wide: inconsistency). Avatar circle with 4px white ring, pastel fill (pink #F1C7F6, yellow #FFF18A, lime #C8F95B...).
- Name below avatar: bold ~26px white, centered, max 1 line ("Username", "User p name"). Scene name below: regular ~22px 70% white, centered, max 2 lines (87 uses 1 line ellipsis "Experience Name..."; 92/98 show 2 lines "Experience Name max character"). Third item peeks at right edge.
- Data: friends service (online friends) + their current place/realm (comms/Rust bridge). Hidden when no friends online.

### 3.4 Categories chips (95, 98, 99 label)
- Horizontal scroll row, y 652..800 in 95. Each chip: tile 120x120 at x 48, 206, 364, 522, 680 (gap 38), radius ~14, violet vertical gradient (#C03FD0 top -> #7B2BB0 bottom) with 1px light border, 3D emoji illustration centred ~84px (SOCIAL: two avatars, MUSIC: notes, GAME: gamepad, ART: palette, FA(shion?): cut off). Label below, uppercase semibold ~20px white centered ("SOCIAL", "MUSIC", "GAME", "ART", "FA..."). No section title. 5th chip peeks.
- Position varies: after banners (95: banner -> chips -> Popular -> Events) vs after "New in Decentraland" and before Events (98). Inconsistent; choose 98 (full-length is the most complete composition) unless design says otherwise.
- Tap -> Category page (3.5). Category ids: legacy places API categories (social, music, game, art, fashion, ...). Legacy: `categories/categories_bar.gd` (HFlowContainer of `category_tag.tscn`, uses `set_categories([String])`, no icons).

### 3.5 Category page (96) and Category search (97)
- Header (96): chevron x~75,y~114; title "Game" bold ~44px at x=117,y~116; search icon x~649.
- Grid: 2 columns, same PlaceCard as rails: image 294x155 (x 48 and 378, so col gap 36), title semibold ~26px (single line ellipsis "Casai..." / "Casa Rousto"), stats row (person "000", thumb "100%"). Row pitch 291px (y 200, 491, 782, 1073, 1364). Infinite scroll (pagination via places API `offset/limit`; `categories=<id>`). Bottom nav visible (compass active).
- 97: header replaced by chevron x~75 + search field x 133..672, y 88..156 (68 tall, radius ~10, dark violet fill #2d0650 translucent), magnifier icon x~172, placeholder "Search" (~28px, 50% white). Keyboard is Android Gboard "Done" return key, grid is dimmed? no: it stays visible above keyboard. Bottom nav is hidden under keyboard.
- States: loading (skeleton grid not designed: reuse place skeleton 294x155 + 2 bars), empty ("no results" copy not designed), end of list, search no-results.
- Data: places API `GET /api/places?categories=<c>&order_by=...` and search param `search=<q>`; title = category display name (localised).

### 3.6 Story viewer (93, 94)
- Full-bleed video/animated-image of the place (portrait 720x1420 above the nav), back button top-left: 60x60 at x 48..108, y 86..146, translucent black ~50% radius 8, white chevron. 
- Bottom overlay: dark gradient scrim from y~1040. Title bold ~42px white at x=48, max 2 lines ("Antrom RPG (Dice Masters) SDK7"), then "by" (lilac #D7A8FF, regular) + creator name (white semibold ~28px) at y~1243, then JUMP IN full-width (x 48..672, y 1300..1395, 95 tall, radius 12, coral #FF2D55, text bold ~28px white, no arrow icon, centred). Bottom nav shown beneath, compass active.
- Capture shows an actual in-world recording (avatar "Didot" with name tag, cursor visible), so the asset is a video/GIF per place (not a screenshot): needs a source (places API has `image` only; a `story`/video asset or capture of the scene is new back-end work).
- Entry point and swipe-to-next-story gesture are NOT shown (TikTok-like vertical paging is the likely design). Mark as open question; deferred behind a feature flag in RN.

### 3.7 Rails seen only in 98/99 (full-length Discover)
Order top to bottom (98, 720x3368): header -> feature banners (+dots) -> "Jump back in" -> "Popular" -> "Friends online" -> "New in Decentraland" -> categories chips -> "Events" -> "Favorites" -> "My Places".
- "Jump back in" (renames "Last Played"): horizontal list of compact rows, thumbnail 120x120 (x 48..168, y 692..812, radius 8) + title right at x=188, semibold ~24px white, 2 lines max with ellipsis ("WonderMine Crafting Game DC..."), vertically centered. In 99 a second item starts at x 460 (items ~412 wide, horizontal scroll). Different from the PlaceCard rail.
- "Popular" (renames Most Active): PlaceCards (294x155 images; title "Butterfly Prawn Farmi..." etc., x 48, 374, third peeks at 700; gap 32).
- "New in Decentraland": PlaceCards, 2 items, new data source (places created/deployed in the last N days: `order_by=created_at`).
- "Favorites", "My Places": PlaceCards (already in discover-c seq 75). My Places sample title "My place name".
- "Events": as in discover-c (406x240 event cards, LIVE / attendee pill / Trending / IN 5 MINS, title + "by <creator>"; here "by" in lilac and creator in white semibold).
- Section titles: semibold ~30px, colour slightly dimmer than the Discover title (#F0E0FF-ish). Section pitch: title y -> rail 40px; sections separated by ~60px.
- 'Featured' no longer has its own title: it is the banner carousel.

### 3.8 Connection lost (86)
- Layout: header with chevron (x~75) + title "Discover" (x=117), search icon dimmed (opacity ~40%) and presumably disabled.
- Centre block (vertically ~ y 560..900): rounded square 140x140 (x 291..431, y 562..700, 3px stroke, radius ~14, colour #C9A6DB ~70% white) containing a crossed-out wifi icon and a small "--  -----" dash base; title "Connection lost" bold ~44px (#D9B8E8), body "Please check your internet connection and try again." regular ~32px, two lines, centered, 500px max width, colour #CDB0DA.
- No CTA. In RN add pull-to-refresh or auto-retry on connectivity regained. Trigger: first-load failure of all/primary rails or no network.
- Legacy: `discover.gd` `_update_global_messages` (global loading/error text across carousels). RN `ConnectionLost` component exists in backpack and can be reused.

### 3.9 Seasonal theme (99, 100, 101)
- 100: full-screen background asset 720x1600: radial/vertical vignette dark violet (#3F1070 centre -> #14062A edges) with 4 bat sprites at (x,y,size): (345,100,150w), (645,640,150w), (240,845,95w), (540,1225,120w). 99 overlays the content on it (header and chips sit above the bats; bats are behind cards, partly translucent over them).
- 101: same sprites on a near-black grey radial (#1F1F1F centre -> #0A0A0A). Probably the "dim" or dark-mode variant of the seasonal bg; where it is used is not shown.
- In 99 the bottom nav stays solid #151515, header/search unchanged, banner = "HALLOWEEKENDS", Jump back in thumb = Genesis Plaza halloween art.
- Implementation: the Discover gradient background becomes a themeable layer (image or gradient) from remote config; default = existing violet gradient (discover-c). Date-driven? Open question.

## 4. Components and tokens
New vs discover-c: FeatureBanner (card + scrim + title + CTA square or JUMP IN button), PageDots, CategoryChip (tile + label), CategoryHeader (back + title + search) and SearchField (inline), PlaceGrid (2 col), SkeletonSheet, CompactPlaceRow (Jump back in), StoryViewer, ConnectionLost full-screen (illustration variant), SeasonalBackground.
Colours seen: coral #FF2D55 (CTA), violet button fill #43106F, lilac #D79BFF (counts/"by"), bg gradient (discover-c), nav #151515, chip gradient #C03FD0 -> #7B2BB0, dim text rgba(255,255,255,0.7).
Existing in rn/src/ds: AppText, BottomNav, IconButton, Icon, PrimaryButton, Pill, Gradient, ItemCard, tokens, scale. No dots, chip, search field, banner, or skeleton components exist yet. New icons needed: wifi-off, arrow-in-box (JUMP IN already has one in DetailSheet?), emoji PNG assets for categories (social/music/game/art/fashion...).

## 5. Legacy counterparts
- Banner: `godot/src/ui/components/molecules/snap_carousel/snap_carousel.gd` and `snap_carousel_card.gd`; used as `%PlacesFeatured` in `godot/src/ui/pages/discover/discover.gd` (tap -> `_on_featured_card_tapped`; featured items are also exported for the debug inspector). `discover_banner.png` asset in the discover folder.
- Categories: `godot/src/ui/pages/discover/categories/categories_bar.gd/category_tag.gd` (text tags only, no icons, no category page).
- Search: `search_bar/` and `search_sugestions_container.tscn`; search history saved with `Global.get_config().add_search_history` (discover.gd:442). Figma 97 has no suggestions list.
- Friends: `friends/friend_discover_card.gd`, `friend_discover_skeleton.tscn`.
- Skeletons: `carrousel/discover_carrousel_skeleton.gdshader`, `place_discover_skeleton.tscn`.
- Story: no legacy counterpart (grep for "story" in discover finds only unrelated hits). New feature.
- Jump back in / New in Decentraland / Popular: legacy carousels in discover.gd (Last Played / Most Active / Newest not present as such), Figma renames sections.
- Loading screen (91): `godot/src/ui/...` loading scene covered by loading-hud.md; not Discover.

## 6. RN status and gaps
Exists (rn/src/discover: DiscoverScreen.tsx 208 lines, Cards.tsx, DetailSheet.tsx, api.ts, useDiscover.ts, history.ts, format.ts): rails Featured / Events / Last Played / Most Active as simple lists, search that filters loaded lists, detail sheet, history; section titles are the old ones ("Featured", "Last Played", "Most Active"). `ConnectionLost` lives in rn/src/backpack (reusable).
Gaps for seq 86-103:
1. Feature banner carousel (snap, dots, title scrim, CTA variants), replacing the "Featured" rail.
2. Section renames: Last Played -> "Jump back in" (compact row style), Most Active -> "Popular", add "New in Decentraland", "Friends online"/"Friends N", Favorites, My Places.
3. Categories chips row + Category page (grid, infinite scroll) + Category search (inline field, keyboard handling).
4. Sheet skeleton state (89) and per-rail skeletons.
5. Full-screen Connection lost for Discover (illustration, dimmed search) with retry.
6. Seasonal background/theme layer.
7. Story viewer (new; needs video asset source and entry point).
8. Bottom nav present on Discover/category/story; keyboard hides it.
9. Banner data source (remote config or highlighted places) and category definition list.

## 7. Open questions / inconsistencies
- Banner CTA: red arrow-in-box square (87/90/95/98/99) vs full "JUMP IN" text button (92). Does a banner tap open the sheet (88) or jump directly? Is 92 the "banner of an event/world" variant?
- Banner card y differs (215 in 87/95/98 vs 192 in 92) and friend avatar size differs (150 vs 160); standardize on 87/95/98 values.
- Friends section named "Friends 5", "Friends online", "Online Friends".
- Category chip row position differs between 95 and 98. Category list and icon set (only SOCIAL/MUSIC/GAME/ART and a truncated FA... visible) unknown: need the full list and 3D icon assets.
- 97: back from search - clear field or leave category? Search suggestions/history not designed. Empty/no-results copy missing.
- Story: entry point, source of media, swipe paging, mute/progress UI, back behaviour all unspecified. 93/94 are recorded desktop clips (cursor visible) so real assets are not available.
- 86: no retry CTA; confirm auto-retry or pull-to-refresh. 85 (loading) vs 86 differ only in body.
- 100/101: where the grey overlay variant is used; whether seasons are remote-configured; asset resolution for 720x1600 only (needs tall-phone coverage).
- 91 "Loading" in the discover page is a landscape loading screen pasted rotated (portrait frame), likely an accidental paste: treat as loading-hud.md content; confirm it is not meant to be a portrait loading screen.
- 102/103 are empty colour swatches (no UI); 90 duplicates 87.
- "Top mobile" rail in 92 shows black placeholder tiles (270x~380 tall portrait tiles x 48, 358, 668; gap 32): an unfinished rail (portrait cards?) - confirm whether it ships.
