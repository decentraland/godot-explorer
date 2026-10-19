# settings-revamp report (manifest seq 327-342)

Source: Figma page "HI I revamp Setings" (newer). It REPLACES the older `settings-portrait` page (seq 220-258) and the
wrong `settings-landscape` source (`reports/_superseded/settings-landscape.md`). All 16 PNGs viewed; the 39 older
settings-portrait PNGs were also viewed (the old report was written without them). get_design_context NOT called;
tokens below are pixel-sampled from the PNGs (hex exact, sizes are Figma px).

## 1. Inventory

| seq | nodeId | name | shows | state |
|---|---|---|---|---|
| 327 | 1397:6501 | Settings-Graphics | detail: PERFORMANCE (Dynamic Graphics ON, Graphic Profiles "High" disabled) + VISUAL (Dynamic Skybox ON, Custom Skybox "Select" disabled) | default, mobile |
| 328 | 1397:6829 | Settings-Audio | VOLUME: 5 sliders all 100% | default |
| 329 | 1397:6961 | Settings-Gameplay | CHAT & SOCIAL + INTERFACE; Hide Interface OFF so 2 children disabled | default |
| 330 | 1397:7138 | Settings-Language | "App Language" dropdown = English | closed |
| 331 | 1401:1032 | Settings-Language | identical to 330 | duplicate |
| 332 | 1401:1082 | Settings-Language (dropdown open) | list English (checked, lavender) / Español / Português (Brasil); chevron up, border highlighted | dropdown open |
| 333 | 1401:4359 | Settings-Language | dropdown shows "Español" (helper copy still English) | after selection |
| 334 | 1397:6528 | Settings-Account (section menu) | ROOT list "Settings": Graphics, Audio, Gameplay, Language, Account, Help & Support, Storage; "Version 1.14.0" + copy icon | root list |
| 335 | 1401:1132 | Settings-Account (section menu) | same root list, "Version 1.10.0", slightly different text weight | root list (older iteration) |
| 336 | 1398:7315 | Settings-Account (guest upgrade) | name, #tag, address, "Unlock the full experience" card + ADD EMAIL, SIGN OUT, delete link | guest |
| 337 | 1398:7378 | Settings-Account (upgraded) | same header + "Email" read-only field "name@email.com"; no card | upgraded guest/email account |
| 338 | 1398:7818 | Settings-Help | "Device no longer supported" banner + SUPPORT / COMMUNITY / LEGAL & POLICIES | warning variant |
| 339 | 1398:8062 | Settings-Help (temporary files / dev tools) | NAME IS WRONG: it is the **Storage** screen "TEMPORARY FILES" (Current Cache Size 0.5 GB bar, Max Cache Size 1 GB, CLEAN CACHE) | default |
| 340 | 1401:4513 | Language landscape | 1600x720, left section list (Language selected) + right content | closed |
| 341 | 1401:4572 | Language landscape (dropdown open) | dropdown list under field | open |
| 342 | 1401:4790 | Language landscape | Español selected | after selection |

Nothing for: Dev Tools, sign-out confirm, delete-account confirm/splash/login, camera-mode, SDK skybox-lock warning (revamp did
not redraw these; keep the older frames, section 3).

## 2. Flow
- Portrait is master-detail. Root "Settings" list (no back arrow, large title) -> tap row -> detail with `<` + section title. `<` returns to root list.
  The old horizontal pill tab strip is gone. Bottom nav (compass / backpack / gear active / avatar) stays visible on root and details.
- Landscape: no root screen; the list is a permanent left column; tapping a row swaps the right pane (no navigation push). The `<` at top-left of
  the column leaves Settings (close menu). Opening landscape lands on a section (Language shown selected; default section presumably Graphics, unconfirmed).
- Account: ADD EMAIL -> guest upgrade OTP flow (Global.upgrade_to_otp, not drawn); SIGN OUT -> confirm modal (old frame 245) -> splash -> login (246/247);
  "Delete your account?" -> delete confirm (old 254) -> splash/login (255/256).
- Help: Report Bug opens in-app bug modal (no external icon); Report Content, Help, Discord, Privacy, Content Policy, Term Of Service open URLs (external icon).
- Language dropdown: tap field -> inline list; tap item -> list closes, field updates, locale applied app-wide immediately (strings re-render; frame 333 is the field only).

## 3. What changed vs older settings-portrait (which is newer, per section)

