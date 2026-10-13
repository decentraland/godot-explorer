# Report: menu-a (manifest seq 273-299)

Source: Figma file `nxP78pUCEHj3BY0meiXipe`, page `2:60` ("menu"). All 27 screenshots captured and viewed (`docs/rn-ui-spec/shots/menu/`). 1600x720 landscape frames except 279, 289 and 291 (720x1600 portrait). `get_design_context` was not called (Figma MCP quota throttled to about 3 calls/min); values are measured from the PNGs plus `rn/src/ds/tokens.ts`. Counterpart report for seq 300-326: menu-b.

## 1. Screen inventory

| seq | id (suffix) | name | what it shows | state / variant |
|---|---|---|---|---|
| 273 | hud__106-2203 | HUD | In-world HUD with first-run coach-mark labels: "Chat" (under chat button), "Menu" (under avatar button top-right), "Character movement" (above joystick), "Emotes" (beside emote button), "Controls" (above gamepad). Scene dimmed with a dark-red tint | tutorial overlay 1; avatar button top-RIGHT, chat top-left |
| 274 | hud__108-2191 | HUD | Same HUD dimmed, no labels except a "Switch camera" pill (center) and a large outlined hand with a curved two-way arrow at (1190,330) | tutorial overlay 2 (camera-switch gesture hint) |
| 275 | hud__367-4161 | HUD | Final live HUD: avatar button (82px) at top-LEFT (150,72) with chat button (66px) to its right (256,73), 5 chat lines below ("Clara #1l0u Teleported ..." orange name + white text on 40% black rows, last one multi-line), emote button (bottom-left, 80px), gamepad bottom-right (F, +, E, hand, big jump) | HUD, chat populated |
| 276 | discover__367-5030 | Discover | In-world Discover drawer: left rail + panel, scrolled down: "Events" row (LIVE card + IN 5 MINS card) and EXPLORE MORE button | drawer scrolled to bottom |
| 277 | discover-menu-open__609-3721 | Discover-Menu open | Drawer at top: title "Genesis Plaza PlazaPl...", "by Lazaro", kebab menu, Featured carousel (Genesis Plaza, Meta Residence Tov...), start of Events | base state (identical content to 278/280) |
| 278 | discover__888-16872 | Discover | Drawer, same as 277 but kebab closed | base state |
| 279 | discover-ingame__889-4068 | Discover-Ingame | Fullscreen PORTRAIT Discover: back chevron + "Discover" title + search icon; hero carousel card with dots; "Friends 5" avatars; "Events" cards; "Last Played" | opened by EXPLORE MORE (in-game variant of Discover page) |
| 280 | discover__1351-15367 | Discover | Drawer, identical to 278 | duplicate |
| 281 | discover-place__1351-15649 | Discover-Place | Centered modal 1060x460 over blurred world: grey image placeholder (600x460) | "Name experiencia / Name experiencia", "By Creator", JUMP IN | place detail modal |
| 282 | discover-event__1511-6307 | Discover-EVENT | Drawer where the scene has no Featured section: Events row first (CRYPTOM1NOTAUR - THU... "IN 5 MINS" + Featured tag; DCL Japan "IN 30 MINS"), EXPLORE MORE peeking below | events-only variant, upcoming events |
| 283 | discover-event-live__1511-6210 | Discover-Event_live | Event modal: image area has LIVE pill + green-dot user count + "Featured" tag; right: "Name event", "By Creator", ruby JUMP IN | event live |
| 284 | discover-menu-open__893-15236 | Discover-Menu open | Kebab dropdown open: Share, Report Content, Report bug | menu open (copy of 277 + dropdown) |
| 285 | discover-share__893-14904 | Discover-Share | Native Android share sheet ("Sharing text") with message "📍 Join Me At Space Runner following this link: https://mobile.dclexplorer.com/open?realm=spacerunner.dcl.eth", copy button, direct-share targets, app row | OS share sheet (not a custom UI) |
| 286 | discover-event-reminder__1525-5237 | Discover-Event-Reminder | Event modal for upcoming event: "IN 5 MINS" outlined pill + Featured tag; outlined button "REMIND ME" with bell icon | event upcoming, reminder not set |
| 287 | discover-event-set__1525-5303 | Discover-Event-Set | Same, button now ruby-outlined "REMINDER SET" with check icon | reminder set |
| 288 | discover-menu-open__893-15568 | Discover-Menu open | Kebab dropdown open (same as 284) | menu open; leads to Report content |
| 289 | settings-account__893-16220 | Settings-Account (misnamed) | PORTRAIT "Bug Report" modal over blurred portrait Discover: bug icon, Issue Type* dropdown "Select", Description* textarea ("Briefly describe what happened.", 0/300), Screenshot (one filled thumbnail with X + one "+" slot), CANCEL, SUBMIT (disabled look) | report-bug form, empty/invalid |
| 290 | discover-menu-open__894-17477 | Discover-Menu open | Kebab dropdown open (same as 284) | menu open; leads to bug report |
| 291 | report-content-form__894-16932 | Report COntent Form | Reference capture of an external Google Form in iOS Chrome ("Decentraland: Report Content Policy Violation", docs.google.com, status bar 20:44) | external browser page, not RN UI |
| 292 | social__426-5733 | Social | Social drawer, FRIENDS tab: ONLINE (2) two rows (one with enabled ruby jump-in button, one dimmed "Same Place"), OFFLINE (2), BLOCKED (2) peeking | friends list |
| 293 | social-request__889-4540 | Social-Request | REQUESTS tab: RECIEVED (1) two rows with check + X outlined buttons, SENT (1) row with X | requests |
| 294 | social-request-accept__889-5102 | Social-Request-Accept | Same, first row's check button pressed (green border + green check) | accept pressed state |
| 295 | social-request-decline__889-5361 | Social-Request-Decline | Same, first row's X pressed (ruby border + ruby X) | decline pressed state |
| 296 | social-nearby__889-5850 | Social-Nearby | NEARBY tab (underline): 4 rows each with ruby ADD FRIEND button | nearby list |
| 297 | social-nearby__889-6541 | Social-Nearby | Same list but no tab underlined (all three tab labels light) | in-between / tab-unselected frame (likely duplicate) |
| 298 | social-nearby-empty__931-24777 | Social-Nearby-Empty | NEARBY tab: icon, "No one nearby", "Try traveling somewhere else to meet new people." (tab shows no count) | empty |
| 299 | social__933-25219 | Social | FRIENDS tab empty: "No friends yet", "Meet people nearby and add them as friends." | empty |

