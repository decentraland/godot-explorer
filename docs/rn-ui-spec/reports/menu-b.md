# Report: menu-b (manifest seq 300-326)

Source: Figma file `nxP78pUCEHj3BY0meiXipe`, page `2:60` ("menu"). All 27 screenshots captured and viewed (`docs/rn-ui-spec/shots/menu/`). Frames 300-314 are 1600x720 landscape; 315-326 are fragments and banner strips. Strips were downscaled on disk (3200 px wide) and cropped with PIL to read them. `get_design_context` was NOT called (shared Figma quota); every value is measured from the PNGs (Figma px, 1600x720) plus `rn/src/ds/tokens.ts`. Companion: `menu-a.md` (seq 273-299: HUD, Discover, kebab/modals, Social friends/requests/nearby). Related: `settings-landscape.md` (older right-sheet settings design), `settings-portrait.md` (settings keys/data), `backpack-a.md`/`backpack-b.md`.

Important finding: the task brief says "Settings tabs in-HUD", but in this page Settings, Account, Backpack and Profile are NOT drawer panels. They are fullscreen landscape pages (back chevron at top-left, no icon rail). Only Social (300-302) and Notifications (303) use the drawer shell from menu-a (rail x108-208 + panel x208-832, y32-672).

## 1. Screen inventory

| seq | id (suffix) | name | what it shows | state / variant |
|---|---|---|---|---|
| 300 | social__1135-16597 | Social | Drawer, FRIENDS tab selected, empty state: people+heart icon in rounded square, "No friends yet", "Meet people nearby and add them as friends."; REQUESTS has ruby unread dot, NEARBY shows "10"; rail Social active, bell has ruby dot | friends empty (counts from other tabs visible) |
| 301 | social__933-26017 | Social | FRIENDS tab, three collapsed section headers "ONLINE (2)", "OFFLINE (2)", "BLOCKED (2)" with chevron-down | all sections collapsed |
| 302 | social__933-26834 | Social | FRIENDS tab: ONLINE (2) expanded with one row (PlayerName + verified badge, "Place Name", ruby jump-in square), OFFLINE (2) expanded with one row (same, no action), BLOCKED (0) collapsed | sections expanded (Figma shows 1 row for a count of 2: placeholder data) |
| 303 | notifications__426-6356 | Notifications | Drawer, Notifications active (filled bell, ruby dot). Title "Notifications" + READ ALL outlined button. List of 4 cards: friend request accepted, friend wants to be your friend, "New Item Received" wearable, "Event Name / Event started!" (cut off at panel bottom) | populated, scrollable |
| 304 | backpack-wearable__533-5322 | Backpack-Wearable | Fullscreen landscape Backpack: back chevron, pills Wearables (active, ruby) / Emotes, filter button; avatar (Godot) left; tabs BODY (active) HEAD ACCESSORIES TORSO LEGS; sub-pills Body shape (active, lavender) / Skin x4; swatch row (rainbow wheel + skin tones, 2nd ringed); 3 cards, middle equipped with UNEQUIP strip | wearables, body category |
| 305 | backpack-emotes__609-3439 | Backpack-Emotes | Same header with Emotes active; vertical emote-slot column (x851-989, 138px tiles pitch 158), 2-column emote grid (200px tiles, orange border + UNEQUIP strip), scrolls | emotes, no slot selected |
| 306 | settings-graphics__525-3344 | Settings-Graphics | Fullscreen Settings: back chevron, left list (Graphics active ... Storage), "Version 1.14.0.1844" + copy icon, right content PERFORMANCE: Dynamic Graphics switch ON, "Graphic Profiles" dropdown dimmed "High" + dimmed hint; VISUAL: Dynamic Skybox switch ON + hint | Dynamic ON => dependent dropdown disabled |
| 307 | settings-audio__725-6881 | Settings-Audio | VOLUME header; sliders General 100%, Scene 100%, UI SFX 100%, Music/Ambient 100% (cut) | default, scrolls |
| 308 | settings-gameplay__725-7280 | Settings-Gameplay | CHAT & SOCIAL: "Close Chat After Sending" OFF; CAMERA: "Camera Mode" dropdown "First person"; INTERFACE: "Hide Player Names" ON (more below) | default, scrolls |
| 309 | settings-account-guest__822-5962 | Settings-Account-Guest | Account tab: avatar (Godot) center, name "Namemaxcharacte" #1234, "0x4e1...1c13B", card "Unlock the full experience" + ADD EMAIL, SIGN OUT outlined, "Do you want to Delete your account?" | guest/no email yet |
| 310 | settings-help__740-4709 | Settings-Help | SUPPORT: Report Content, Help (external); COMMUNITY: Discord (external); LEGAL & POLICIES: Privacy Policy, Content Policy, Term Of Service (cut) | default, scrolls |
| 311 | settings-storage__740-4980 | Settings-Storage | TEMPORARY FILES: "Current Cache Size 0.5 GB" progress bar (50%), "Max Cache Size" dropdown "1 GB" + hint, CLEAN CACHE outlined button + caption | default |
| 312 | settings-language__1019-11914 | Settings-Language | "App Language" dropdown "English" + hint | default |
| 313 | settings-account-upgraded__823-6143 | Settings-Account-Upgraded | Same as 309 but replaces the unlock card by "Email" label + read-only field "name@email.com"; left list has NO Language row (6 rows) | account with email linked |
| 314 | profile__533-5757 | Profile | Fullscreen Profile: avatar left, name+tag, address + copy, EDIT PROFILE, ABOUT ME paragraph + SEE MORE, EQUIPPED row of 5 item cards | own profile, default |
| 315 | hand-fill-only__192-2116 | Hand - Fill Only | 45x61 near-black hand/pointer glyph on transparent (fill-only vector; the "hand" button glyph on the gamepad and the tutorial hand) | icon asset |
| 316 | buttons__889-4519 | Buttons | Dark banner labelled "HUD" (+ ghost bar below) | Figma section label, not UI |
| 317 | buttons__889-4060 | Buttons | Banner "DISCOVER" | section label |
| 318 | buttons__889-4522 | Buttons | Banner "SOCIAL" | section label |
| 319 | buttons__889-6533 | Buttons | Banner "NOTIFICATION" | section label |
| 320 | notifications__891-8628 | Notifications | 528x130 notification card: periwinkle "PlayerName", "Accept your friend request", "2 days ago", avatar 80 circle | friend-accepted card |
| 321 | notifications__891-6976 | Notifications | Same, body "Want to be your friend!" | friend-request card |
| 322 | notifications__891-6977 | Notifications | Card, title "Event Name" in mint green, body "Event started!", 80px rounded-square event image with 1px border | event card |
| 323 | notifications__891-6978 | Notifications | Card, title "New Item Received" in magenta-lavender, body "[Name wearables]", 80px square wearable thumb | reward card |
| 324 | buttons__889-6535 | Buttons | Banner "BACKPACK" | section label |
| 325 | buttons__889-6537 | Buttons | Banner "SETTINGS" | section label |
| 326 | buttons__889-6539 | Buttons | Banner "PROFILE" | section label |