| Section | Older (220-258) | Revamp (327-342) | Newer | Change |
|---|---|---|---|---|
| Navigation | title "Settings" + horizontally scrolling pill tabs with icons (Graphics, Audio, Gameplay, Account, Storage, Dev tools) | root list of text rows with chevrons; detail has `<` + section title | revamp | pill tabs removed; Language added; Account/Help split |
| Graphics | same two groups. 238-241: Dynamic Graphics OFF -> profile dropdown enabled with Low/Medium/High; Custom Skybox list Midnight/Afternoon/Midday/Morning with a live scene preview thumbnail; 252 creator-lock banner "The skybox is set by the creator. It can't be changed." | only default (all-disabled) state; heading "Graphic Profiles" unchanged; no preview image, no banner drawn | partially (visual only) | dropdown-open/preview/lock states exist ONLY in old; keep them. Old option list had no "Very Low"; Low/Medium/High only |
| Audio | 5 sliders (General, Scene, UI SFX, Music/Ambient, Avatar & Emote SFX); 228 shows 50% and 0% | same 5 | same | no change; revamp slider track is taller (40px bar) and rounder; header now `<` Audio |
| Gameplay | CHAT & SOCIAL, optional CAMERA group (Camera Mode dropdown First/Third person, locked variant with banner "The camera mode is set by the creator. It can't be changed."), INTERFACE with 4 toggles (Hide Interface master; Hide Player Names, Hide "View Profile", Hide Interaction Prompts) | CHAT & SOCIAL + INTERFACE only; Hide Player Names moved ABOVE Hide Interface and is enabled independently (not a child); only 2 children (View Profile, Interaction Prompts) disabled | revamp for layout | CAMERA group not redrawn (keep old); **Hide Scene Interface dropped from both** (legacy key still exists) |
| Language | absent | new section: "App Language" dropdown | revamp only | new |
| Account | merged: REPORT / HELP link list + Version + SIGN OUT + "Delete Account" underlined | identity header (name, #tag, shortened address), guest card or Email field, SIGN OUT, "Do you want to **Delete your account?**" (pink underline link) | revamp | links moved to Help; Version moved to root list; identity added; copy changed |
| Help & Support | not a section | new: banner + SUPPORT / COMMUNITY / LEGAL & POLICIES groups | revamp only | regrouped (Discord now own group; old "REPORT"/"HELP" groups removed) |
| Storage | TEMPORARY FILES: cache bar, Max Cache dropdown 1/2/4 GB (list open states 248-251), CLEAN CACHE; empty state "00 GB" + disabled CLEAN CACHE | same layout, 0.5 GB state | same | no change (revamp shows only default; keep the open-list / empty / disabled states from old) |
| Dev Tools | Scene Logs, Raycast Debugger, Scene Processing, Realm dropdown, WebSocket URL + CONNECT, Process Tick Quota slider, TEST NOTIFICATIONS... (237); Note 257 "Review section dev settings" | absent | old (only source) | designer asked for review; keep non-production-only, text-heavy |
| Sign out / Delete | modal "Sign out / You'll be signed out of your account on this device." CANCEL / SIGN OUT; "Delete account / This will permanently delete your account and cannot be undone." CANCEL / DELETE; splash; login "Get started!" CREATE ACCOUNT / SIGN IN | not redrawn | old | still current. Note 258 (ES): "Ver que cosas implican borrar la cuenta" (review what deleting implies) |
| Landscape | old `settings-landscape` source was wrong | 3 real landscape frames | revamp | see section 5 |

Net: use revamp for navigation shell, Language, Account, Help, root list, Gameplay ordering; use old for everything the revamp omitted (open dropdowns, locks, banners, preview, dev tools, modals, empty states).

## 4. Per-screen spec (portrait 720x1600)

Common: background diagonal gradient, top-left `#6B1FA6` -> `#470071` (token purple ~450/500; `colors.purple350/450` close but not identical - verify). Side margin 48. Detail header: back chevron
(x 64-88, y~115) + title Inter Bold ~44 white, baseline y~117. Bottom nav 1420-1600 `#161518` (= colors.shadow, `layout.navHeight` 77dp = 154px; here 180px incl. safe area). Section caption: Inter Bold ~24 caps
lavender `#E8B9FF` at y~232, 1px lavender divider at y~258 spanning 48..672. Row label Inter Medium ~30 white; description ~22 lavender-70%, 2-line max, wrapped at ~440 wide (leaving the toggle).
Disabled = ~35% alpha on label, helper, and control. Toggle 140x64: ON `#FF2D55` track with white `#FCFCFC` thumb right; OFF track `#2B0044`-ish with thumb left; disabled OFF track `#3A1F55` thumb grey. Slider: label row (left) + value "100%" (right, y~307), bar 48..672 x 40 high at y~348, fill `#FF2D55`, track `#2B0044`, thumb white pill 32x50 rounded. Dropdown field 624x68 radius ~10, fill `#2B0044`/`#32004F` (disabled darker 35%), text 30, chevron right at x~636; helper text below 22 lavender. Open: field gets 1px lavender border + chevron up; list panel `#1A0030` radius ~10 with 1px lavender border + shadow, rows 76 high, selected row slightly darker bg + lavender text + check at right.

### 4.1 Root list (334/335)
Title "Settings" (Inter Bold ~44) at y~117, x 48 (no back). Rows 102 high at y 212,314,...,824: text x=60 (Inter Medium 32 white), chevron-right x~644. Rows (order): Graphics, Audio, Gameplay, Language, Account, Help & Support, Storage. Dev Tools appears only in non-production (not drawn). Footer: "Version 1.14.0" lavender 24 at (50,1344) + copy icon (copies full version, toast "Version copied to clipboard" + haptic per legacy). 334 vs 335: version string differs (1.14.0 newer) - show `getVersion()`; in prod "major.minor.patch", otherwise full string.

### 4.2 Graphics (327)
PERFORMANCE: row "Dynamic Graphics" + toggle; desc "Adjusts graphics automatically for optimal performance." Then label "Graphic Profiles" + dropdown (value "High") + helper "Sets a graphics quality preset for your device." (disabled while Dynamic Graphics ON; shows manager's current profile). VISUAL: "Dynamic Skybox" toggle + "The sky changes over time as you play."; "Custom Skybox" label + dropdown (placeholder "Select"; disabled while dynamic ON; options Midnight, Afternoon, Midday, Morning; turning dynamic OFF selects Morning). Old extras to keep: scene preview thumbnail under Custom Skybox when dynamic OFF; lock banner when SDK scene sets skybox time (warning icon + "The skybox is set by the creator. It can't be changed.", dropdown + toggle disabled). Dynamic Graphics row is MOBILE-only in legacy (always shown in RN).

### 4.3 Audio (328)
VOLUME caption; rows General, Scene, UI SFX, Music/Ambient, Avatar & Emote SFX (pitch 174px). Value label "NN%" right. Drops: Voice Chat and Mic Amplification (legacy keys, see section 6). Slider 0-100 step 1, applies live, persists on release.

### 4.4 Gameplay (329)
CHAT & SOCIAL: "Close Chat After Sending" toggle. INTERFACE: "Hide Player Names" (independent); "Hide Interface" + desc "Hide the on-screen interface to take clean screenshots." (master); children "Hide “View Profile”", "Hide Interaction Prompts" disabled while master OFF. Old-only CAMERA group between them: "Camera Mode" dropdown First person / Third person; locked: dropdown disabled + banner "The camera mode is set by the creator. It can't be changed."; shown only in explorer. Interface group hidden outside explorer (legacy).

### 4.5 Language (330-333)
Label "App Language" (Inter Medium 30, y 225), dropdown (y 262-330), helper "Changes the Decentraland interface language. Experiences may use a different language depending on the creator." Options (native names): English, Español, Português (Brasil) (+ "xx-pseudo" non-prod). Selected = lavender + check.

### 4.6 Account (336/337)
Header: name Inter Bold ~36 (y 230; truncates, design uses "Namemaxcharacte"), right "#1234" grey-lavender 28 (hidden for claimed names), address "0x4e1...1c13B" lavender bold 24 at y 283. Guest: card 624x222 radius ~10, 1px lavender border, dark translucent fill; title "Unlock the full experience" Inter Bold 32; body "Add email to play across mobile and desktop, and claim an exclusive Wearable."; button ADD EMAIL full-width ruby `#FF2D55` 54 high radius 6 text Inter Bold ~22 caps. Upgraded: label "Email" + read-only field `#2B0044` with "name@email.com" (no card). Bottom pinned: SIGN OUT outlined button (y 1152-1246, `#2B0044` fill, 2px white-ish border, radius 10, logout icon left + "SIGN OUT" Inter Bold 32 caps); below, centred "Do you want to " white + "Delete your account?" `#FF5A8A`-ish pink underline link (y 1327, 26px).

### 4.7 Help & Support (338)
Banner (only on unsupported devices; = legacy MODAL_END_OF_DEVICE_SUPPORT): 624x86 card with warning triangle + "Device no longer supported". Groups: SUPPORT: Report Bug (no icon, opens native bug modal), Report Content (external icon), Help (ext). COMMUNITY: Discord (ext). LEGAL & POLICIES: Privacy Policy, Content Policy, Term Of Service (sic, "Term" singular - copy bug; legacy string is "Terms of Use"). Row pitch 84, label Inter Medium 32, external icon trailing 16px after text. URLs from legacy (3.5 of settings-portrait report).

### 4.8 Storage (339, wrongly named "Help (temporary files/dev tools)")
TEMPORARY FILES caption; "Current Cache Size" + right "0.5 GB" (empty state "00 GB" typo in old; show "0 MB"), bar 624x40 fill ruby proportional to size/max; "Max Cache Size" dropdown (1 GB / 2 GB / 4 GB) + "Sets how much storage the app can use."; CLEAN CACHE outlined button (disabled at size 0) + "Clears downloaded files used to speed up loading. Safe to delete." (Godot copy still says "…most visited scenes…": update).

## 5. Landscape layout rules (1600x720; frames 340-342, Language shown)

- Two columns, no title, no bottom nav (nav hidden in landscape), no root screen. Gradient bg as portrait (`#6B1FA6`->`#470071`).
- Left column x 120..496 (376 wide): back chevron `<` at (120-145, y 40-78); section rows start y 146 (row centre), pitch 68, row height 60; text x 132 Inter Medium ~32 white, chevron right at x~468; selected row = filled `#7B3AA0` radius ~8, bold text, spans 120..496 (padding 12). Order: Graphics, Audio, Gameplay, Language, Account, Help & Support, Storage. Footer "Version 1.10.0" + copy icon at (120, 653), lavender 24.
- Right pane x 640..1490 (850 wide; gutter 144 from list; right margin 110): content starts y 133 (first label centre, 22px below section top). NO pane title (section name is only the selected left row). Same controls as portrait stretched to 850: dropdown 640..1490 x 68 at y 170-238; helper at y 262, 2 lines of 22px lavender text (copy wraps at ~820).
- Dropdown open: field gets lavender 1px border, list panel 640..1490 starting 8px below (y 248), rows 76 high with 24px inner padding, checked row lavender text + check at right 1448; panel overlays content below (z over, shadow), does not push layout.
- Pane scrolls vertically independently; list column fixed. Section captions (PERFORMANCE etc.), dividers (full pane width), toggles (right aligned at pane right edge, 140x64), sliders (full pane width), cards: reuse portrait components, only the container width changes (848 vs 624). No frame shows a two-column pane split inside a section, so keep one column.
- Rules per section in landscape (derived, no frame): Graphics/Audio/Gameplay/Help/Storage stack exactly as portrait inside the 850 pane; Account: identity header row (name left, tag right) and guest card/email field at pane width, SIGN OUT + delete link NOT pinned to bottom of screen but pinned to pane bottom (safe assumption; legacy landscape showed an avatar in the header - revamp shows none, confirm); Help banner full pane width. Safe margins: 120 left (notch inset included), top ~40.
- Rotation: selected section persists; portrait->landscape from a detail view selects that section; landscape->portrait lands on the detail of the selected section.

## 6. Settings keys: shown / dropped / added vs reports/settings-portrait.md

| Legacy key (cfg `config/...`) | Revamp |
|---|---|
| graphic_profile | shown (dropdown High; options Low/Medium/High in old frames; Very Low/Custom not drawn - Custom dev only) |
| dynamic_graphics_enabled | shown (toggle) |
| dynamic_skybox | shown |
| skybox_time | shown (Custom Skybox: Midnight/Afternoon/Midday/Morning) |
| view_distance, particle_quality, max lights (Custom profile) | not shown (dev-only, stays dev) |
| audio_general_volume | shown "General" |
| audio_scene_volume | shown "Scene" |
| audio_ui_volume | shown "UI SFX" |
| audio_music_volume | shown "Music/Ambient" (merges old SETTINGS_MUSIC_AMBIENT label) |
| audio_avatar_and_emotes_volume | shown "Avatar & Emote SFX" |
| audio_voice_chat_volume | DROPPED from UI (legacy VOICE CHAT caption gone) - decide: hide, keep key default 100 |
| audio_mic_amplification | DROPPED (was already a no-op) |
| submit_message_closes_chat | shown |
| camera mode (session) | not in revamp, present in old frames (keep) |
| Hide Interface / View Profile / Interaction Prompts / Player Names | shown (Player Names now independent, above Hide Interface) |
| Hide Scene Interface (set_hide_scene_ui) | DROPPED from both old and revamp frames |
| locale | ADDED as its own section (en, es, pt_BR) |
| max_cache_size | shown (1/2/4 GB; note legacy default idx 1 = 2 GB but Figma shows 1 GB) |
| cache size / clear_cache | shown, section title "TEMPORARY FILES" (legacy locale key SETTINGS_TEMPORARY_FILES exists) |
| Help links (Help, Discord, Report bug, Report content, Privacy, Content, Terms) | shown, regrouped SUPPORT / COMMUNITY / LEGAL & POLICIES |
| Account: identity header (name, tag, address) | shown; email field for upgraded accounts ADDED; guest card kept |
| Account: delete / sign-out | shown, delete is now an inline sentence link |
| "Device no longer supported" | ADDED to Help (maps to legacy end-of-device-support modal) |
| Version row | moved from Account/Dev to root list/landscape column footer |
| Dev Tools section | not redrawn (old frame 237 only); dev builds only |
| process_tick_quota_ms, realm, preview URL etc. | dev tools only |

## 7. Tokens and components

Colors (sampled): bg `#6B1FA6`->`#470071`; section caption/divider/address `#E8B9FF` (= `colors.lavender`); ruby `#FF2D55` (= `colors.ruby`); snow `#FCFCFC`; nav/shadow `#161518`; control fill `#2B0044`/`#32004F` (closest `colors.purple550 #24033B`, not exact); selected landscape row `#7B3AA0`; pink delete link `#AE3575`-`#FF5A8A` range. Font Inter (existing `fonts`). Radii: buttons ~10 (existing `radii.button` u(8)), fields ~10, cards ~10.
Existing in `rn/src/ds`: AppText, BottomNav (has `settings` tab), IconButton, Icon, PrimaryButton (ruby), Pill, Tabs, Gradient, tokens. Missing: Switch, Slider, Dropdown (+open list), ProgressBar, SectionRow (chevron row + selected state), SettingRow (label+desc+control), Banner (warning icon), ReadOnlyField, ConfirmModal, LinkRow (external icon), outlined button (SIGN OUT / CLEAN CACHE).
Icons needed: chevron left/right/down/up, check, copy, external-link, warning triangle, logout; section icons no longer needed (pills removed).

## 8. Legacy and RN status
Legacy: `godot/src/ui/pages/settings/settings.gd` + `settings.tscn` (sections incl. Temporary Files title `SETTINGS_TEMPORARY_FILES`, Language `SETTINGS_LANGUAGE*`, end of device support `MODAL_END_OF_DEVICE_SUPPORT_*` in `godot/locale/*.csv`). Behaviour not in Figma: camera lock logic, SDK skybox lock, preview image, delete-account processing/done/fail dialogs, guest on-device deletion, dev tools, haptic on version copy, `track_click_button("HIDE_UI","SETTINGS","")`.
RN (`rn/src`): no settings screen, no settings bridge (see previous report section 5); only `BottomNav` `settings` tab label in `rn/src/Home.tsx` / `ds/BottomNav.tsx`. Gap list = all components in section 7 + screens (RootList, Graphics, Audio, Gameplay, Language, Account, Help, Storage) + bridge `getSettings/setSetting/clearCache/getCacheSize/signOut/deleteAccount/getVersion` + events (camera lock, skybox lock, session hide-UI sync, dynamic-graphics profile change).

## 9. Open questions / inconsistencies
1. Frame 339 named "Help (temporary files / dev tools)" is the Storage screen; Dev Tools is not in the revamp - is it dropped from the app or just not redrawn?
2. Version strings differ (1.14.0 in 334 vs 1.10.0 in 335 and landscape): 1.14.0 treated as the newest; use live version.
3. Voice Chat volume and Mic Amplification removed: confirm the Voice Chat slider is intentionally dropped (users lose control of voice volume).
4. Hide Scene Interface missing in both pages - confirm.
5. Camera mode group and lock banners absent from revamp - assumed kept.
6. Help copy "Term Of Service" vs legacy "Terms of Use"; Report Bug has no external icon (in-app modal) - intended.
7. Language helper copy does not translate in frame 333/342 (design placeholder); real UI must localise it.
8. Landscape frames only show Language; landscape Account/Graphics layout is derived (section 5). Default section on landscape open unknown.
9. Max cache default: Figma 1 GB, legacy default 2 GB (idx 1).
10. Landscape has no bottom nav; confirm how user leaves Settings (only the `<`).
11. Old Graphics dropdown options Low/Medium/High; legacy has Very Low (0) - hidden on mobile?