## 2. Flow graph

- World (HUD seq 275) -> tap avatar "Menu" button (top-left) -> in-world drawer opens as an overlay on the left: a 100px icon rail + 724px content panel (x 108-832, y 32-672, ~640 high). Rail icons top to bottom: Discover (compass, y 103), Social (people+heart, 205), Notifications (bell, 305, ruby unread dot), Backpack (408), Settings (gear, 509), Profile (avatar 46px, 605). Active item = filled lavender icon plus a faint vertical glow at the rail/panel seam. Notifications content = menu-b; Backpack = fullscreen (backpack reports); Settings = settings-landscape report (right sheet in that file).
- Discover drawer (277/278/282/276): scrolls vertically. Kebab (752,103) -> dropdown (284/288/290) -> Share (-> native share sheet 285), Report Content (-> external Google form 291), Report bug (-> portrait Bug Report modal 289). Featured card tap -> place modal (281) -> JUMP IN (teleport, closes menu). Event card tap -> event modal (283 live, 286/287 upcoming) -> JUMP IN / REMIND ME toggle. EXPLORE MORE -> fullscreen portrait Discover (279) (orientation flips to portrait; back chevron returns to the world/landscape).
- Social drawer: tabs FRIENDS / REQUESTS / NEARBY. Jump-in button -> teleport to friend; check/X -> accept/decline; Add friend -> send request.
- Dismiss: tapping the world area right of the panel (x>832) presumably closes the drawer (not drawn); modals close with X (1300,160).
- HUD tutorial overlays (273, 274) appear once on first world entry: dim + coach marks; 273 then 274 (next step) then HUD 275 (assumed order).

## 3. Per-screen spec (Figma px, 1600x720)