Strips 316-319 and 324-326 are 6576-12466 px wide x 128 px: a dark #121212 bar with an uppercase Inter label at left and a second shorter ghost bar. They are section dividers of the Figma board (what row of screens follows), not components. Implement nothing; use them only as evidence of the page structure: HUD | DISCOVER | SOCIAL | NOTIFICATION | BACKPACK | SETTINGS | PROFILE (seven top-level menu destinations, same order as the nav rail minus the profile avatar).

## 2. Flow graph

- World HUD -> avatar button -> Menu. Rail (menu-a 3.2) order: Discover, Social, Notifications, Backpack, Settings, Profile avatar. In 300-303 Social and Notifications are the only drawer panels; tapping Backpack/Settings/Profile rail items leaves the drawer and opens a fullscreen page (304-314) with the back chevron (132,60).
- Social (300-302): tabs FRIENDS / REQUESTS / NEARBY (REQUESTS and NEARBY detailed in menu-a). Section header tap toggles collapse (chevron down = collapsed, up = expanded). Row tap -> that user's profile (legacy; no profile-of-other design in this range). Jump-in square -> teleport and close menu.
- Notifications (303): READ ALL -> marks all read (button disabled when 0 unread, legacy). Card tap -> mark read + navigate by type: friend request/accepted -> Social REQUESTS/FRIENDS; event -> event modal / jump in (menu-a 3.5); reward -> Backpack. Pull/scroll for more.
- Backpack (304/305): back chevron -> returns to world (legacy "back to explorer"). Wearables/Emotes pills toggle mode; filter button opens the dropdown (backpack-b seq 152-154). Same screen as backpack reports; this is the landscape variant (see backpack-a seq 108, backpack-b seq 169-175).
- Settings (306-313): left list = master, right content = detail, single page; tapping a left row swaps the right column (no navigation stack). Back chevron -> world. Help rows: Report Content / Help / Discord / policies open external URLs (`Linking.openURL`), Report Content may instead open the menu-a bug/report flow (see Open questions). Version row copy icon -> clipboard + toast. ADD EMAIL -> email OTP flow (legacy `upgrade_otp_popup`, `input_modal`), SIGN OUT -> sign out and return to entry flow, Delete account link -> `account_deletion_popup`.
- Profile (314): back chevron -> world. EDIT PROFILE -> profile editor (not drawn here; legacy `profile_editor`). Copy icon -> clipboard + toast. SEE MORE expands ABOUT ME. Equipped card tap -> item detail (not designed; legacy `profile_equipped_item`).

