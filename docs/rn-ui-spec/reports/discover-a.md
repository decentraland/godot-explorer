# discover-a report (manifest seq 13-43, source discover)

STATUS: COMPLETE for visual analysis, 31/31 screenshots read. `get_design_context` was NOT called (its tool contract requires loading the figma-design-to-code skill first, which is not available in this agent), so there are no Figma-extracted tokens. Numbers below are read off the 720x1600 screenshots (Figma px; RN dp = px / 2 via `u()`), and tokens already in `rn/src/ds/tokens.ts` are cross-referenced. Context: discover-c.md covers seq 75-103 (same Discover list, sheets for events/places/friends); this report covers FTUE, the Discover list's early variants, and the Place / World / Event detail sheets in all their states.

## 1. Screen inventory

| seq | id (node) | name | what it shows | state/variant |
|---|---|---|---|---|
| 13 | 516:9558 | Discover-FTUE | Welcome "@UserName start here", single Genesis Plaza card (330x265 image, title, "by Decentraland Foundation"), JUMP IN, "Skip and explore" link | FTUE v1, non-carousel (static card) |
| 14 | 1710:23643 | Discover-FTUE | Same copy, snap carousel: centre card Genesis Plaza (360x287), neighbours peek left/right; button, "Skip" link | FTUE carousel, card 1 selected |
| 15 | 1712:23706 | Discover-FTUE | Same, centre card "Tower of madness" (neighbours peek) | FTUE carousel, card 2 selected |
| 16 | 133:5392 | Discover | Tab-root Discover: title, search, Featured / Events / Last Played / Most Active (partial), bottom nav | loaded, tab root, nav (compass, backpack, gear, avatar) |
| 17 | 566:18371 | Discover-Landscape | Same content, back chevron left of title, no bottom nav | loaded, pushed (from in-world), no nav |
| 18 | 538:11191 | Discover | Same as 16 (nav icon x positions differ slightly: 18 has avatar thumbnail ringed, compass active lilac) | loaded, tab root; near-duplicate of 16 |
| 19 | 168:5039 | 02-Detail-Place | Place bottom sheet collapsed over blurred Discover: title (2 lines), heart, "by Name Creator", hero image, stats row, 4 buttons | sheet, collapsed (peek), place |
| 20 | 168:10160 | 03-Detail-Place-More | Place sheet expanded full-screen (X close at top-left): + DESCRIPTION, links, category chips ART/BUSINESS/SOCIAL | expanded, outline heart, thumbs unselected |
| 21 | 313:2460 | 04-Detail-Place-More-Fav | Expanded, heart filled red | expanded + favorited |
| 22 | 319:3148 | 05-Detail-Place-More-Share | Expanded dimmed + Android system share sheet ("Sharing text") | expanded + OS share sheet |
| 23 | 319:6130 | 06-Detail-Place-More-Like | Expanded, thumb-up selected (filled glyph, lighter tile) | expanded + liked |
| 24 | 319:6640 | 07-Detail-Place-More-Dislike | Expanded, thumb-down selected | expanded + disliked |
| 25 | 319:8950 | Loading | Landscape loading screen (720x1600 frame, rotated content): "LAND" tip text, progress "LOADING 20%" "(10/76 resources at 0.0mb/s)", Decentraland MOBILE ALPHA logo | scene loading after JUMP IN (see loading-hud.md) |
| 26 | 1520:24684 | 02-Detail-Place | Collapsed place sheet, bottom row = location pin "-000,-000" left, thumb "100%" right (no online count) | collapsed, place v2 (Genesis-style: location + likes only) |
| 27 | 1520:25203 | 02-Detail-world | Collapsed sheet for a world: globe icon + world name "Mmmmmmmmmmmmmmmmm" left, "100%" right | collapsed, world |
| 28 | 1376:24108 | 03-Detail-Place-More | Expanded place sheet (X, heart outline, 3-stat row, description, chips) | expanded, same as 20 (hidden variant) |
| 29 | 1376:24621 | 03-Detail-Place-More | Expanded, thumb-up filled | expanded + liked (dup of 23) |
| 30 | 1376:25135 | 03-Detail-Place-More | Expanded, thumb-down filled | expanded + disliked (dup of 24) |
| 31 | 319:4672 | 04-Detail-Place-Fav | Collapsed, heart filled red | collapsed + favorited |
| 32 | 319:3669 | 05-Detail-Place-Share | Collapsed dimmed + OS share sheet (722x1606 frame, 1px offset) | collapsed + OS share |
| 33 | 319:5158 | 06-Detail-Place-Like | Collapsed, thumb-up selected | collapsed + liked |
| 34 | 319:5643 | 07-Detail-Place-Dislike | Collapsed, thumb-down selected | collapsed + disliked |
| 35 | 1521:28149 | 02-Detail-Place | Collapsed, row with only pin "-000,-000" (no stats) | collapsed, place without votes/online (e.g. no data) |
| 36 | 1521:27686 | 02-Detail-world | Collapsed, row with only globe + world name | collapsed, world without stats |
| 37 | 376:6791 | Discover-01 | Discover list, bottom nav; here nav avatar is thumbnail and the compass is active lilac | loaded, tab root; = 16 |
| 38 | 376:5429 | 02-Detail-Event | Collapsed event sheet: emoji + "Chiri Builds", "by Lazaro", hero with LIVE / green dot + 000 / Trending, meta row Weekly / 1 Hr / -000,-000, buttons share, calendar, JUMP IN | collapsed, event, live |
| 39 | 376:7236 | 03-Detail-Event-More | Expanded event (X): long title "Meta Residence Tower - Apartments in Decentraland" (placeholder), DESCRIPTION (emoji text), buttons | expanded, event |
| 40 | 461:11695 | 04-Detail-Event-More-Calendar | Expanded, dimmed + Android "Complete action using" chooser (Calendar, Outlook; Just once / Always) | expanded + OS calendar intent chooser |
| 41 | 1019:22562 | 02-Detail-Event-World | Collapsed event whose location is a world: globe + "Aaaaaaaa..." (ellipsized) replaces pin coords | collapsed, event in world |
| 42 | 805:21685 | Detail Event-01 | Expanded event scrolled to bottom (720x1224): title, hero, meta, DESCRIPTION (full long text), divider, schedule line "Mon, Jan 19. 04:10pm to 06:10pm (UTC-3)" + small calendar icon button at right, buttons row cut off | expanded, scrolled end, shows start/end line |
| 43 | 1520:27182 | 02-Detail-Event | Collapsed event, meta row only "Weekly" and "1 Hr" (no location) | collapsed, event with no location |

