# discover-b report (manifest seq 44-74, source discover: event-detail variants, search flow, skeletons, results/errors)

STATUS: COMPLETE. All 31 screenshots (seq 44-74) were read. `get_design_context` was NOT called (its mandatory figma-design-to-code skill was not loaded, and the seat has a tight call limit), so there are no exact tokens; every number is read off the 720x1600 PNGs (Figma px = RN `u()` units, see `rn/src/ds/scale.ts`). Note: the range label says "landscape" but every frame in 44-74 is 720x1600 portrait; the landscape Discover is in discover-c (seq 77-79).
Consistent with `discover-c.md` (list/cards/sheets/tokens) and `legacy-godot.md` section 3 (Discover, search). Common elements are described once in discover-c; only deltas are repeated here.

## 1. Screen inventory

All files: `docs/rn-ui-spec/shots/discover/discover__<id>.png`. Shared tab-root chrome = gradient bg, "Discover" header, bottom nav (compass active), unless noted.

| seq | id (suffix) | name | what it shows | state/variant |
|---|---|---|---|---|
| 44 | 388-8777 | Discover-01 | Discover home: header + search icon; Featured (2 place cards + peek), Events (LIVE/Trending card, "IN 5 MINS" card), Last Played, "Most Active" title cut by nav | loaded, tab root |
| 45 | 388-9300 | 02-Detail-Event-Rimind-Off | Event sheet over blurred Discover: "Chiri Builds", by Lazaro, hero with "IN 5 MINS"/"Featured", Weekly / 1 Hr / -000,-000, share + calendar squares + REMIND ME | modal, reminder OFF |
| 46 | 399-12413 | 03-Detail-Event-Rimind-On | Same sheet, button now "REMINDER SET" with check, coral/red outline and text | modal, reminder ON |
| 47 | 399-12958 | 04-Detail-Event-Share | Android system share sheet ("Sharing text") over blurred Discover. Text: "Visit the event '<emoji> Cartoon Nights at Chiri's ' following this link https:..." | OS share sheet (after tap share) |
| 48 | 399-13457 | 05-Detail-Event-Calendar | Event sheet (dimmed) + Android chooser "Complete action using" (Calendar, Outlook; Just once / Always) | OS intent chooser (after tap calendar) |
| 49 | 399-13955 | 06-Detail-Event-Calendar | Android calendar editor prefilled: title "<hammer emoji>Chiri Builds", Wed Jan 21 5:00 PM -> 6:00 PM, location "Decentraland at -35,-31", reminder "10 mins before", description "Chiri Storytelling Community / Chiri Builds / Streams to learn how to create 3D Low Poly assets..." ; Cancel / Save | OS calendar insert intent |
| 50 | 414-8213 | Discover-01 | Identical to 44 (start frame of the search flow) | loaded, tab root |
| 51 | 414-10514 | 02-Search | Search mode entered: back chevron + focused empty search field "Search"; Discover content still shown behind keyboard | search focused, empty query, Discover content NOT yet replaced (keyboard open; no recents shown here) |
| 52 | 433-9004 | 03-Searching | Query "Ca": suggestion list (Caos, Casa, Casas, Casa Azul, Casa Py, Casa Rousto...), keyboard open, clear (X) in field | typing, suggestions |
| 53 | 461-9328 | 03-Searching | Query "Casa": same list (cursor variant), keyboard open | typing, suggestions |
| 54 | 433-11550 | 04-Sugestions | Query "Casa", keyboard dismissed: full suggestion list (Casa, Casadei, Casas, Casa Azul, Casa Py, Casa Rousto, Casa Sur, Casa True, Casa Z, Casa Zombie) with bottom nav | suggestions, keyboard closed |
| 55 | 635-17545 | Discover-Skeleton | Search mode (query "Casa") over skeleton rails Featured / Events / Last Played | results loading |
| 56 | 433-11756 | 05-Results | Search "Casa" results: Featured (Casai..., Casa Rousto), Events (Casa Fashion, Casa SheFi Salon), Last Played (Casa Crystal City, Exodus (casa)), Most Active title | results loaded |
| 57 | 635-17806 | 06-Message Error | Search error: icon, "Something went wrong", "We couldn't load the results. Please try again." | error |
| 58 | 627-12457 | 07-Not-Results | "No result found", "Try searching with different keywords." | empty results |
| 59 | 500-10741 | 03-Searching | Empty focused field with "Recent searches" + "Clean all" and one history row "Casa" (clock icon) | recents (query empty, history non-empty) |
| 60 | 509-12698 | 03-Searching | "Ca" typing with suggestion rows still animating out/in under the keyboard (ghost rows "Casa Sur / True / Z / Zombie" overlapped at y~960-1320) | transition frame (list reflow); same screen as 52 |
| 61 | 1061-16348 | Discover-01 | Identical to 44 (start of coordinate-search flow) | loaded |
| 62 | 1061-16762 | 02-Search | Identical to 51 | search focused, empty |
| 63 | 1061-17183 | 03-Searching | Query "44,-48": ONE suggestion "GolfCraft mini golf" (place name resolved from coordinates), keyboard open | coordinates query, suggestion |
| 64 | 1061-17307 | 05-Results | Query "44,-48" results: only Featured rail with one card "GolfCraft mini golf..." (left-aligned, single card), other rails hidden | coordinates results |
| 65 | 1061-17624 | Discover-01 | Identical to 44 | loaded |
| 66 | 1061-17760 | 02-Search | Identical to 51 | search focused, empty |
| 67 | 1061-17854 | 03-Searching | Partial coordinates "44,-" -> 4 placeholder rows "Nombre 1 Place in 44,XX" (row 3 reads "1 Nombre 1 Place in 44,XX in 44," = design typo) | partial-coordinate suggestions (placeholder copy) |
| 68 | 1061-17716 | 05-Results | Query "44,-" results with no keyboard: Featured (GolfCraft only), Last Played (Casa Crystal City, Exodus (casa)), Most Active (Butterfly Prawn Farmi..., Genesis Plaza), Favorites (Audio Hot Lab, Exodus) | results; multi-rail, rails with no matches omitted |
| 69 | 546-11668 | Discover-Skeleton | Home skeleton: header + search icon, Featured/Events/Last Played titles with shimmering placeholder cards | loading (initial) |
| 70 | 1387-32741 | Discover-Skeleton | Identical to 69 | loading |
| 71 | 566-22151 | Discover-Skeleton-Scroll | Home loaded, but Featured card 1 (Genesis Plaza) and Events card 1 (LIVE) show the "image failed" placeholder (framed dark-violet tile + crossed-image icon); badges still render | image-load-error state per card |
| 72 | 546-14740 | Discover-Skeleton-Scroll | Home fully loaded (same as 44, 1px y offset) | loaded |
| 73 | 546-16084 | Discover-Skeleton-Scroll | Featured rail scrolled to its end (Audio Hot Lab card at x~52, previous card clipped at left) plus a trailing skeleton card (294x155 + 2 bars) at x=378 | pagination: infinite-scroll "loading more" skeleton at rail end |
| 74 | 546-16604 | Discover-Skeleton-Scroll | Same as 72 | loaded |

