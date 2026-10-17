# discover-c report (manifest seq 75-103, source discover)

STATUS: seq 75-85 analysed here. Seq 86-103 were captured later and are analysed in docs/rn-ui-spec/reports/discover-d.md. get_design_context was never called, so no exact tokens were extracted; numbers below are read off the 720x1600 screenshots.

## 1. Screen inventory

Captured (shot files under docs/rn-ui-spec/shots/discover/):

| seq | id (suffix) | name | what it shows | state |
|---|---|---|---|---|
| 75 | 546-17533 | Discover-Skeleton-Scroll | Discover scrolled: tail of Last Played, Most Active, Favorites (2 cards), My Places (skeleton card+2 bars). Bottom nav visible | scrolled, partial skeleton (My Places loading) |
| 76 | 635-18454 | ThirdPerson (2400x1080 landscape) | In-world HUD landscape: chat btn, compass+share pill (top-left), eye btn + avatar (top-right), emote btn (bottom-left), action cluster bottom-right (hand/interact, E, F, 1-2 emote-slot, jump arrow) | in-world, no menu |
| 77 | 635-18547 | Discover-Landscape-loading | Discover with back arrow; skeleton rows Featured / Events / Last Played / Most Active | loading |
| 78 | 635-18998 | Discover-Landscape | Same screen loaded (back arrow, no bottom nav) | loaded |
| 79 | 1293-25430 | Discover-Landscape | Duplicate of 78 (identical content, 1px frame/border difference) | loaded |
| 80 | 1293-25827 | Discover-Portrait | Discover loaded, no back arrow, bottom nav present (title left-aligned at x=48) | loaded, tab root |
| 81 | 771-15737 | 01-Alert Data Events | Bottom sheet over blurred Discover: event "Chiri Builds" | modal, event detail |
| 82 | 771-16251 | 01-Alert Data Places | Bottom sheet: place "Meta Residence Tower - Apartments in Decentral..." | modal, place detail |
| 83 | 1607-21908 | Discover+Friends | Discover with a Friends rail between Featured and Events | loaded + friends |
| 84 | 1539-20007 | 01-Alert Data Events (actually a friend sheet) | Bottom sheet: friend "Namemracte #1234" at top, the place they are in, JUMP IN full width | modal, friend-in-place detail |
| 85 | 1582-20007 | Discover-Connection lost-01 | Same as skeleton 77 with search icon dimmed (disabled) | loading/connection-lost step 1 |
| 86 | 1582-20203 | Discover-Connection lost-02 | see discover-d.md | likely the connection-lost message/retry state |
| 87 | 1767-39473 | Discover-FeatureBanners | see discover-d.md | feature banners carousel |
| 88 | 1944-34968 | Discover-FeatureBanners-Detail | see discover-d.md | banner detail |
| 89 | 2005-21896 | Discover-FeatureBanners-Detail | see discover-d.md | banner detail variant |
| 90 | 1952-21149 | Discover-FeatureBanners-Button | see discover-d.md | banner CTA button variant |
| 91 | 1783-41275 | Loading | see discover-d.md | loading |
| 92 | 1693-20759 | Discover+Friends | see discover-d.md | friends variant |
| 93 | 1693-21242 | Discover-Story | see discover-d.md | story |
| 94 | 1700-23498 | Discover-Story | see discover-d.md | story variant |
| 95 | 2107-25669 | Discover+Categories01 | see discover-d.md | categories chips |
| 96 | 2110-26055 | Category | see discover-d.md | category page |
| 97 | 2159-27663 | Category-Search | see discover-d.md | category search |
| 98 | 2123-26835 | Discover-all carousels (720x3368) | see discover-d.md | full-length discover |
| 99 | 2190-22565 | Discover+Categories01 | see discover-d.md | categories variant |
| 100 | 2214-22767 | Discover+Background | see discover-d.md | background treatment |
| 101 | 2214-23123 | Discover+Overlay | see discover-d.md | overlay |
| 102 | 1783-41282 | Banner Feature (strip 7727x130) | see discover-d.md | banner component strip |
| 103 | 2130-27268 | "." (strip 7727x130) | see discover-d.md | component strip |

Note 84 is named "01-Alert Data Events" but shows a friend's place. Seq 79 duplicates 78.

## 2. Flow graph

- Entry: bottom-nav compass tab (80, 83, 75) -> Discover tab root (title only, no back arrow). Landscape / pushed variant (77/78) has a back arrow (<) left of the title; back returns to the in-world HUD (compass button in the HUD top-left pill, seq 76, opens Discover).
- Search icon (top-right) -> search mode (legacy has a search bar + keyword suggestions; Figma search screen for categories is seq 97, see discover-d.md). In 85 the icon is dimmed while loading/offline.
- Tap place card (Featured / Last Played / Most Active / Favorites / My Places) -> place bottom sheet (82). Tap event card -> event sheet (81). Tap friend avatar -> friend sheet (84). Sheets dismiss by drag handle (top bar, 200x8 at y~748) or tapping the blurred backdrop.
- JUMP IN (sheets) -> teleport/enter world -> in-world HUD (76). Share button -> OS share sheet. Calendar button (events) -> add-to-calendar / reminder. Like/Dislike (places) -> vote. Heart (place title row) -> favorite.
- Loading (77/85) -> loaded (78/80). Failure -> "connection lost" (86, see discover-d.md; RN already has ConnectionLost in backpack).