Duplicates / near-duplicates: 16 = 18 = 37 (nav variations); 20 = 28; 23 = 29; 24 = 30. 14/15 differ only by the selected card.

## 2. Flow graph

- Entry points: (a) first run after avatar creation: lobby -> FTUE (13-15) -> JUMP IN -> loading (25) -> world; Skip -> Discover tab root (16). (b) Bottom-nav compass -> Discover root (16/18/37). (c) In-world compass HUD pill -> Discover with back chevron, no nav (17). (d) Deep links `events?id=` / `places?id=` and notification taps open the matching sheet over Discover (not drawn).
- Back: 17 chevron returns to world HUD; 16 root has no back (tab switch). Sheets: collapsed sheet dismisses by dragging the handle down or tapping the blurred backdrop; drag the handle up (or tap the title area, inferred) -> expanded; expanded shows X (top-left) that closes the sheet (back to Discover, not to collapsed). The Android back gesture = same as X.
- Discover card tap: place card -> place sheet (19/26/27/35/36); event card -> event sheet (38/41/43). World places -> variant 27/36 (globe + name instead of coordinates).
- Sheet actions: heart toggles favorite (31 filled); share -> OS share sheet (22/32; text is "Visit the event ... following this link https:..." for events, a jump link); thumb-up/down toggle vote (33/34; mutually exclusive); calendar (events) -> OS calendar intent, Android chooser 40; JUMP IN -> close sheet, teleport/join world, show loading screen 25 (landscape), then world.
- FTUE: snap carousel swipe (14<->15), JUMP IN jumps to centre card's place, Skip / "Skip and explore" -> Discover root. Only the first FTUE (13) says "Skip and explore"; 14/15 say "Skip" (inconsistency).
- Loading 25 -> world; cancel (legacy: X) returns to Discover (see loading-hud.md).

## 3. Per-screen spec (portrait 720x1600, Figma px)

Common background: vertical-to-diagonal violet gradient, approx #7a22b0 top-left / #4c0a7a middle / #7b1595 bottom-right (see `ds/Gradient.tsx`; token `purple350 #691FA9`, `purple450 #460F6F`). Font Inter; section titles semibold ~32; body ~26-30.