Groupings: 44/50/61/65/72/74 (home loaded); 51/62/66 (search empty focus); 52/53/60 (typing "Ca"/"Casa"); 69/70 (home skeleton); 47-49 are OS screens, not RN.

## 2. Flow graph

- Entry: bottom-nav compass tab -> Discover home (tab root, no back arrow). Also reachable from the in-world HUD compass (see discover-c).
- Home -> tap search icon (top-right, 44px outline at x~645,y~122) -> Search mode (51): header replaced by `[<] [search field]`; keyboard opens; bottom nav hides while keyboard is up (51,52,53,59,63,66,67) and returns when the keyboard is dismissed (54,56,57,58,64,68).
- Search empty + focused: show "Recent searches" (59) if history exists; else (51) the page content stays visible behind the keyboard (design shows Discover rails with no recents list; legacy shows recents only if history non-empty and otherwise an empty-query places fetch).
- Typing, length <= 2: history filtered (Legacy: max 4 matches by contains / similarity > 0.2). Length >= 3: debounce 0.4 s then suggestions fetch (places + events, limit 8 each) (52-54). Coordinates input ("44,-48", "-35,-31", or a bare integer) resolves a place name suggestion (63) instead.
- Tap a suggestion (or press keyboard Done/submit) -> field text = keyword, keyboard closes, three states in order: skeleton rails (55) -> results rails (56/64/68) | no results (58) | error (57). Successful non-empty result adds the query to history (legacy `add_search_history`).
- Tap X in field (642,122) -> clears text (legacy `cleared`: search filter reset, shows recents/suggestions again, stays in search mode). Tap back chevron (76,122) -> exits search mode, Discover home restored (filter cleared).
- Tap place/event card in results -> same detail sheets as home (discover-c 81/82). Event card -> event sheet (45).
- Event sheet (45/46): share square -> OS share sheet (47) with text "Visit the event '<title>' following this link <url>"; calendar square -> Android intent chooser (48) -> calendar editor (49), Save/Cancel return to the sheet; REMIND ME -> toggles to REMINDER SET (46) (tap again = unset, inferred); drag handle / blurred backdrop dismisses; JUMP IN absent in the "upcoming" variant (see discover-c 81 for LIVE variant).
- Home loading: 69/70 -> loaded 44; rails load independently (71, 73 show partial states). Failure: whole-page "connection lost" (discover-c 85/86); search failure: 57.