## 3. Per-screen spec (720x1600 portrait, Figma px)

### Discover list (80 loaded, 77/85 loading, 75 scrolled, 83 with Friends)
- Background: diagonal purple gradient (lighter violet #7a2bb0-ish top-left -> deep #43006b centre -> magenta-purple #7d1596 bottom-right). Gradient is app-wide.
- Header: title "Discover" bold ~46px white at x=48, baseline y~123 (tab-root). With back arrow (77/78): chevron at x~75, title shifts to x=117, y~116. Search icon (outline magnifier ~44px) at x~645, right gutter 48.
- Side gutter 48. Section title (semibold ~32px) e.g. "Featured" y=220; rail starts y=262.
- Place card: image 294x155 (radius ~16, 1px light border), gap 32 between cards (second card starts x=374), third card peeks at x=700 (rail scrolls horizontally, edge-bleed to right). Below image: title one line, ellipsized ("Meta Residence Tow...", "WonderMine Crafting...") ~28px semibold white at +30px; stats row at +72: person icon + "000" (online users) and thumb-up icon + "100%" (like rate), ~26px, 60% white.
- Event card (Events rail): image 406x240 (wider), second card peeks at x=486. Top-left badges on image: red "LIVE" pill (white text on #ff2d55-ish) followed by dark pill with green dot + person icon + attendee count "000"; top-right green "Trending" pill (teal-green #2fcf9a, white text). Upcoming events show white pill with red text "IN 5 MINS" top-left instead of LIVE. Title 2 lines max, ellipsized ("Experience Name two lines..."), subtitle "by <creator>" ~26px 70% white.
- Sections in order: Featured, (Friends N when friends online: 83), Events, Last Played, Most Active, Favorites, My Places (75). Section row spacing: ~360px between titles for place rails, ~446 for the event rail.
- Friends rail (83): section title "Friends" + count "5" in lighter purple. Avatars: 150px circle with 4px white ring and pastel background colour (face color per avatar), centred in a ~244px column; below name (bold ~26px white, "Username") and "Experience Name..." (scene name, ellipsized, 70% white ~22px). Rail peeks a third item. Sheet header for the same (84) calls the section "Online Friends 5" (blurred) - inconsistency with "Friends 5" in 83.
- Skeletons (77/85): rounded rect fills in translucent purple (rgba white 6-12%), place skeleton = 294x155 + two bars (294x28 and 294x28 at lower alpha), event skeleton = 406x240 + bars 406x34 and 406x28. Shimmer: lighter diagonal gradient on first tiles (legacy uses discover_carousel_skeleton.gdshader). Titles ("Featured","Events","Last Played","Most Active") remain visible while loading.
- 75: "My Places" and Favorites sections. Favorites show 2 cards without third peek (fewer than 3 items). My Places shows skeleton card while loading its own request, i.e. each rail loads independently.
- Bottom nav (80, 83, 75): black #151515 bar y=1420-1600 (180px), four icons: compass (active: filled lilac circle ~52px, #c9a8ff-ish), backpack, settings gear, avatar thumbnail ~72px circle. Content scrolls under/behind nav. Not shown in the pushed variant (77/78).
- Data: Featured = places API highlighted (`only_highlighted=true`), Events = events API live+upcoming, Last Played = local history, Most Active = places `order_by=most_active`, Favorites = authenticated places favorites, My Places = places owned/deployed by the user (needs wallet), Friends = friends service + their current realm/place (comms/Rust bridge). Exact figures "000"/"100%" are placeholders: users online and like_rate*100.
- States that must exist: loading skeleton per rail, empty rail (hide section; not designed), error/connection lost (85/86), loaded, scrolled, search active.

### Detail bottom sheets (81 event, 82 place, 84 friend)
- Sheet: top radius ~24, fill deep violet gradient (#43106f -> #320a5a), drag handle 200x8 pill centred, light violet. Backdrop = blurred, dimmed Discover. Sheet height ~870 (event: top y=730; place: y=678; friend: y=632 as it has the extra avatar row). Side padding 48.
- Event (81): title row with emoji + "Chiri Builds" (bold ~40px), "by Lazaro" (lighter, creator name in bolder lilac); hero image 624x~368 radius 16 with LIVE / attendees / Trending badges as on the card; bottom strip inside the image: dark translucent bar with warning triangle icon and "May download up to XX MB of data" (placeholder XX = scene size from download check; legacy download_warning.tscn). Meta row with two dividers: calendar icon "Weekly" (recurrence), clock icon "1 Hr" (duration), pin icon "-000,-000" (x,y parcel). Buttons row y=1420, 95px tall: share (square 94, dark violet fill), calendar (square 94, add reminder/calendar), primary JUMP IN (coral-red #ff2d55, flex, white bold 30px text + arrow-in-box icon).
- Place (82): title up to 2 lines with ellipsis ("Meta Residence Tower - Apartments in Decentral..."), heart (favorite) outline icon at right (x~636, y~790); "by Name Creator"; hero image 624x~370 with same download warning strip; stats row: person "000" | thumb-up "100%" | pin "-000,-000" (dividers between); buttons: share, like (thumb up), dislike (thumb down) squares + JUMP IN.
- Friend (84): header row: avatar thumbnail 80px circle + name "Namemracte" bold + "#1234" claimed-name discriminator (lighter, smaller); then place title (full 2 lines, no ellipsis), "by Name Creator", hero image with download warning, single full-width JUMP IN (no share/like). The button here has no arrow icon (differs from 81/82; likely design inconsistency).
- Interactions: JUMP IN text uppercase; haptic not shown; dismiss via drag or backdrop; Share = OS share of a decentraland jump link; calendar = event reminder (add via events API `attend`/notification); like/dislike = places API votes (needs auth); heart = favorite toggle.

### In-world landscape HUD (76, 2400x1080 -> shot 1600x720)
Documented by the in-world/HUD report too, listed here only because the compass pill is the Discover entry. Elements (shot px at 1600 width): chat bubble button (92,70, circle ~66), black pill with compass and share icons (175-322, 70), eye (hide-UI/camera) circle (1393,70), own avatar thumbnail ring (1491,68, ~84px), emote button bottom-left (123,637), bottom-right cluster: big interact hand button (1475,612, ~115px), E (1368,560), F (1510,486), "1 2" emote-slot (1427,502), jump up-arrow (1341,636). Virtual joystick area bottom-left not drawn.

## 4. Components and tokens
Seen: gradient background, section title, PlaceCard (img+title+users/likes), EventCard (badges LIVE / IN 5 MINS / Trending / attendees), FriendAvatar (ring + name + scene), Skeleton block, bottom sheet with handle + blur backdrop, square icon buttons (94x94, violet), primary coral button, stat row with dividers, warning strip, bottom nav (4 icons, active circle), search icon button, back chevron.
Existing in rn/src/ds: AppText, BottomNav, IconButton, Icon, PrimaryButton, Pill, Gradient, ItemCard, tokens, scale (u()). Existing in rn/src/discover: Cards.tsx (PlaceCard, EventCard), DetailSheet.tsx (sheet with heart, stats, share/like/dislike, Jump in), DiscoverScreen.tsx (Featured/Events/Last Played/Most Active, search filter, ConnectionLost, no skeleton: just "Loading places…").

## 5. Legacy counterparts
- godot/src/ui/pages/discover/discover.gd + discover.tscn (585 lines): carousels, search suggestions (search_bar/search_suggestions.gd), categories bar (categories/categories_bar.gd, category_tag), event pills bar (event_pills_bar), friends (friends/friends_generator.gd, friend_discover_card, panel_friend_detail_portrait, friend_jump_in), jump-in panel (jump_in/panel_jump_in_portrait.tscn, download_warning.gd), carousel generator/requester with skeleton shader (carrousel/), places/ (places_generator, places_helper), first_time_user_experience.tscn, scrollbar_requester.
- Old-app behaviour not in captured Figma: device-support / low-spec warning modals on first visit (discover.gd `_show_device_support_modal_if_needed`, `_show_low_spec_warning_if_needed`), opening an event/place from a notification (`_async_open_event_by_id`, `_async_handle_event_notification`), search with keyword suggestions, global loading/error messages across carousels (`_update_global_messages`), back-to-explorer button, `ui_location` analytics, event pills (filter chips) bar.

## 6. RN status and gaps
Exists: basic Discover (4 rails + search filter + detail sheet + connection-lost).
Gaps vs. Figma: skeleton loading per rail (currently text), Favorites and My Places rails, Friends rail + friend sheet, event sheet layout (recurrence/duration/location row, calendar button, LIVE/Trending badges check), download-size warning strip (needs size source), categories chips + Category page + Category search, feature banners carousel and banner detail/CTA, Story screens, back-arrow variant, bottom-nav compass state, pull-to-reload, per-rail independent errors, notification deep-link to event/place, device-support modals.

## 7. Open questions / inconsistencies
- "XX MB" is a placeholder; where does the size come from (scene content hash sizes via content server?) and is it shown for every place?
- Sheet 84 JUMP IN lacks the arrow icon and its section is "Online Friends" vs "Friends" on the page.
- Seq 78 and 79 identical; seq 84's layer name says Events but is a friend sheet.
- Calendar button behaviour (local calendar vs reminder via events API) unspecified.
- Landscape Discover (77/78) is simply the portrait content with back arrow at 720x1600; there is no real landscape layout. Confirm intended orientation handling.
- Seq 86-103: see discover-d.md.

## Missing
None. Seq 86-103 are covered in docs/rn-ui-spec/reports/discover-d.md.