### 3.1 FTUE (13, 14, 15)
- Layout 13: full-bleed gradient; headline block centred at y~300-440: "Welcome" (white bold ~46px), "@UserName" (lavender-ish #bfa6ff, bold ~46), "start here" (white). Card: image 660x530 (x=50-380 in 0.6 sheet => 550x440 real; radius ~12, 1px light border) at y~450; title "Genesis Plaza" 40px semibold centre; "by" regular + "Decentraland Foundation" semibold 26px 80% white. JUMP IN: full width 550x95 coral #FF2D55 (`colors.ruby`), radius ~10, white bold 30px uppercase "JUMP IN" + arrow-in-box glyph. Below: "Skip and explore" underlined white 70%, 28px, centred.
- 14/15 (carousel): headline sits at y~260-400 (lower line spacing), the card is wider, 600x480 at centre with ~30px of neighbour cards peeking on both sides (neighbours dimmed, cut by screen edge), snap scrolling; JUMP IN ~550x95 at y~1210; "Skip" underlined (shorter label).
- Interactions: horizontal snap swipe, pagination none visible; centre card name/creator update; JUMP IN uses centre place coords/world; "@UserName" = the profile name (with `#tag` omitted, claimed or not).
- Data: places from places API (legacy FTUE_PLACE_ID Genesis Plaza + featured list; `load_places()`), username from profile, jump via bridge `teleport`/`enter_world`.
- States: loading (not drawn; legacy shows skeleton), loaded, error (not drawn; fall back to Jump into Genesis Plaza).

### 3.2 Discover list (16/18/37 root; 17 pushed)
Already specified in discover-c.md section 3. Added detail from these shots:
- Root header: title "Discover" bold 46px white at x=48, y~123 baseline; search outline icon 44px at x=645; pushed header: chevron-left ~34px at x=75 plus title at x=117, y~116.
- Rails shown: Featured (place cards 294x155, gap ~32, third peeks), Events (event cards 406x240, "LIVE" red pill + dark pill "green dot, person, 000" top-left, green "Trending" pill top-right; second event card has white pill "IN 5 MINS" in red text; title ellipsized after 1-2 lines "Experience Name two lines...", "by Lazaro"/"by CryptoM1"), Last Played (WonderMine Crafting..., ". dejay Bar & Restauran"), Most Active (clipped). Place stats: person icon + "000", thumb-up outline + "100%".
- 16 vs 17 vs 18: identical lists; the only differences are header and nav. In 17 (no nav) the list runs to the screen bottom (safe area).
- Bottom nav (16/18/37): black bar 180px (`layout.navHeight` u(77)=154dp? Figma 180px = 90dp, check), 4 icons: compass (active = lilac filled disc ~52px), backpack, settings gear, avatar thumbnail ~72px circle. Compass active only on Discover.

### 3.3 Place detail sheet (19-24, 26-36), the most variant-rich component
Collapsed ("peek"), 720x~1195 visible: sheet top y~680 (radius ~28, fill gradient #4a0f78 -> #350854 = `purple450`/`purple500`), handle 200x8 pill #9d6fc7 centred at y~694. Backdrop = blurred+darkened Discover (Gaussian ~12, overlay ~rgba(24,3,48,0.5)) incl. dimmed title and search icon (not interactive).
- Content (x padding 48): title 2-line max semibold ~42px white, ellipsis "Meta Residence Tower - Apartments in Decentral..." at y~760-830; heart icon 52px at right (x~1265/0.6=~636, y~790): outline white, filled #FF2D55 when favorited (21, 31). "by Name Creator": "by" regular 70% + creator semibold ~26px at y~877.
- Hero image 624x368 (16:9-ish, radius 16, 1px light border) at y~925-1292.
- Stats row y~1345 (Place, 3 stats): person "000" | thumb "100%" | pin "-000,-000" with 1px vertical dividers (~40px tall, 30% white), each cell centred in thirds, 30px regular.
  - Variant v2 (26/35): only location pin + coords left aligned, thumb-up "100%" right-aligned (26), or only pin (35).
  - World (27/36): globe icon + world name (ellipsized ~16 chars) left, "100%" right (27) or name only (36).
- Buttons y~1420-1515, 95px tall, radius ~12, gap 12: three 94x94 squares (share, thumb-up, thumb-down; fill `purple350`-ish #5b1b95, white outline icons 44px) and JUMP IN flex (~410 wide, #FF2D55, bold 30px "JUMP IN" + white rounded square with red arrow). Selected thumb: solid glyph and lighter tile (#a06ad6 at ~60%); like and dislike are mutually exclusive.
Expanded (20/21/23/24/28-30): full-screen sheet (no radius), X close icon 44px at x=48,y~115; the title/heart/by row moves to y~245-355; hero 624x368 at y~400 (full size); stats row (always the 3-stat form here) at y~820; divider; label "DESCRIPTION" (caps, 22px, 60% white, letter-spaced) at y~935; description 26px regular 90% white, lines 34px high, links (discord, twitter) in periwinkle #9d8cff, not underlined; divider; category chips (pill 110-130px wide, 38px tall, 1px 40% white outline or dark fill, icon + caps text 20px): ART, BUSINESS, SOCIAL (icons: palette, building/bookmark, people); buttons row fixed at the bottom y~1420 (same as collapsed). Description region scrolls between the stats row and the button row (the header stays? Not specified: in 42 everything scrolls except the button row).
- Share (22/32): native OS share sheet (Android "Sharing text" on screen), message "Visit the event <emoji> <title> at <creator>' following this link https:..." (the example text is the events template; place text unspecified). Use RN `Share.share`.
- States: collapsed, expanded, favorited, liked, disliked, share-open. Missing from designs: loading skeleton for sheet, vote-failed toast, favorite requires login (guest), disabled votes for guests, download-size warning strip (appears in discover-c seq 81/82 but NOT in this set; confirm whether it was dropped).

### 3.4 Event detail sheet (38-43)
- Collapsed (38/41/43): sheet top y~730 (lower than place because no heart; handle y~750). Title row: emoji icon (hammer "🛠") + "Chiri Builds" bold 40px, "by Lazaro" (semibold creator). Hero 624x368 with same LIVE / attendees dark pill / Trending pill overlays (positions top-left x=24,y=24 and top-right) as the card. Meta row y~1345: calendar icon "Weekly" (recurrence), clock icon "1 Hr" (duration), pin "-000,-000" (or globe + world name "Aaaaaaaa..." in 41; row of two cells only in 43 when location absent). Dividers between cells. Buttons: share 94 square, calendar 94 square, JUMP IN (wider ~600-94*2-... i.e. flex ~500).
- Expanded (39/42): X, title (no emoji, plain "Meta Residence Tower - Apartments in Decentraland" - placeholder copy; real: event name; full, 2 lines, no ellipsis), by-line, hero, meta row (3 cells), divider, DESCRIPTION label + long text (emojis inline, line breaks preserved), divider, schedule row (42): "Mon, Jan 19. 04:10pm to 06:10pm (UTC-3)" 24px at left + small calendar icon button (72x72 square, violet) at right. Share / calendar / JUMP IN buttons fixed at the bottom (cut off in 42's shorter frame).
- Calendar (40): taps the calendar button -> Android chooser (Calendar / Outlook), i.e. an ACTION_INSERT calendar intent with title, description, start/end, and a jump link. No in-app reminder toggle is shown (legacy had `button_reminder.gd` with `PATCH /attendees` + local notification). Open question below.
- JUMP IN for an event: teleports to event coordinates or world; not live yet (upcoming): unspecified (legacy allowed jump anyway).
- Data: events API item: name, user_name, image, live, start_at/finish_at, recurrent + recurrent_dates, duration, x/y or server (world), total_attendees, description, estate_name, trending, url.

### 3.5 Loading (25)
- 720x1600 frame, content rotated 90 (it is the landscape loading screen drawn on a portrait artboard): vertical-reading tip "LAND" title + paragraph text ("Decentraland is made up of over 90,000 LANDs: virtual spaces backed by cryptographic tokens. Only LANDOwners can determine the content that sits on their LAND."), tip navigation chevrons up/down, abstract 3D art, bottom "LOADING 20%" and "(10/76 resources at 0.0mb/s)", top-right "Decentraland MOBILE ALPHA" logo strip. Full spec lives in loading-hud.md; here only the entry from JUMP IN. Note the artboard is rotated: implement in landscape, not as a portrait screen.

## 4. Reusable components / tokens
- Seen: gradient background; section header (semibold 32 white); PlaceCard (img 294x155 + title + stat row); EventCard (406x240 + badges LIVE / attendee pill / Trending / "IN 5 MINS"); bottom sheet (handle + blur backdrop + collapsed/expanded snap points); square icon button 94x94 (share, calendar, like, dislike; selected state); primary coral JUMP IN with arrow-in-box; stat cell with dividers; category chip (icon + caps); description label; icon-link text; heart toggle; FTUE snap carousel card; skip link; OS share/calendar intents.
- Colors: ruby #FF2D55 (JUMP IN, LIVE, filled heart), lavender #E8B9FF (compass active/ @UserName tint), purple350 #691FA9 (tile fill), purple450/500 (sheet), white70/white10 (stat labels, dividers) all in `tokens.ts`; green Trending pill ~#2FCF9A and link periwinkle ~#9D8CFF are NOT in tokens.
- Existing in `rn/src/ds`: AppText, BottomNav, Gradient, Icon, IconButton, PrimaryButton, Pill, ItemCard, Tabs, SlotTile, tokens, scale. Missing: BottomSheet primitive (DetailSheet implements its own), Skeleton, Chip, StatRow, SnapCarousel, Toast.

## 5. Legacy counterparts
- FTUE: `godot/src/ui/components/molecules/snap_carousel/ftue_screen.gd` + `ui/pages/discover/first_time_user_experience.tscn` (Skip, Jump In, `load_places`, Genesis Plaza place id).
- Discover: `ui/pages/discover/discover.gd/.tscn`; carousels `pages/discover/carrousel/`, places `pages/discover/places/`, event pills `pages/discover/event_pills_bar/`.
- Place sheet: `ui/pages/discover/jump_in/panel_jump_in_portrait.tscn` (+ `jump_in.tscn`, `download_warning.tscn`), `components/molecules/place_item/place_item.gd`, engagement (`components/organisms/engagement_bar`), `fav_button`.
- Event sheet: `ui/pages/events/event_details_portrait.tscn`, `button_reminder.gd`, `components/atoms/buttons/calendar_button`, `events_generator.gd`, `event_discover_card.tscn`.
- Not in Figma but in legacy: event reminder toggle (signed `PATCH/DELETE events/<id>/attendees` + OS local notification, disabled for guests), download warning ("May download up to XX MB"), recurrent dates list, event pill filter bar, search bar + suggestions + history, guest upgrade card, credits button, friends rail, last visited (local), favorites / my places rails, drag-to-expand via side panel, private-world precheck, `Global.favorite_destination_set`, review prompt on favorite.

## 6. RN status and gaps
- Exists: `discover/DiscoverScreen.tsx` (rails + search filter + ConnectionLost), `Cards.tsx`, `DetailSheet.tsx` (single-state place sheet with drag-to-close, heart/stats/share/like/dislike, `Share.share`), `api.ts`, `useDiscover.ts`, `history.ts`, `flow/FtueScreen.tsx` (mock FTUE with ghost Skip).
- Gaps vs this set:
  1. FTUE: real snap carousel with peeking neighbours, "Welcome @UserName start here" copy, JUMP IN with arrow, underline "Skip" link; places source; 1-card static fallback (13).
  2. Sheet: two snap points (collapsed peek ~/ expanded full screen) with X close in expanded; description, links (tappable), chips, categories; stat row variants (3-stat, location+likes, world globe+name, only pin); expanded-scroll with pinned button row.
  3. States: favorite persisted (signed `PATCH favorites`), like/dislike mutually exclusive (signed `PATCH likes`), selected styling, guest gating, error toasts.
  4. Event sheet: whole thing (emoji title, LIVE/attendee/Trending overlays, recurrence/duration/location cells, share + calendar buttons, schedule line with timezone, calendar intent via `Linking`/native calendar module).
  5. World variants (globe + name; JUMP IN -> `enter_world`).
  6. Backdrop blur (Figma blur ~12) vs current flat scrim 0.72.
  7. Back chevron/no-nav pushed Discover; compass active nav.
  8. Event card badges, "IN N MINS" pill.
  9. Loading screen after JUMP IN (not designed in RN).

## 7. Open questions / inconsistencies
- Skip label: "Skip and explore" (13) vs "Skip" (14/15); 13 is a static card, 14/15 a carousel; which is final (the later node ids 1710/1712 suggest the carousel).
- Download-size warning strip present in seq 81/82 (discover-c) but absent in these older sheet screens; confirm the final design.
- Collapsed place sheet stat row differs across nodes: 3-stat (19), location+likes (26), pin only (35), world (27/36); rule needed for which fields to show when data is missing (e.g. Genesis-style places without online count).
- Expanded titles use placeholder names (event expanded shows the place title); real data is event name.
- Event reminder: Figma shows only the OS calendar chooser (40) and the schedule-line calendar icon (42); legacy has an attendee reminder + notification. Which one is canonical, and does it need auth?
- Share text is the events template ("Visit the event ... following this link"); place/world share copy and jump link URL are not defined.
- 42 is cut at 1224px and omits the button row; confirm the buttons stay pinned while scrolling.
- Loading (25) is drawn portrait-rotated; in RN it must render landscape and be coordinated with the orientation switch.
- Nav avatar thumbnail ring differs between 16 and 18/37; compass highlight colour (lilac) vs token `lavender`.