## 3. Per-screen spec (Figma px, 1600x720)

### 3.0 Shared drawer shell (300-303)
As in menu-a 3.2: rail x108-208, panel x208-832, y32-672 (624 x 640), radius 24, panel gradient #691FA9 -> #460F6F, content left x=257, right edge x=784 (padding 49/48). Rail icons 48: Discover (y103), Social (205), Notifications (305, ruby 20px unread dot at top-right of the bell), Backpack (408), Settings (509), profile avatar 46 circle with 2px white border (605). Active item: filled lavender #E8B9FF icon (Social in 300-302, Notifications in 303), faint vertical glow on the seam; inactive: white outline icons. In 300-303 the HUD chat/avatar buttons are hidden but the gamepad (F, +, E, hand, jump) stays visible over the world on the right.

### 3.1 Social, friends (300, 301, 302)
- Title "Social" Inter Bold 44 white at (257,92). Tabs y163 (underline y191): "FRIENDS" (active: white + 4px ruby underline spanning label, x256-383), "REQUESTS" (white 70%, ruby 20px unread dot at top-right of label (575,150)), "NEARBY" + count "10" (count in Regular 26, white 70%). Inter SemiBold 30 uppercase. Tab x: 261, 424, 631.
- Section header row (height ~61, y240/301/362 when collapsed): chevron 24 (down collapsed, up expanded) at x265, label uppercase Inter SemiBold 26 white "ONLINE (2)" at x283. When expanded, a row of height ~108 follows (302: header 240 -> row centre 314 -> next header 401, so header-to-header = 161 with one row).
- Friend row: avatar 60 circle (x257-315, colored backdrop), name Inter SemiBold ~28 in per-user color (cyan #73D3D3 here) + pink verified badge 22 for claimed names; subtitle "Place Name" Inter Regular 24 white 80%. Online row: jump-in square button 54x54 at (730-784), ruby fill #FF2D55 radius 8, arrow-into-box icon white (disabled/dim when same place, see menu-a). Offline row: same layout, no button, not visibly dimmed (design leaves last-seen text undefined).
- BLOCKED (0) collapsed with count zero: header still shown (so section headers appear even at 0; menu-a empty state is only when there are no friends in any section). Blocked rows (unblock action) are not drawn in this range.
- Empty state (300): 120px rounded-square outline (radius ~24, 3px lavender-ish border) holding a people+heart glyph at (520,340); title "No friends yet" Inter SemiBold 32 white (y437); body "Meet people nearby and add them as friends." Inter Regular 24 white 80%, 2 centered lines (y477/506). Centered in panel (x520).
- Data: Rust social service (friends with online status/current place, pending counts, nearby from comms). Legacy: `Global.social_service`, `friends_request_size_changed` for the REQUESTS dot. Unread/rail dot data from `NotificationsManager` plus pending friend requests.
- States to build: loading skeleton (not designed), empty (300), populated (302), all collapsed (301), error ("social service unavailable", not designed), guest (hide friend actions).

### 3.2 Notifications drawer (303, 320-323)
- Title "Notifications" Inter Bold 44 at (257,92); READ ALL outlined button 160x52 at (625-783, 65-117): 2px white border, radius 8, transparent/dark fill, label Inter Bold 24 white uppercase (locale `NOTIFICATIONS_READ_ALL`). Disabled when 0 unread (legacy).
- List y156 onward, card 528x130 (x256-784), pitch 146 (gap 16), radius ~16, fill near-black translucent (#121212 in the fragment frames; reads as dark purple over the panel in 303). List scrolls under the panel bottom edge (4th card clipped at y672); no scrollbar.
- Card layout (padding 24): leading image 80x80 at x278 (person avatar = circle with periwinkle background; event/reward = rounded square radius 8, 1px light border); text column x382: title Inter SemiBold 28 (colored by type), body Inter Regular 26 white, timestamp Inter Regular 20 white 50% ("2 days ago").
- Title colors (match legacy `NotificationTextHelper`): friend request/accepted `#A0ABFF` (sender nickname), events `#73FFAF` (event name), reward/item `#DF9CFF` ("New Item Received"), others white.
- Copy in frames: "PlayerName / Accept your friend request / 2 days ago" (legacy key `NOTIF_BODY_ACCEPT_YOUR_FRIEND_REQUEST` = "Accepted your friend request": Figma copy differs), "PlayerName / Want to be your friend! / 2 days ago" (`NOTIF_BODY_WANT_TO_BE_YOUR_FRIEND`), "New Item Received / [Name wearables] / 2 days ago" (wearable name as body), "Event Name / Event started! / 2 days ago" (`NOTIF_BODY_EVENT_STARTED`).
- No unread/read visual difference is designed (legacy shows unread highlight + mark-as-read tap). No empty ("No notifications yet" / "Check back later for updates!"), guest ("Sign in to get notifications!") or error state is drawn; legacy has them (`NOTIFICATIONS_NO_NOTIFICATIONS_YET`, `NOTIFICATIONS_SIGN_IN_TO_GET_NOTIFICATIONS`).
- Relative time strings are localized (`NOTIFICATION_*` keys: Now, {m}m ago, {h}h ago, Yesterday, {days} day ago, weeks, months, years).
- Data: Decentraland notifications API polled by `NotificationsManager` (not in RN; needs signed fetch with identity from the bridge, or a Rust-bridge event stream `notifications`). About 40 notification types in legacy `notification_text_helper.gd` (friend, community, badge, marketplace/credits, governance, land, worlds, events, rewards); the Figma shows only 4 families, the rest need a generic white-title fallback.

### 3.3 Backpack landscape (304, 305)
Fullscreen purple (#7B27B8 top-left glow to #480670), no rail. Detailed behaviour, data and tokens live in backpack-a/backpack-b; only the landscape layout is captured here.
- Header: back chevron (132,60); pills y33-93 radius 30: "Wearables" with t-shirt icon (active ruby #FF2D55 fill, x238-427) and "Emotes" with person icon (inactive dark purple #2B0847 fill, x447-601); filter button 60x60 radius 8 dark at (639-699) with sliders icon (shows a ruby dot when filter active, backpack-b 154).
- Left: Godot avatar stage with radial lavender floor glow, avatar x330-505, y150-650 (RN must leave a transparent hole; AvatarStage exists).
- 304 right panel (x848-1490): tabs BODY / HEAD / ACCESSORIES / TORSO / LEGS Inter SemiBold ~28 uppercase at y56, active white + ruby 4px underline, inactive white 70%. Sub-pills y118-174 radius 28: active "Body shape" lavender #E8B9FF fill + dark text, inactive "Skin" translucent purple + light text (Inter SemiBold 26). Swatches y210-290 (80px circles, pitch 110; first = rainbow wheel, equipped swatch ringed ruby+white 3px). Item cards y328-520, 192x192 each, gap 32 (x848, 1072, 1296), 1px white-ish border radius 8; equipped card = 4px orange (#FF7439) border + black strip "UNEQUIP" Inter Bold 24 white.
- 305 emotes: slot column x851-989 (5+ slots 138x138, pitch 158, light gradient tiles with black silhouette, scrolls vertically), emote grid x1037-1461: 2 columns of 200x200 tiles, pitch 224, ALL shown equipped (orange border + UNEQUIP). Note this is a different layout from backpack-a seq 108 (3-col grid x720-1440, 112px slot column): same screen, two generations.
- Data/state: see backpack reports; RN already has `rn/src/backpack/*` (portrait only).

### 3.4 Settings fullscreen (306-313)
Shared shell:
- Background: same purple gradient as Backpack (top-left #7B27B8 -> #460F6F/#480670), no panel, no blur.
- Back chevron white 40 at (132,60).
- Left list x120-496 (376 wide), rows 60 tall, pitch 68, first row y116-176: Graphics, Audio, Gameplay, Language, Account, Help & Support, Storage. Row: label Inter Medium 32 white at x133 + chevron-right 28 (white, bold) at x468. Active row: fill white 25-30% (#8A4DB3-like) radius 8 and label Inter Bold. Version footer at (120,653): "Version 1.10.0" (306 shows "Version 1.14.0.1844") Inter Regular 26 lavender #E8B9FF + copy icon 28 (tap copies version).
- Right content x640-1490 (850 wide), scrolls vertically; sections: header = uppercase Inter Bold 22 lavender `#E8B9FF` + 1px lavender rule full width (header y134, rule y154); rows start ~y185. Row title Inter Medium 28 white; hint Inter Regular 22 white 70%; dimmed/disabled = 30-40% opacity.
- Switch: 134x60 pill at x1357-1490: ON = ruby #FF2D55 fill + white 48 thumb right; OFF = dark #2B0847 fill + white thumb left. Dropdown: full width 850x68, fill #2E0050 (dark purple), radius 12, value Inter Medium 30 white, chevron-down 40 at right (x1455). Slider: 850 wide track 40 tall radius 8 (ruby fill, dark remainder), thumb white 32x50 radius 8; value label "100%" right-aligned at row title line, Inter Regular 26. Outlined button (SIGN OUT, CLEAN CACHE): 850x74 (account 504x74) fill #2B0847, 2px white border, radius 14, label Inter Bold 28 uppercase white (SIGN OUT has a leading exit icon 32). Progress bar: same track as slider w/o thumb, ruby fill 50%.
- Settings list order drift: 306-312 have Graphics, Audio, Gameplay, Language, Account, Help & Support, Storage; 313 omits Language and moves Account to row 4 (see Open questions).

Per tab:
- Graphics (306): PERFORMANCE header; "Dynamic Graphics" switch (ON) + hint "Adjusts graphics automatically for optimal performance."; "Graphic Profiles" label + dropdown "High" + hint "Sets a graphics quality preset for your device." (all 3 dimmed because Dynamic is ON); VISUAL header; "Dynamic Skybox" switch ON + hint "The sky changes over time as you play." (below the fold: Custom Skybox dropdown + creator-locked warning per settings-landscape 259-262).
- Audio (307): VOLUME header; sliders "General", "Scene", "UI SFX", "Music/Ambient" (4th clipped; maybe "Voice Chat" below), value labels 100%. Slider rows pitch 158 (title y203, track y244-284).
- Gameplay (308): CHAT & SOCIAL header, "Close Chat After Sending" switch OFF (no hint); CAMERA header, "Camera Mode" label + dropdown "First person" (dimmed + creator warning when scene sets camera, settings-landscape 266); INTERFACE header, "Hide Player Names" switch ON, further rows below the fold (Hide Interface, Hide View Profile per settings-portrait).
- Language (312): "App Language" label (Inter Medium 30, no section header) + dropdown "English" + hint "Changes the Decentraland interface language. Experiences may use a different language depending on the creator." Dropdown options = locale list from `godot/locale`.
- Account (309 guest / 313 upgraded): center = Godot avatar (x650-850, y95-655, radial glow), right column x993-1497: name "Namemaxcharacte" Inter Bold 36 white + "#1234" tag Inter Regular 28 lavender 70% right-aligned (tag hidden for claimed names), shortened address "0x4e1...1c13B" Inter SemiBold 26 lavender (copy on tap). Guest (309): card x993-1497 y237-460, 1px lavender 40% border, radius 16, transparent fill: title "Unlock the full experience" Inter Bold 32, body "Add email to play across mobile and desktop, and claim an exclusive Wearable." Inter Regular 24 white, button ADD EMAIL ruby 453x54 radius 8 Inter Bold 24 white. Upgraded (313): label "Email" Inter Medium 30 + read-only field "name@email.com" 504x68, fill #2B0847 radius 8. Both: SIGN OUT 504x74 at y520-593; "Do you want to " white 70% + "Delete your account?" ruby underlined link Inter Regular 24 centered at y658.
- Help & Support (310): SUPPORT header; rows "Report Content" (internal action, no icon), "Help" + external-link icon; COMMUNITY: "Discord" + ext icon; LEGAL & POLICIES: "Privacy Policy", "Content Policy", "Term Of Service" (+ext; typo for "Terms of Service"). Row = Inter Medium 30 white, height 72, ext icon 28 lavender right after the label (not at the edge).
- Storage (311): TEMPORARY FILES header; "Current Cache Size" + right value "0.5 GB" (Inter Regular 28), bar 850x40 at 50%; "Max Cache Size" label + dropdown "1 GB" + hint "Sets how much storage the app can use."; CLEAN CACHE outlined button 850x74 (y552-626); caption "Clears downloaded files used to speed up loading. Safe to delete." Inter Regular 22 white 70% centered.
- Data source: `settings-portrait.md` key table (Godot `Global.get_config()`, `GraphicSettings`, `SettingsHelper`); needs bridge `getSettings`/`setSetting`. Account: bridge session/profile (name, address, email), sign-out, delete-account, upgrade OTP. Storage: bridge cache size + max + clear. Language: RN i18n + bridge set.

### 3.5 Profile (314)
- Back chevron (132,60). Avatar (Godot stage) x290-480, y95-655 with radial lavender glow, floor gradient.
- Header x640: name "Namemaxcharacte" Inter Bold 40 white + "#1234" Inter Regular 30 white 60% (tag hidden for claimed names); address row "0x4e1...1c13B" Inter SemiBold 26 lavender + copy icon 36 (822,141); EDIT PROFILE outlined button 208x60 at (1285-1493, 58-118): dark fill #2B0847, 1px white border radius 8, pencil icon + Inter Bold 22 uppercase.
- ABOUT ME: header uppercase Inter Bold 22 lavender at y206; body Inter Regular 26 white, line height 34, 3 visible lines ("Visit my personal web. I love building and making friends in Decentraland. When I'm not decorating LAND, I like to play games and hang out in the districts!"), then "SEE MORE" Inter Bold 20 lavender underlined right-aligned at (1447,351) (legacy: compact mode toggles).
- EQUIPPED: header at y400; horizontal row of cards x639, 813, 987, 1161, 1335 (150 wide, pitch 174), y428-646: 150x150 square image with 1px light border radius 8 on a rarity gradient (base grey/white, legendary purple #A14BF3, unique gold #FEA217 with pattern, common teal #73D3D3), tiny category icon on a rarity-colored top-left triangle, bottom plate dark #1B0030 with name Inter Medium 18 white ("Asset Name") and a rarity pill (BASE, LEGENDARY, UNIQUE, COMMON; Inter Bold 14 uppercase, rarity-colored text on a rarity 25% fill). Last two are emotes (silhouette art). Row scrolls horizontally past x1490.
- Not designed: links section, other-user profile (mute/block/add friend), loading skeleton, empty About, copy toast, edit-profile screen. Legacy has all (see section 5).
- Data: own profile from the Rust profile (`DclUserProfile`: name, tag, address, description, links, equipped wearables/emotes) via bridge `profile`; item thumbnails from the asset-server/catalog (`rn/src/backpack/catalog.ts` already resolves thumbnails and rarity).

### 3.6 Fragments
- Hand fill (315): vector glyph (hand with raised index finger), 45x61, near-black fill; recolor to white/lavender in the gamepad "hand" button (menu-a 3.1 shows it at (1245,633), 76px button) and the tutorial. Add to `iconSvgs.ts` as `hand`.
- Notification cards 320-323: see 3.2 (the isolated 528x130 components; `bg #121212` is the component's own fill, so use `rgba(18,18,18,~0.6)` over the panel gradient to match 303).

## 4. Reusable components / tokens

Present in `rn/src/ds`: `colors` (ruby, orange, lavender, purple350/450/500/550, cardLight, white10/30/70, glass), `rarityColors`, `radii`, `fonts` (Inter set), `AppText`, `Icon`+`iconSvgs`, `IconButton`, `Pill`, `PrimaryButton`, `Tabs` (underline), `ItemCard`, `SlotTile`, `ColorPicker`/`SwatchRow`, `Gradient`.

New to build (shared with menu-a where marked): `DrawerShell`+`RailButton` (menu-a); `CollapsibleSection` + `FriendRow` + `EmptyState` (menu-a); `NotificationCard` (title color by type, relative time); `Switch` (134x60, ruby/dark), `Dropdown` (850x68 + option popover, settings-landscape 270), `Slider` (ruby fill, white thumb, % label), `ProgressBar`, `OutlinedButton` (2px white, #2B0847 fill; menu-a and settings share it), `SectionHeader` (lavender uppercase + rule), `SettingsRow` (title/hint/control, disabled state), `MasterList` (left list rows with chevron), `BackChevron`, `CopyField`, `RarityPill`/`ProfileItemCard`, `LinkText` (ruby underlined).
New tokens: rail unread dot ruby 20; field/dropdown fill ~#2E0050 and button fill #2B0847 (purple500 `#350854` at 70%+ is the nearest existing token); notification title colors #A0ABFF / #73FFAF / #DF9CFF; settings content width 850, left list 376; verified-badge pink.
New icons: compass, people-heart (filled+outline), bell (filled+outline, with dot), backpack (filled+outline), gear (filled+outline), chevron-left/right/up/down, external-link, copy, exit/sign-out, pencil, hand, check, arrow-into-box (jump in), person-plus, shirt, emote-person, sliders, speaker/monitor/gamepad (if tab icons are reused from settings-landscape), rarity category glyphs.

## 5. Legacy counterpart (Godot)

- Rail/drawer host: `godot/src/ui/components/organisms/navbar/navbar.gd` (+ `organisms/menu/menu.gd`, `menu.tscn`, `menu_profile_button.gd`, icons in `organisms/menu/icons/*.svg`), side panel base `organisms/panels/side_panel.gd`.
- Social: `godot/src/ui/pages/friends/friends_panel.gd/.tscn`, `organisms/social/social_list.gd`, `social_item/`, `mutual_friends.gd`.
- Notifications: `organisms/notifications/notifications_panel.gd/.tscn`, `notification_item.gd/.tscn`, `notification_content.gd`, `notification_text_helper.gd` (types, header colors, bodies), `notification_toast.gd` (in-game toast, not in Figma), `notification_bell_button.gd`. Legacy extras: per-item mark-as-read on tap, READ ALL disabled at zero unread, guest message, empty state, error handler, refresh on open.
- Backpack: `pages/backpack/backpack.gd`, `backpack_landscape.tscn`, `backpack_portrait.tscn`, `backpack_responsive.gd`, `avatar_preview.gd`.
- Settings: `pages/settings/settings.gd/.tscn`, `slider.gd`, `section_title.gd`, `settings_button.gd`; Account block: `organisms/menu/menu.gd`, `account_deletion_popup.gd`, `upgrade_otp_popup.gd`; email add uses `organisms/input_modal`; bug report `organisms/bug_report`.
- Profile: `pages/profile/profile.gd/.tscn` (+ `profile_about`, `profile_equipped*`, `profile_links*`, `profile_editor`), `organisms/profile_settings/profile_settings.gd`, `organisms/change_nick_popup`. Legacy behaviours not in Figma: links list, block/mute/add friend buttons for other users, mutual friends, nickname change popup, name claim, deploy debounce, avatar rotate-by-drag, emote tap to play.
- Locale keys available: `NOTIFICATIONS_*`, `NOTIF_BODY_*`, `NOTIF_HEADER_*`, `NOTIFICATION_*_AGO`, `COMMON_ADD_EMAIL`, `COMMON_UNLOCK_THE_FULL_EXPERIENCE`, `GUEST_UPGRADE_CARD_ADD_EMAIL_TO_PLAY_ACROSS_MOBILE`, `CUSTOM_BUTTON_SIGN_OUT`, `MENU_*` (delete account), `MODAL_ADD_EMAIL_*` in `godot/locale/en.csv`.

## 6. RN status and gap list

Exists in `rn/src`: `backpack/*` (portrait Backpack incl. AvatarStage, panels, filter, catalog), `discover/*`, `world/WorldScreen.tsx` (minimal HUD), `flow/*` (entry flow), `ds/*`, `GodotBridge.ts`/`events.ts` (events: `ready`, `loading`, `state`, `chat`, `avatar`, `world`). Nothing for notifications, social, settings, account, profile, drawer/rail, switch/dropdown/slider.

Gaps (in priority order):
1. Drawer host + rail + Godot touch contract (menu-a gap 1); Social friends panel and bridge (menu-a gap 5).
2. Notifications: bridge or direct API (signed fetch), card list with read/unread, READ ALL, rail/avatar dot, type-to-navigation mapping, empty/guest/error states (not designed).
3. Settings landscape page (master list + 7 tabs) plus `getSettings`/`setSetting` bridge, Switch/Dropdown/Slider/ProgressBar/OutlinedButton atoms, version copy, cache size/clear bridge, language switcher with RN i18n.
4. Account: avatar stage reuse, session/profile/email via bridge, ADD EMAIL OTP flow, sign-out, delete-account confirmation.
5. Profile screen (own): avatar stage, About, Equipped carousel (reuse `ItemCard`, `catalog.ts` thumbnails), edit profile and links (not designed).
6. Backpack landscape layout (rail slot column + 192px grid); Emotes landscape slot column.
7. Icon set additions (section 4), `hand` glyph.
8. Orientation: Settings/Backpack/Profile frames are landscape only here, while settings-portrait/backpack reports specify portrait; RN needs one responsive layout per screen (breakpoint on width, not orientation lock).

## 7. Open questions / inconsistencies

1. Settings drawer vs fullscreen: menu-a/settings-landscape draw Settings as a right sheet with a rail on the right; these frames draw it fullscreen with a master list. Which is current? (Assume these, they are newest in file order and share the Backpack/Profile fullscreen pattern.)
2. Settings list order/content: 306-312 include Language (row 4) and put Account at row 5; 313 (upgraded) has 6 rows without Language and Account at row 4. Version text drift: "1.14.0.1844" (306) vs "1.10.0" elsewhere (show the real `app_version`).
3. Graphics 306: Dynamic Graphics ON dims "Graphic Profiles"; legacy hides "Custom" profile in prod and disables the dropdown while dynamic is on (consistent). Skybox rows below the fold are in settings-landscape only.
4. Help tab lists "Report Content" without an external-link icon while "Help"/"Discord" have one: confirm it opens the report flow (menu-a 291 Google Form) in-app or externally. "Term Of Service" should read "Terms of Service".
5. Notification copy: Figma "Accept your friend request" vs legacy "Accepted your friend request"; wearable name as body vs legacy item text; Figma shows no read/unread difference nor a mark-as-read gesture.
6. Social: "BLOCKED (0)" still shows a header; "ONLINE (2)" with one row; offline rows show "Place Name" (legacy shows last seen/none); BLOCKED rows and unblock action undefined; jump-in square position (x730-784) overlaps no tab but the right gutter (784) differs from tab right edge (775).
7. Backpack 305 (2-col, 200px tiles) vs backpack-a seq 108 (3-col grid); two generations of the landscape design.
8. Profile equipped row: 4th card (emote) is 2px shorter and uses a smaller name size (18 vs 20): treat as noise. Emote cards show no equip slot number.
9. Strips 316-319/324-326 and hand 315 are not screens; the "Buttons" frame name is a misnomer (these are board section labels).
10. Account name field "Namemaxcharacte" is a truncated placeholder: define truncation (single line, ellipsis) and claimed-name verified badge placement (not shown here, shown on Social rows).
11. Safe-area/notch handling: all frames assume a 1600x720 canvas with 108px left margin matching menu-a; wider/taller devices unspecified.

> **SUPERSEDED (seq 306-313):** the landscape Settings frames in this report (Graphics, Audio, Gameplay, Account, Help, Storage, Language) are the OLD landscape design. The authority for settings, portrait and landscape, is `reports/settings-revamp.md` (Figma page "HI I revamp Setings", 1397:6493). Social, Notifications, Backpack and Profile frames in this report are unaffected.