### 3.1 HUD tutorial overlays (273, 274) and HUD (275)
- Final HUD (275, matches `loading-hud` report): menu avatar button 82 circle with 3px lavender ring at (150,72) (the older 273/274 versions place it top-right at (1502,78) with the pill "Menu" below it - design drift, use 275). Chat button 66 dark disc at (256,73). Chat rows: avatar-less lines, 12px radius-5 black 40% rows, name color orange `#FF7439` + "#1l0u" tag in white 60%, text white Inter Medium ~22; rows 38px pitch; latest message wraps. Emote button 80 at (148,633). Gamepad: big jump 160 at (1410,591), hand 76 at (1245,633), E 80 at (1256,524), F 80 at (1342,438), "+" 80 at (1457,426).
- Coach marks (273): purple-dark (#350854 at ~85%) rounded pills (radius 12), Inter Medium ~26 white text, small notch pointing to target: "Chat", "Menu", "Character movement", "Emotes", "Controls". Whole scene dimmed ~55% with dark-red overlay (scrim); targets stay lit.
- 274: pill "Switch camera" at (790,355) + animated hand-with-curved-arrows illustration (lavender stroke, 300px) at right: a swipe/rotate gesture hint (look-around drag area is the right half).
- Data: local flag "tutorial seen"; no network.

### 3.2 Menu drawer shell (276-278, 280, 282, 284, 288, 290, 292-299)
- Rail: x 108-208 (100 wide), y 32-672, fill ~#24033B at 90% (`purple550`), left corners radius ~24; icons 48, stroke style, white (inactive) / lavender #E8B9FF filled (active). Profile avatar 46 circle at bottom with white 2px border.
- Panel: x 208-832, y 32-672 (624x640), gradient #691FA9 -> #460F6F (`purple350`->`purple450`, diagonal), right corners radius ~24. Content padding-left 49 (text x=257), right gutter ~48. Panel is clipped at x=832 (cards run into the edge: horizontal carousels bleed off the right side).
- Background: live world (no blur) outside the drawer; gamepad (right) remains visible and interactive; HUD chat/avatar are hidden while open.

### 3.3 Discover drawer (277, 278, 280, 276, 282)
- Header: title Inter Bold 40 white, single line ellipsis ("Genesis Plaza PlazaPl...", width ~430), subtitle "by Lazaro" Inter Regular 26 white 80%; kebab = 3 vertical lavender dots (#E8B9FF, 8px) at (752,103) 64px hit area.
- Section titles "Featured", "Events": Inter SemiBold 32 white, x=257, y=203 / 558.
- Featured card: image 294x156 radius 8 with 1px lavender-translucent border; title Inter SemiBold 26 ("Genesis Plaza"), meta row: person icon + "000" (users online) + thumbs-up icon + "100%" (like ratio), Inter Regular 24, white 90%. Cards 32px apart, horizontally scrolling (next card cut at the panel edge).
- Event card: image 406x240 radius 8; badges top-left: "LIVE" ruby (#FF2D55) pill radius 6 + black pill with green dot + person icon + "000"; top-right "Trending" mint-green (#34CE76) pill; upcoming: white pill with ruby text "IN 5 MINS" / "IN 30 MINS"; top-right "Featured" orange-to-magenta gradient pill. Title 2 lines max ("Experience Name two lines..." ellipsis) Inter SemiBold 28; "by Lazaro" 26 white 70%.
- EXPLORE MORE: 524x74 outlined button, radius 14, 2px white border, fill dark purple #24033B-ish, label Inter Bold 28 uppercase, centered; Opens fullscreen Discover (279).
- 282 variant: when the scene has no featured place the drawer shows only Events (title y=200, cards at y=238). 276 is just 277 scrolled down by ~443px.
- Data: header = current scene/world title + creator (places API by parcel/realm, cached per scene); Featured = places API featured; Events = events API (live first, then upcoming); card likes % and online counts from the places API. Skeleton/error states are not designed (legacy retries failed carousels on next open).

### 3.4 Kebab dropdown (284, 288, 290)
Popover 318x246 at (466,141), fill #1B0030-ish (deep purple, `purple550` darker), radius 12, 1px lavender 30% border, 3 rows of 72: icon 32 + label Inter Medium 30: "Share" (share-nodes icon), "Report Content" (flag), "Report bug" (bug). Tap outside dismisses. 284/288/290 are the same frame (three entry points: Share, Report Content, Report bug each starts from here).

### 3.5 Place / Event modals (281, 283, 286, 287)
- Modal 1060x460 at (270,130), radius ~20, overflow hidden, centered on a blurred (~12px) + dimmed world with HUD ghosts (avatar, chat) still visible at top-left.
- Left 600x460: cover image (grey #D9D9D9 placeholder in Figma). Badges inset 16px: event live = LIVE + count at top-left and Featured at top-right; upcoming = "IN 5 MINS" outlined pill (white fill, ruby text/border) at top-left.
- Right 460x460, gradient purple (#4F1580 -> #8F2BAE radial glow at bottom), padding 40: X close at (1300,160) 32px; title Inter Bold 40 white (place: 2 lines "Name experiencia / Name experiencia"; event: "Name event"), "By Creator" Inter Regular 28 white 85%, CTA at bottom 380x74 (x 910-1290, y 456-530) radius 14.
- CTA states: place + live event: ruby fill "JUMP IN" + arrow-in-box icon; upcoming event: outlined white "REMIND ME" with bell icon; after tap: ruby-outlined "REMINDER SET" with ruby check (text/border ruby `#FF2D55`/pink). Legacy locale keys: EVENTS_REMIND_ME, EVENTS_REMINDER_SET, EVENTS_SIGN_IN_TO_USE_REMINDERS (guests see "SIGN IN TO USE REMINDERS").
- Not designed: description, schedule/date, attendee count, likes, location coordinates, share button inside the modal, loading skeleton for the image. Legacy event_details has description + time.
- Data: places API (`places.decentraland.org`), events API (`events.decentraland.org/api/events/<uuid>`), reminders via events API with signed fetch; teleport via bridge `teleport` / `enter_world`.

### 3.6 Share (285)
Native OS share sheet. RN: `Share.share({ message })` where message = `tr("SHARE_SCENE_MESSAGE")` = "📍 Join Me At {scene} following this link: {link}"; link = `https://mobile.dclexplorer.com/open?position=x,y` (Genesis City) or `...?realm=<short realm>` (worlds). Events use SHARE_EVENT_MESSAGE. Nothing custom to build; Android shows the OEM sheet shown.

### 3.7 Bug Report modal (289)
Portrait 720x1600; modal 624 wide at x 48, y 222-1342, radius 20, same purple gradient as modals, bug emoji icon 70 overlapping top edge, title "Bug Report" Inter Bold 40 centered. Fields: "Issue Type" (red * required) dropdown 512x68 fill #350854-dark with "Select" placeholder + chevron; "Description" (required) multiline 512x216, placeholder "Briefly describe what happened.", counter "0/300" Inter Regular 24 left under it; "Screenshot": 2 slots 170x170 radius 12 - first filled (auto-captured world screenshot, white 2px border, X remove badge top-right), second "+" add slot (dark fill, lavender plus); buttons: CANCEL (dark purple fill, white text) and SUBMIT (magenta #A52068, text 50% opacity = disabled until type+description valid) each 240x74 radius 12. Background is the portrait Discover, blurred: the form forces portrait while open (legacy behaviour) and returns to landscape on close.
Legacy specifics: issue categories Performance, Crash/Freeze, Chat, Streaming, Friends, Wearables/Emotes, Profile, Other; max 3 screenshots, 3 MB, JPEG q0.85; POST to Intercom proxy `/intercom/tickets`; success modal + failure toast. Screenshot captured when the menu/settings opens (`BugReportCapture`). RN needs the bridge to hand over a world JPEG (`capture_screenshot`) since Godot renders the world.

### 3.8 Report content (291)
Not an RN UI design: a screenshot of the external Google Form in an iOS in-app browser. Legacy `ReportContentHelper.open_form()` opens `https://docs.google.com/forms/d/e/<FORM_ID>/viewform` (with scene/location prefill). RN: `Linking.openURL` / in-app browser, no screen to build. Title: "Decentraland: Report Content Policy Violation".

### 3.9 Social drawer (292-299)
- Title "Social" Inter Bold 44 at (257,92). Tab bar y 163: FRIENDS / REQUESTS / NEARBY, Inter SemiBold 30 uppercase; active = white + 4px ruby underline (width of label +/-4, y 191); inactive = white 70%. Counter next to label: "10" in Inter Regular 26 white 70% on NEARBY (absent when 0/empty: 298 label shifts). REQUESTS carries a ruby 20px unread dot at top-right of the label. Existing `ds/Tabs.tsx` already matches the underline style (add badge + count).
- Collapsible section header: chevron-up 24 + uppercase Inter SemiBold 26 "ONLINE (2)", "OFFLINE (2)", "BLOCKED (2)", "RECIEVED (1)" (typo in Figma; should be RECEIVED), "SENT (1)". Tapping toggles collapse (chevron flips). Counts in parentheses (note 293 shows (1) with 2 rows: placeholder data).
- Row (height ~108, pitch 108): avatar 60 circle with colored backdrop at x=257; name Inter SemiBold 30 in cyan (#73D3D3, name color derived from the avatar's name hash; claimed names get a ruby/pink verified badge 22 after the name), subtitle Inter Regular 24 white 80% = current place ("Place Name") or "Same Place" when in the viewer's scene; offline rows show last place / nothing. Tapping row = open profile (legacy).
- Row actions at right edge (x 730-784): jump-in 54x54 ruby square radius 8 with arrow-in-box icon (disabled = 40% opacity when friend is in the same place; absent for offline); requests: received = accept (outlined 54 square, check) + decline (outlined square, X); pressed state accept = green (#34CE76) border + icon, decline = ruby border + icon; sent = single X (cancel). Nearby: ADD FRIEND ruby button 230x54 radius 8, person-plus icon + Inter Bold 26 uppercase.
- Empty states: 120px rounded-square outline icon (people+heart) at (520,340), title Inter SemiBold 32 ("No friends yet" / "No one nearby"), body Inter Regular 24 white 70% 2 lines centered.
- Data (Rust social service, `Global.social_service` in legacy): friends/online status + current place; pending/sent requests; accept/reject/cancel; nearby = avatars in scene from comms; unread badge via `friends_request_size_changed`. RN needs bridge access to these (none exists today).
- Missing states: loading skeleton, error/"social service unavailable", guest users (legacy hides friend actions for guests), BLOCKED section rows (unblock button), confirmation of remove friend, search.

## 4. Reusable components / tokens

- Present in `rn/src/ds`: colors purple350/450/500/550, ruby, lavender, orange, glass, scrim, white10/30/70, `radii`, Inter font set, `AppText`, `Icon`+`iconSvgs`, `IconButton`, `PrimaryButton`, `Pill`, `Tabs` (underline), `ItemCard`, `Gradient`.
- New for this area: `DrawerShell` (rail + gradient panel + safe-area-aware position), `RailButton` (with unread dot), `KebabMenu` popover, `PlaceCard`/`EventCard` (rn/src/discover/Cards.tsx likely reusable - verify), `StatusPill` (LIVE, user-count, Trending, Featured, IN X MINS), `ModalSplit` (image + info + CTA), `OutlinedButton`, `CollapsibleSection`, `FriendRow`, `ActionSquareButton` (jump/accept/decline/cancel with pressed colors), `EmptyState`, `CoachMark` + scrim overlay, `BugReportForm` pieces (Dropdown, TextArea with counter, ScreenshotSlot). Missing icons: compass, people-heart, bell, backpack, gear (filled+outline), share-nodes, flag, bug, bell-outline, check, X, chevrons, person-plus, person-count, thumbs-up, hand, emote, chat.
- Token additions: pill greens (#34CE76 exists as rarity rare.dark), cyan name color #73D3D3 (rarity common.dark), verified badge pink, modal gradient stops #4F1580 -> #8F2BAE, rail fill.

## 5. Legacy counterpart

- Drawer/rail: `godot/src/ui/components/organisms/navbar/navbar.gd` (in-world dropdown navbar, landscape only; Discover/Friends/Notifications = side panels, Backpack/Settings = fullscreen Menu screens) and `organisms/menu/menu.gd`.
- Discover drawer: `godot/src/ui/pages/discover/discover_panel/discover_panel.gd/.tscn` (header, featured + events carousels, EXPLORE MORE, kebab with Share / Report Content / Report bug; share via `explorer.gd::_share_place()` -> `DclAndroidPlugin.share_text` / `DclIosPlugin.share_text`, URLs `https://mobile.dclexplorer.com/open?position=` or `?realm=`; locale SHARE_SCENE_MESSAGE).
- Fullscreen Discover: `pages/discover/discover.gd/.tscn`; place card: `components/molecules/place_item/place_item.gd` (share logic too); event modal: `pages/events/event_details*.tscn`, `button_reminder.gd/.tscn`, `events_generator.gd`; jump-in: `pages/discover/jump_in`, `organisms/jump_in_popup`.
- Bug report: `organisms/bug_report/bug_report_modal.gd` + `bug_report/` services; report content: `components/utils/report_content_helper.gd`.
- Social: `pages/friends/friends_panel.gd/.tscn`, `organisms/social/social_list.gd`, `social_item/social_item.gd`, `social_item_data.gd`; Rust `Global.social_service`.
- HUD: `godot/src/ui/explorer.gd`, `organisms/joypad/`, `organisms/chatbar`, `pages/chat`. No legacy first-run coach-mark/tutorial for "Chat / Menu / Character movement / Emotes / Controls / Switch camera" was found (no such locale strings in `godot/locale/en.csv`): NEW feature.
- Behaviours legacy has that Figma omits: friends reconcile timer 15 s, guest gating of friend actions, Blocked section with unblock, social badges, mutual friends, "Same Place" detection, event description/time, IAP/credits button in Discover, guest upgrade card, search.

## 6. RN status and gap list

Exists: `rn/src/discover/*` (DiscoverScreen, Cards, DetailSheet (place detail sheet), api.ts, useDiscover, history), `world/WorldScreen.tsx` (minimal HUD: chat toggle + log + input, spike buttons), `ds/*`, bridge events `ready`, `loading`, `state`, `chat`, `avatar`, `world`. Nothing for drawer/rail, social, reminders, bug report, share, tutorial overlays.
Gaps: (1) in-world drawer host in WorldScreen with rail + panels and Godot touch pass-through contract (`set_hud_insets`, drawer bounds); (2) Discover drawer with current scene header + featured/events carousels (reuse `discover/api.ts`; need "current scene" place lookup from `state` event parcel/realm); (3) kebab + share (`Share.share`) + bug report modal (needs bridge `capture_screenshot`, image picker, Intercom POST, orientation lock) + report-content link; (4) place/event modals with reminder (events API with signed fetch, needs identity from Rust/bridge); (5) social service bridge (friends, requests, nearby, actions, push events) and the whole Social drawer; (6) fullscreen portrait Discover opened from the landscape drawer (orientation switching under RN host control, `Global.set_orientation_*` follows host); (7) HUD top-left avatar button + chat button per 275 (see loading-hud gaps); (8) first-run tutorial overlays + persisted flag; (9) unread badges on rail (notifications, requests).

## 7. Open questions / inconsistencies

1. HUD avatar/menu button is top-right in 273/274 but top-left in 275 (and Settings rail is on the right). Assume 275 is current.
2. Order and trigger of the two tutorial overlays (273 then 274?) and whether they replay; "Controls" label vs gamepad meaning; what "Switch camera" gesture is (the hand glyph is ambiguous: swipe vs two-finger tap; legacy has no such tutorial).
3. 277, 278, 280 are identical; 284, 288, 290 identical (kebab open); 297 differs from 296 only by a missing tab underline (probably a design-time leftover).
4. 289 is named "Settings-Account" but is the Bug Report modal; 291 is an external web form screenshot (decide: keep Google Form or build a native form + backend, per legacy-godot report).
5. Typos in Figma copy: "RECIEVED"; "Name experiencia" (placeholder).
6. Place modal lacks description/likes/coordinates; event modal lacks date/time and description; text truncation rules for 2-line titles.
7. Drawer panel is 724x640 fixed in a 1600x720 frame; how it adapts to taller/wider screens and safe areas (notch) is unspecified. Legacy HUD margins (left 108 / top 32 px) match the drawer origin (108, 32).
8. Whether tapping the world (right of the panel) closes the drawer, and whether gamepad stays live while the drawer is open (design shows it visible).
9. Guest/signed-out behaviour for Social, reminders and Share is not designed.
10. Share sheet is OEM UI (Samsung One UI in the capture); nothing to implement beyond the message string.