## 3. Per-screen spec (720x1600 portrait, Figma px)

Common chrome (all frames): background diagonal gradient (see discover-c), side gutter 48, status bar not drawn (RN safe area). Text is Inter. Section title ~32px semibold white. Bottom nav 180px (y 1420-1600, #151515-ish) with 4 icons: compass (active: lilac filled circle ~52px), backpack, gear, avatar circle ~72px.

### 3.1 Discover home (44, 50, 61, 65, 72, 74) - loaded
Identical to discover-c section "Discover list". Delta/confirmations: header "Discover" bold ~46px at x=48 centre y~122; search icon at x~644. Rails: Featured titles y=220 (cards y 260-415, image 294x155 radius ~16 with 1px translucent light border, title y~446 ~28px semibold, stats row y~487: person icon "000" + thumb icon "100%"); Events title y=581 (cards y 620-860, 406x240; title y~897 ~28px semibold 1 line ellipsis; "by <creator>" y~941 ~26px 70% white); Last Played title y=1041 (cards y 1081-1237; title y~1267; stats y~1308); Most Active title y~1402 (cut by nav). Card pitch 326 (294 + 32 gap) for places, 438 (406 + 32) for events; third card peeks off-screen right. Texts: "Genesis Plaza", "Meta Residence Tow...", "Experience Name two lines...", "by Lazaro", "CRYPT0M1NOTA...", "by Crypt0M1", "WonderMine Crafting...", ". dejay Bar & Restauran..." (placeholder stats "000" and "100%").
Event card badges (top-left, y~652): LIVE = red #ff2d55-ish pill with white bold "LIVE" then black pill with green dot + person icon + count; "IN 5 MINS" = white pill, red bold text; "Trending" = teal-green pill top-right (white text); event sheet hero uses a pink-orange gradient "Featured" pill top-right instead. Event time pills: "IN 30 MINS", "IN 2 DAYS" (56) => format "IN <n> MINS|HOURS|DAYS" uppercase.
Interactions: rails horizontal scroll (snap not required; free scroll with 32 gap, left inset 48); page vertical scroll under the bottom nav; tap card -> sheet; tap search -> search mode.

### 3.2 Search field + states (51-68)
Header in search mode (replaces title+icon), y 88-156: back chevron (white, ~28px, x~76, hit area >= 88x88 left of field); field x 133-672 (539 wide), 68 tall, radius ~12, 1px lilac border when focused/filled, dark-violet translucent fill (#2b0050-ish, darker when idle without border: 51 shows no border, text placeholder "Search" ~28px 50% white); leading magnifier ~38px at x~172; input text 30px white at x=208; trailing clear X (~24px, lilac) at x~643 only when text non-empty (not shown in idle 51). Cursor caret white. Keyboard "Done" return key (design uses Android IME action Done) = submit.

**Search idle (51/62/66)**: field empty/placeholder; content below is the Discover home rails (no recents section; design relied on keyboard covering). Open question: recents not shown here but shown in 59 (see Q1).

**Recents (59)**: section row at y~214: "Recent searches" (~30px semibold, 60% white lilac #c9a8ff-ish) left at x=48; "Clean all" right-aligned (x~575-672), underlined ~24px lilac. Row (y 290-372): clock icon (~30px at x~67), text "Casa" ~30px white at x=97, divider 1px translucent white across x 48-672 at row bottom. Row height ~120 (list pitch 120). Tap row -> runs that search; "Clean all" -> clears history (legacy writes empty array). Max 4 shown (legacy).

**Suggestions list (52/53/54/60/63/67)**: rows start y~236 (row centre), pitch 120, each row: leading magnifier outline (~30px at x~68), text ~30px white at x=97, 1px divider below (x 48-672). Texts (design placeholders): Caos, Casa, Casas, Casa Azul, Casa Py, Casa Rousto, Casa Sur, Casa True, Casa Z, Casa Zombie; 54 shows Casa, Casadei, Casas, ... (list reorders as the query changes). List scrolls; with keyboard open it is clipped at y~918, keyboard-aware (RN: KeyboardAvoidingView / `keyboardShouldPersistTaps="handled"`). Background in the list state is the plain gradient (rails hidden). Max 8 per source (legacy `SEARCH_LIMIT`); legacy icon variants by type: HISTORY clock, PLACE/ coordinates map pin ("world-outlined"), EVENT calendar-outlined, default search; Figma only draws the magnifier (65: every row is magnifier) -> only magnifier required, keep type-icons optional.
Coordinates (63): query "44,-48" -> a single row with the place name "GolfCraft mini golf" (magnifier icon). 67: "44,-" shows placeholder rows "Nombre 1 Place in 44,XX" (design mock copy; real behaviour = none until the query parses as coordinates; see Q2).
60 is the same list mid-transition (rows from a longer list slide under the keyboard); no separate design.

**Results (56, 64, 68)**: header stays in search mode with the committed query; rails re-use home cards. Order: Featured, Events, Last Played, Most Active, Favorites (68). A rail with 0 matches is omitted (64 only Featured; 68 no Events). Single-result rail left-aligned, no peek. Section titles keep same y rhythm but rails collapse (68: Featured y 220, Last Played y 581, Most Active y 942, Favorites y 1303, i.e. rail pitch 361 for place rails). Result card titles show the query token ("Casa Rousto", "Casa Fashion"). Single-word titles can ellipsize earlier ("Casai..." at 294 wide). Coordinates search returns the place whose bounds contain the parcel (64).
Quirk: 56's rail y of Events title (581) and card (620) match home; in 68 the first rail starts at 220 with search header 156 bottom -> same spacing as home.
Data: places `GET /api/places?search=<q>&limit=..&sdk=7` and events `GET /api/events?search=<q>&limit=..` (legacy `search_suggestions.gd`, `PlacesHelper`), Last Played = local history filtered; Favorites = authenticated places favorites; coordinates -> places lookup by position (`PlacesHelper.parse_coordinates`, `async_get_name_from_coordinates`). RN currently filters loaded rails client-side by title.

**Skeleton while searching (55)**: same rails skeleton as home (69), header in search mode with query filled, X visible; keyboard hidden, nav shown.

**Error (57)**: no rails; centred block: 140x140 rounded-square outline (radius ~16, 3px lilac #c9a8ff at ~70%) containing a magnifier with two dash lines underneath (search-empty glyph), top y~497; title "Something went wrong" bold ~46px, 2 lines centred, colour lilac-grey (#c7a9dc-ish, ~75% white) y 690-800; body "We couldn't load the results. Please try again." ~30px regular, same colour, 2 lines (y 830-880). No retry button drawn (retry = edit/resubmit query; Q3). Legacy strings: `COMMON_SOMETHING_WENT_WRONG_RETRY`.
**Empty (58)**: same layout/icon; title "No result found" (1 line, ~46px bold) and body "Try searching with different keywords." (2 lines). Legacy keys `DISCOVER_NO_RESULT_FOUND`.

### 3.3 Event detail sheet (45, 46) - and OS hand-offs (47-49)
Same sheet shell as discover-c 81 (top radius ~24, gradient #43106f->#320a5a, handle 200x8 at y~752, backdrop blurred Discover). Sheet top y=732. Content (side padding 48):
- Title row y~823: emoji from event name + "Chiri Builds" (bold ~42px white). Emoji is part of the title string.
- "by Lazaro" y~880: "by" lighter 60%, creator name lilac-bold ~28px.
- Hero image x 48-672 (624) x y 927-1295 (368), radius ~16, 1px light border; badges: top-left white pill "IN 5 MINS" (red text, y~959), top-right pink-to-magenta gradient pill "Featured" (white text). LIVE events: red "LIVE" + attendee pill instead (discover-c 81). No download-warning strip here (unlike places/world sheets) - consistent with events having no scene size.
- Meta row y~1347 between 1px vertical dividers at x~243 and x~431: calendar icon + "Weekly" (recurrence; "Once"/"Daily"/... derive from event.recurrent/rrule), clock icon + "1 Hr" (duration), pin icon + "-000,-000" (x,y). Each cell is a third of the width, text ~30px, centred.
- Button row y 1419-1514 (95 tall): share square (94x94, dark violet #45146e-ish, radius ~12, share-nodes icon), calendar square (94x94, calendar icon), then wide button x 268-672 (404 wide) 95 tall: radius ~12, 3px white outline, fill #4a1478-ish, bell icon + "REMIND ME" (bold ~30px white, uppercase).
- Reminder ON (46): button outline and text colour switch to coral/red (#ff2d55-ish), fill darker (#2a0a48), icon becomes a check, label "REMINDER SET". Rest unchanged.
- Behaviour: REMIND ME -> events API attend (`POST /events/{id}/attendees`, needs signed fetch) + local notification (legacy `button_reminder.gd` 262 lines: sets/clears reminder, schedules notification) -> state 46. Share -> OS share sheet (47) with text "Visit the event '<title>' following this link <https://...>" ; the link is the decentraland jump link for the event. Calendar -> Android `ACTION_INSERT` intent (48/49: title, begin/end time, location "Decentraland at <x>,<y>", description = event description) / iOS: EventKit or ICS (RN: `react-native-calendar-events` or `Linking` with intent; Q4). 47-49 are native screens; RN only fires intents.
- In 48/49 the sheet stays dimmed behind; after returning the sheet is as left (not shown).
- Note: 45/46 show an upcoming event whose primary action is REMIND ME; for LIVE events the primary action is JUMP IN (discover-c 81) - one component with variants (live -> JUMP IN, upcoming -> REMIND ME/REMINDER SET). The rest of the sheet states (place/world/more) belong to other reports (seq 1-43 range, discover-a).

### 3.4 Skeleton / loading (69, 70, 55, 71, 73)
- Rail skeleton (home, 69/70): section titles are real text; Featured/Last Played: 294x155 radius 16 block (fill translucent violet, ~rgba(255,255,255,0.07), first card lighter with diagonal highlight gradient = shimmer), then two bars 294x28 at y+170 and y+214, radius ~6, lower alpha; 2 cards + peek. Events skeleton: 406x240 card + bars 406x34 (y 880-914) and 406x28 (y 927-955). Header "Discover" + search icon are live. Bottom nav live. Titles: "Featured", "Events", "Last Played", "Most Active" (partially hidden). 70 is a duplicate of 69.
- Shimmer: highlight sweeps diagonally across blocks (legacy shader `carrousel/discover_carrousel_skeleton.gdshader`); RN: Reanimated/LinearGradient translateX loop ~1.2 s, same gradient on all blocks in sync.
- Per-card image failure (71): card keeps its frame (1px border, fill #3d0f66-ish, radius 16) with a centred 56px "image-off" icon (lilac, 50%); texts and badges still shown. Show on `onError` of `Image`.
- Pagination (73): at the end of a rail a trailing skeleton card (same size) with 2 bars appears while the next page loads (infinite scroll `on_request(offset, limit)` in legacy `carrousel_generator`); then replaced by cards (74/72).
- Search skeleton (55): see 3.2.

## 4. Reusable components / tokens
- Colours (read from pixels, approximate): bg gradient #7b2cb5 (top-left) -> #43006b (centre) -> #7d1596 (bottom-right); text white; secondary text ~70% white; lilac accent #c9a8ff (recents title, clear icon, nav active, creator name); skeleton fill white 6-10% alpha; sheet fill #43106f -> #320a5a; square button #45146e; bottom nav #151515; red #ff2d55 (LIVE, REMINDER SET); teal #2fcf9a (Trending); pink/magenta gradient (Featured pill).
- Type: Inter; header 46 bold; section 32 semibold; card title 28 semibold; meta 26 regular; suggestion row 30 regular; empty/error title 46 bold, body 30 regular; button 30 bold uppercase.
- New components needed (none exist in `rn/src/ds`): `SearchField` (back + input + clear), `SuggestionRow`, `RecentSearches` (title + Clean all + rows), `EmptyState` (icon + title + body; used for "No result found", "Something went wrong"), `Skeleton`/`Shimmer` (+ `PlaceCardSkeleton`, `EventCardSkeleton`), `ImageFallback`, `StatusPill` (LIVE/IN n MINS/Trending/Featured), `OutlineActionButton` with `on` state, `MetaRow` with dividers.
- Existing in `rn/src/ds`: AppText, IconButton, Icon (+ iconSvgs: need magnifier, chevron-left, X, clock, bell, check, calendar, share, pin, image-off), PrimaryButton, Pill, Gradient, BottomNav, tokens, scale. In `rn/src/discover`: Cards (PlaceCard/EventCard), DetailSheet, DiscoverScreen.

## 5. Legacy counterparts (godot/src/ui)
- Search bar: `components/molecules/search_bar/search_bar.{gd,tscn}` (signals `cleared`, open/close).
- Suggestions: `pages/discover/search_bar/search_suggestions.{gd,tscn}`, rows `pages/discover/search_keyword.tscn`, container `pages/discover/search_sugestions_container.tscn`. Rules: debounce 0.4 s (`SEARCH_DEBOUNCE_SEC`), limit 8 per source (`SEARCH_LIMIT`), <= 2 chars show up to 4 history matches (contains or similarity > 0.2), >= 3 chars hit places (`PlacesHelper.get_api_url()?search=..&limit=..&sdk=7[&tag=allowed_ios]`) + events (`/?sdk=7&search=..`), coordinates parse (`x,y` or bare int) -> name via `PlacesHelper.async_get_name_from_coordinates`; stale-response guard (`_search_request_id`); strips punctuation (`trim_string`: `.*!?¡¿-_[]<>'"%&\/,.;:`). Types: HISTORY (clock), COORDINATES (map), place, event (calendar).
- Page logic: `pages/discover/discover.gd` (`set_search_filter_text` pushes `search_param` to Featured/Last Played/Most Active/Favorites/Events generators; `_update_global_messages` produces no-results/error from the per-rail `LoadingStatus` (OK_WITH_RESULTS/OK_WITHOUT_RESULTS/ERROR/LOADING) - all rails error => error, none with results => "No result found"; `Global.get_config().add_search_history(text)` only when any rail has results). Hidden rails when search active. i18n keys exist: `DISCOVER_RECENT_SEARCHES` "Recent searches", `DISCOVER_CLEAN_ALL` "Clean all", `DISCOVER_NO_RESULT_FOUND` "No result found" (godot/locale/en.csv); the Figma "Something went wrong / We couldn't load the results. Please try again." and "Try searching with different keywords." need new keys (old key `COMMON_SOMETHING_WENT_WRONG` reads "Ups! Something went wrong").
- Skeleton: `pages/discover/carrousel/{place_discover_skeleton.tscn, discover_carrousel_item_loading.tscn, discover_carrousel_skeleton.gdshader, discover_carrousel_skeleton_material.tres}`, `pages/events/event_discover_skeleton.tscn`.
- Events: `pages/events/{event_details_portrait.tscn, event_discover_card.tscn, button_reminder.{gd,tscn}, events_generator.gd, time_pill_red/black.tres}`; metrics: `SEARCH_SELECT_INPUT`, `SEARCH_ERASE`, `SEARCH_GOBACK`, `SEARCH_CLICK`.
- Behaviour Figma omits: search history persistence (config `search_history`, cleared by Clean all); submit on keyboard action; iOS-only `tag=allowed_ios` filter on places/events (Apple review); event suggestions (calendar icon) and world suggestions; analytics; infinite scroll offset paging per rail; "Back" semantics when search opened from in-world (return to explorer).

## 6. RN status and gaps
Exists (`rn/src/discover`): `DiscoverScreen.tsx` has a search toggle (magnifier <-> close icon, `TextInput` placeholder "Search places and events", client-side title filter via `matchesQuery`, `query !== null` = searching), `ConnectionLost`, text "Loading places…" (no skeleton), `history.ts` = in-memory "Last Played" only, no search history, `api.ts` has Featured/MostActive/Live/Upcoming URLs only.
Gaps vs. seq 44-74:
1. Search mode header (back chevron + styled field with clear X) instead of icon toggle; hide bottom nav while keyboard is open (or let the keyboard overlay).
2. Recent searches (persisted storage, AsyncStorage/MMKV; max 4 on typing, list on empty; "Clean all").
3. Suggestions: debounce 400 ms, places+events search endpoints, coordinate parsing and name lookup, stale-response guard, keyword normalisation; submit/select -> results.
4. Server-side results rendering (Featured, Events, Last Played, Most Active, Favorites) with empty-rail hiding; no-results and error states (EmptyState component) and the "all rails error => Something went wrong" rule.
5. Skeleton rails with shimmer (home + search), image-fail fallback tile, rail pagination skeleton and infinite scroll.
6. Event sheet: recurrence/duration/location meta row, share/calendar squares, REMIND ME <-> REMINDER SET state with attend API + local notification, native calendar intent, share text, IN n MINS / Featured badges, emoji in titles.
7. Favorites rail needs auth'd fetch; Last Played must survive restarts for parity (currently memory only).
8. New icons: image-off, search-empty glyph, bell, check, clock, calendar, share, pin, chevron-left, X.

## 7. Open questions / inconsistencies
1. 51/62/66 (empty focused search) show the old Discover rails behind the keyboard while 59 shows recents; clarify: recents only when history exists, otherwise keep rails visible (matches legacy) or show nothing.
2. 67 uses placeholder rows "Nombre 1 Place in 44,XX" (row 3 contains a typo "1 Nombre 1 Place in 44,XX in 44,"): not final copy. Coordinates suggestion semantics (partial "44,-") are undefined; legacy only resolves when the string parses as a coordinate pair (>= 3 chars).
3. Error state (57) has no retry button; sibling "connection lost" pages (discover-c 85/86, backpack) have one. Decide on retry affordance (pull to refresh / button).
4. Add-to-calendar on iOS undefined (47-49 only show Android Samsung UI); the calendar editor in 49 shows description text that must be built from event fields (title, "Decentraland at x,y", description).
5. Event timing in the calendar sample (Wed Jan 21 5:00-6:00 PM) vs sheet "1 Hr" duration: duration drives end time; recurrence "Weekly" in the sheet vs "Don't repeat" in the calendar editor (49): RN does not forward the rrule.
6. Sheet "Featured" badge (45) vs card "Trending" badge (44): two different pills; decide which fields map (`event.highlighted`/`trending`).
7. Seq 44/50/61/65 and 72/74 and 69/70 are duplicates; 60 is a transition frame; no distinct design.
8. Search result rails reuse "Last Played" and "Favorites" (68) which only apply to signed-in/guest-with-history users; guest behaviour for Favorites unspecified.
9. Header in search mode: no explicit "cancel" text, back chevron doubles as exit; Android system back should do the same (Q: first back closes keyboard, second exits search?).
