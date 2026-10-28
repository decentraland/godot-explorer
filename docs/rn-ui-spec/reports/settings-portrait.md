# settings-portrait report (manifest seq 220-258)

Source: Figma file `l6t9Qs4ZueQdFhE6Gcp6hZ`, page `129:2513`. All 39 screenshots were viewed (39/39). `get_design_context` was
NOT called (no tokens beyond pixel sampling of the PNGs; hex values below are sampled, Figma px at 720x1600, RN dp = px/2
per `rn/src/ds/scale.ts`). Section 7 is the legacy Godot setting-key table from the earlier pass (still valid).

## 1. Screen inventory (39 seqs)

Common chrome on every settings frame: purple gradient bg, title "Settings", horizontally scrolling chip tab bar, bottom nav bar.
Distinct screens: Graphics, Audio, Gameplay, Account, Storage, Dev tools, Sign-out dialog, Delete dialog, Splash, Login ("Get started!"),
plus two landscape HUD frames and two designer notes.

| seq | id (node) | name | what it shows | state/variant |
|---|---|---|---|---|
| 220 | 851:10734 | ThirdPerson (2400x1080) | In-world mobile HUD: chat bubble, compass+share pill (top-left), eye icon + avatar (top-right), emote button (bottom-left), action cluster E / F / "1 2" / jump / pointer (bottom-right) | in-world reference; eye icon = "hide interface" entry point |
| 221 | 851:10490 | Settings-back | Graphics tab, NO bottom nav, title row has a back chevron "<" before "Settings" | full-screen/pushed variant (opened from HUD, not nav); Dynamic Graphics ON, profile disabled "High", Dynamic Skybox ON, Custom Skybox disabled "Select" |
| 222 | 348:1584 | Settings-Graphics | Graphics, with bottom nav, title without chevron | Dynamic Graphics ON (profile disabled), Dynamic Skybox ON (custom disabled) = default |
| 223 | 290:2035 | Settings-Graphics | Graphics | Dynamic Graphics OFF: Graphic Profiles enabled "High"; Skybox ON |
| 224 | 348:1986 | Settings-Graphics | Graphics, profile dropdown OPEN | selected High (check mark); options Low / Medium / High |
| 225 | 718:6559 | Settings-Graphics | Graphics, dropdown open | selected Medium (check) |
| 226 | 718:6713 | Settings-Graphics | Graphics, dropdown closed | profile = Medium, Dynamic Graphics OFF |
| 227 | 303:2221 | Settings-Audio | 5 volume sliders | all 100% |
| 228 | 738:7766 | Settings-Audio | same | General 50%, Scene 0% (muted), rest 100% |
| 229 | 1348:4487 | Settings-Gameplay | Gameplay, no camera section | Close Chat OFF; Hide Interface OFF so 3 sub-toggles disabled |
| 230 | 1348:4378 | Settings-Gameplay | same | Close Chat ON |
| 231 | 1348:4615 | Settings-Gameplay | Gameplay with CAMERA section | Camera Mode dropdown "First person" (enabled), Close Chat ON |
| 232 | 1349:4751 | Settings-Gameplay | same | Camera Mode "Third person" |
| 233 | 1350:4893 | Settings-Gameplay | same | Camera Mode disabled "Third person" + warning banner "The camera mode is set by the creator. It can't be changed." |
| 234 | 303:3251 | Settings-Account | Account tab, tab bar scrolled right | list with Version row (1.10.0) |
| 235 | 348:1419 | Settings-Storage | Storage, tab bar scrolled right | cache 0.5 GB of max (bar ~50%), Max "1 GB", CLEAN CACHE enabled |
| 236 | 412:4560 | Settings-Storage | Storage | EMPTY: "00 GB", empty bar, CLEAN CACHE disabled (dimmed) |
| 237 | 381:3096 | Settings-Dev Tools | Dev tools tab (tab bar scrolled to end) | scroll content continues under nav ("TEST NOTIFICATIONS" button cut off) |
| 238 | 718:5548 | Settings-Graphics | Graphics | Dynamic Skybox OFF -> Custom Skybox enabled "Morning" + live preview image below (350px tall) |
| 239 | 718:5766 | Settings-Graphics | Graphics scrolled down ~136px | Custom Skybox dropdown OPEN (opens upward over page): Midnight / Afternoon / Midday / Morning; Morning selected; preview below |
| 240 | 738:7476 | Settings-Graphics | same | open, Midday selected, preview brighter (noon) |
| 241 | 738:6935 | Settings-Graphics | scrolled | closed, Custom Skybox shows "Noon" (label mismatch, see Q3) |
| 242 | 1348:4266 | Settings-Gameplay | Gameplay | ALL toggles ON: Hide Interface, Hide Player Names, Hide "View Profile", Hide Interaction Prompts all enabled/on |
| 243 | 969:4359 | Place (2400x1080) | World view with all HUD hidden except a "Show UI" pill top-right | result of Hide Interface = ON |
| 244 | 738:7985 | Settings-Account | Account, Version row ABSENT | variant without version row |
| 245 | 412:4763 | Settings-Account-SingOut | Confirm modal over blurred Account | modal |
| 246 | 738:8292 | DCL_SPLASH | purple #691FA9 + centered DCL logo circle (~144px) | after sign out |
| 247 | 738:8278 | ACCOUNT_hOME | "Get started!" login/home | after sign out |
| 248 | 738:8696 | Settings-Storage | cache 0.5 GB, Max "1 GB" | duplicate of 235 |
| 249 | 412:4288 | Settings-Storage | Max Cache dropdown OPEN | "1 GB" checked; 1 GB / 2 GB / 4 GB |
| 250 | 738:8796 | Settings-Storage | dropdown OPEN | "2 GB" checked (field still shows "1 GB" = pre-commit render) |
| 251 | 738:9027 | Settings-Storage | closed | Max Cache "2 GB" |
| 252 | 1099:5103 | Settings-Graphics | Graphics | SDK-locked skybox: Dynamic Skybox switch disabled (OFF look), Custom Skybox disabled "Select", warning banner "The skybox is set by the creator. It can't be changed." |
| 253 | 738:8123 | Settings-Account | same as 244 | duplicate |
| 254 | 432:1708 | Settings-Account-Delete | Confirm modal over blurred Account | modal, warning icon |
| 255 | 738:8297 | DCL_SPLASH | same as 246 | after delete |
| 256 | 738:8261 | ACCOUNT_hOME | same as 247 | after delete |
| 257 | 666:3102 | Note 360x164 | designer note: "Review sectionn dev settings" | note, not UI |
| 258 | 432:1923 | Note 360x190 | designer note (Spanish): "Ver que cosas implican borrar la cuenta." (check what deleting the account involves) | note, not UI |

## 2. Flow graph

- Entry: bottom nav gear icon -> Settings (nav visible, active gear is the filled purple one). Frame 221 (no nav, back chevron) is the pushed
  variant, likely opened from the in-world HUD/profile; chevron returns to the previous screen. There is NO master-detail list in Figma:
  the legacy portrait list->detail is replaced by a chip tab bar on a single screen (Graphics default). Back from the chip view = leave Settings via nav.
- Tabs (left to right): Graphics, Audio, Gameplay, Account, Storage, Dev tools (Dev tools visible in this design, non-production only per legacy; note 257 asks to review it). Tapping a chip swaps the content; the strip scrolls horizontally and auto-scrolls to keep the active chip visible (see 234/235/237 offsets). Help/Language from legacy are NOT chips: Help rows live under Account > HELP; Language is not designed (Q1).
- Graphics: Dynamic Graphics OFF -> profile dropdown enabled -> tap opens list -> choose -> closes. Dynamic Skybox OFF -> Custom Skybox enabled + preview image; dropdown opens upward when low on screen.
- Gameplay: Hide Interface ON -> Hide Player Names / View Profile / Interaction Prompts enabled; with the interface hidden the world shows only a "Show UI" pill (243) that restores it. The HUD eye icon (220) is the quick toggle.
- Storage: Max Cache dropdown -> pick -> value commits; CLEAN CACHE -> size goes to "00 GB" and button disables (236). No confirm dialog designed.
- Account: SIGN OUT -> modal 245 -> SIGN OUT -> splash 246 -> Get started 247 (CREATE ACCOUNT / SIGN IN). CANCEL dismisses. Delete Account -> modal 254 -> DELETE -> splash 255 -> Get started 256 (progress/failed states not designed).

## 3. Screen specs (Figma px at 720x1600; dp = px/2)

### 3.1 Shell (all settings frames)
- Bg: vertical purple gradient, top-left lighter `#6C1FA7` fading to `#470071`/`#460F6F` (purple450) by mid screen; radial lighter glow top-left. Use ds `Gradient`.
- Title "Settings": Inter Bold ~48px (24dp), white, x=48, baseline y~123. 221 adds a back chevron at x~76 and shifts the title right (x=117).
- Chip tab bar: y 194-250 (h 56), x starts at 48, gap ~24, horizontally scrollable, overflows right edge (no fade). Chip = pill, icon 24px + label Inter Medium ~26px. Inactive: fill `rgba(255,255,255,0.25)`-ish (sampled `#894CB8` over bg), label white-ish 80%; active: fill lavender `#E8B9FF`, label/icon dark `#161518`. Icons: monitor (Graphics), speaker (Audio), gamepad (Gameplay), person (Account), archive box (Storage), `</>` (Dev tools).
- Content column: left/right padding 48 (24dp), width 624. Section header: uppercase Inter Bold ~26px lavender `#E8B9FF`, 1px lavender divider under it (y+27), then rows. Section spacing ~120-150.
- Bottom nav: bar y 1418-1600 (h 182 = ds `layout.navHeight` 77dp + safe area), bg `#161518`, 4 icons: compass, backpack, gear (active, purple filled), avatar. Exists: `rn/src/ds/BottomNav.tsx`.
- Content scrolls under the nav (237, 238 are cut by the nav). Pages are ScrollViews.

### 3.2 Row components
- Toggle row: label Inter Regular ~30px white (disabled: 30% opacity), optional description Inter Regular ~22px, white ~70%, max 2 lines, width ~440. Switch right-aligned: track 140x64 px (70x32 dp) pill, ON = ruby `#FF2D55` with 2px lighter-ruby border and white knob `#FCFCFC` right; OFF = track `#2B0044` with white knob left; DISABLED = track `#2B0044` at ~60% and knob `#3A2A4A`-ish dim (see 229 sub-toggles).
- Dropdown field: label 30px (disabled dim), field 624x68 radius ~10px bg `#2B0044`, text 30px, chevron-down at right (x~636); open state: 1px lavender border + chevron-up; hint under field 22px white 70% ("Sets a graphics quality preset for your device."). Disabled: whole field 30% opacity.
- Dropdown list (open): panel same width, bg `#1C0033`-ish dark, 1px `#7B3FB0` border, radius 10, shadow; item height 76, text 30px white left pad 24; selected item: dark pill `#0F0020` with lavender text and lavender check at right. Opens downward (224, 249); upward for Custom Skybox (239) when the page is scrolled near the bottom.
- Slider row: label left (30px), value right "NN%" 30px; track y+50, 624x40 px (full-width, radius 8), fill ruby `#FF2D55`, empty `#2B0044`, thumb white `#FCFCFC` rounded rect 32x50 px at the end of fill (at 0% thumb sits at left edge, at 100% flush right). Row pitch 174.
- Link row (Account): label 30px white + external-link icon 24px (open-in-new) after the label (NOT right aligned); Version row uses a copy icon instead. Row pitch 84.
- Outline button: "SIGN OUT" / "CLEAN CACHE": 624x96 px, radius 12, 1px white-ish border, fill `#2B0044`, label Inter Bold uppercase 30px white (sign out has a log-out icon left). Disabled: text `#5E3C7A`-ish.
- Text link: "Delete Account" centered, ruby `#FF2D55`, underlined, Inter Regular 30px.
- Warning banner: 624x~110, 1px lavender-ish border, radius 10, fill `#1C0033`, warning triangle icon 48px left + 2-line text 30px white. Used in 233 and 252.
- Progress bar (Storage): same geometry as slider track without thumb; fill ruby with rounded right end.
- Text input + button (dev tools): input 438x68 `#2B0044` radius 10 placeholder-like text "http://127.0.0.1:8000/"; CONNECT button 170x68 ruby `#FF2D55`, white bold uppercase 26px.
- Modal: scrim blur of the page, dialog 520x~385 px centered (x 100-620), radius ~24, gradient `#4C147C` with magenta glow top-right and bottom, title Inter Bold 40px white centered, body 30px centered white, two buttons 188x74 radius 12: CANCEL `#2F0C4B` (dark, white bold label), confirm ruby `#FF2D55`. Delete variant has an orange warning-triangle badge overlapping the top edge (y~560).

### 3.3 Graphics (222-226, 238-241, 252, 221)
Copy: "PERFORMANCE", "Dynamic Graphics" / "Adjusts graphics automatically for optimal performance.", "Graphic Profiles" (dropdown: Low, Medium, High; placeholder when empty not shown) / "Sets a graphics quality preset for your device.", "VISUAL", "Dynamic Skybox" / "The sky changes over time as you play.", "Custom Skybox" (dropdown "Select" when empty; Midnight, Afternoon, Midday, Morning) + skybox preview thumbnail (624x350 px, screenshot of the avatar in Genesis Plaza under that time of day; changes live with selection).
States: Dynamic Graphics ON => Graphic Profiles disabled and shows the manager's current profile; OFF => enabled. Dynamic Skybox ON => Custom Skybox disabled "Select", no preview; OFF => enabled with value + preview. Creator-locked skybox (252) => Dynamic Skybox disabled, Custom disabled, banner. Design lists only Low/Medium/High (no VeryLow/Custom) - Q2. Data: local config via Godot bridge (see 7.1); profile feedback event from DynamicGraphicsManager; `sdk_skybox_time_active` event.
In legacy the Dynamic Graphics row is MOBILE ONLY; RN is mobile so always shown.

### 3.4 Audio (227-228)
"VOLUME" section; 5 sliders in order: General, Scene, UI SFX, Music/Ambient, Avatar & Emote SFX; each "NN%" (0-100 step 1, integer display). Mapping to legacy keys: General=audio_general_volume, Scene=audio_scene_volume, UI SFX=audio_ui_volume, Music/Ambient=audio_music_volume, Avatar & Emote SFX=audio_avatar_and_emotes_volume. Legacy has two more (voice chat, mic amplification, section "VOICE CHAT") NOT in Figma - Q4. Slider applies live while dragging and persists on release. Percent text is right aligned, updates live.

### 3.5 Gameplay (229-233, 242, 243, 220)
Sections: "CHAT & SOCIAL" -> "Close Chat After Sending" (no description). "CAMERA" -> "Camera Mode" dropdown (First person / Third person), shown only when in the world; disabled + banner when the creator locks it or cinematic. "INTERFACE" -> "Hide Interface" / "Hide the on-screen interface to take clean screenshots." (master); children (disabled/dim while master OFF): "Hide Player Names", "Hide “View Profile”", "Hide Interaction Prompts" (curly quotes). Legacy also has "Hide Scene Interface" - not designed (Q5). Note legacy makes Player Names independent of master; Figma dims it with the master (design wins? Q5). Pressing master ON: world HUD hides and shows "Show UI" pill (243: black 60% pill, white 28px bold text, top-right) which restores the UI and settings state. Session-only flags (not persisted) except Close Chat.

### 3.6 Account (234, 244, 253, 245, 254)
"REPORT": Report Bug, Report Content. "HELP": Help, Privacy Policy, Content Policy, "Term Of Service" (sic; legacy "Terms of Use"), Discord, "Version 1.10.0" (copy icon; tap copies, toast "Version copied to clipboard" from legacy). Then SIGN OUT outline button (y 1182-1278) and "Delete Account" link below. Rows are external links (all `Global.open_url` in legacy; Report Bug = native modal in legacy). NO profile header / guest upgrade card in Figma (legacy has them) - Q6. Page content is bottom-anchored buttons; rows may scroll.
Sign-out dialog: title "Sign out", body "You'll be signed out of your account on this device.", buttons CANCEL / SIGN OUT. Delete dialog: warning icon, title "Delete account", body "This will permanently delete your account and cannot be undone.", buttons CANCEL / DELETE. After confirm: splash (DCL logo on `#691FA9`) then Get started screen.

### 3.7 Storage (235, 236, 248-251)
"TEMPORARY FILES": "Current Cache Size" + value right ("0.5 GB"; empty "00 GB" typo-like, Q7) over progress bar (fill = used/max); "Max Cache Size" dropdown (1 GB / 2 GB / 4 GB) / "Sets how much storage the app can use."; CLEAN CACHE button / "Clears downloaded files used to speed up loading. Safe to delete." Disabled when cache is empty.

### 3.8 Dev tools (237)
Section "DEV TOOLS": toggles "Scene Logs", "Raycast Debugger", "Scene Processing" (all shown ON in the frame); "Realm" dropdown ("Select one"); "WebSocket URL" input (default "http://127.0.0.1:8000/") + CONNECT; "Process Tick Quota" slider with "100%"; then a "TEST NOTIFICATIONS" outline button and more below (cut off; only top of the screen is designed). Subset of legacy 7.7; the rest (raycast area, multiplayer debug, lights...) undesigned. Gate behind non-production.

### 3.9 Splash and login (246, 247, 255, 256)
Splash: solid `#691FA9`, DCL logo circle ~144 px centered at (360,800). Get started: vertical gradient `#C63FCB` top to `#6A1FA6` bottom; ds logo "Decentraland" wordmark at (72,458); "Get started!" Inter Bold ~72px white; body 40px "**Discover a virtual social world** shaped by its community of creators & explorers." (first part bold); buttons x 72-648: CREATE ACCOUNT (ruby, h 112, radius 12) and SIGN IN (dark `#4A1868`), both bold uppercase 36px. Owned by the auth/flow report; RN already has `rn/src/flow/EntryFlow.tsx` / `FtueScreen.tsx`.

## 4. Tokens (sampled; map to `rn/src/ds/tokens.ts`)
ruby `#FF2D55` = colors.ruby (switch ON, slider fill, CONNECT, SIGN OUT confirm, Delete link); lavender `#E8B9FF` = colors.lavender (section labels, dividers, active chip, selected option); snow `#FCFCFC` = colors.snow (knob, thumb); input/track `#2B0044` ~ purple550 `#24033B`/(not exact, add `fieldDark`); dialog `#4C147C`; cancel `#2F0C4B`; nav `#161518` = colors.shadow; bg `#460F6F` = purple450. Font Inter (ds fonts present). Radii: field 10px, button 12px, pill chips full.

## 5. RN status and gaps
Exists in `rn/src/ds`: AppText, BottomNav (gear tab already), Gradient, Icon/iconSvgs, IconButton, PrimaryButton, Pill, Tabs (underline style - not the chip style). `grep settings` in rn/src hits only BottomNav/Icon/Home: NO settings screen, store or bridge.
Missing ds components: ChipTabs (scrollable pill tabs with icons), Switch (3 states), Slider (custom thumb, live value), Dropdown + option list (open up/down, check), ProgressBar, SettingsSection header, LinkRow (ext/copy icon), OutlineButton (+disabled), TextField, ConfirmDialog (blur scrim), WarningBanner, SkyboxPreview image. Icons to add: monitor, speaker, gamepad, person, archive, code, external-link, copy, log-out, warning, chevron up/down/left, check, eye.
Bridge needed (reuse the key list in 7): `getSettings()` snapshot, `setSetting(key,value)`, events for dynamic-graphics profile change, `sdk_skybox_time_active`, camera mode set/blocked, session hide-UI sync, `getCacheSize/clearCache`, `signOut`, `deleteAccount`, `copyVersion`/`getVersion`, `openUrl`, bug-report modal, skybox preview capture (Godot screenshot at selected time or static assets per time of day - assets are one image per time, 3 variants seen).
Godot remains owner of persistence (`user://settings.cfg`, read at boot before RN).

## 6. Open questions / inconsistencies
1. Language selector and legacy "Help & Support" section are not designed as chips; Language missing in Figma.
2. Profile list is Low/Medium/High; legacy has VeryLow..High (+Custom dev). Map Figma Low->legacy Low? Confirm.
3. Skybox labels: 241 shows "Noon" while list/other frames say "Midday"; legacy 86400/64800/43200/21600 = Midnight/Afternoon/Midday/Morning.
4. Voice chat volume + mic amplification missing from Audio; Scene vs "Music/Ambient" naming overlaps legacy (Scene bus labelled Music & Ambient; Music bus separate). Figma has no separate music-bus row beyond "Music/Ambient" - need mapping decision (suggest Music/Ambient -> audio_music_volume, Scene -> audio_scene_volume).
5. Hide Scene Interface missing; Hide Player Names dimmed under master in Figma but independent in legacy.
6. Account tab has no profile header/guest upgrade card; guest users need an upgrade path and the delete flow for guests differs (auto on-device); note 258 asks what deletion implies.
7. Empty cache shows "00 GB" (likely should be "0 GB"/"0 MB"); progress denominator should be the selected max.
8. Max cache sizes 1/2/4 GB, legacy default index 1 = 2 GB; Figma default shows 1 GB.
9. "Term Of Service" vs "Terms of Use"; version row present in 234 and absent in 244/253 (decide: show always).
10. Frame 221 (back chevron, no nav) vs 222 (nav): two entry modes; confirm where 221 is reached from.
11. Dev tools tab visible in the same chip bar; note 257 says review dev settings; hide in production.
12. Missing designed states: loading of cache size, clean-cache confirm/progress, delete processing/failed, bug-report modal, toast for version copy.

## 7. Legacy contract (Godot setting keys; kept from the first pass) (source of truth: godot/src/ui/pages/settings/settings.gd, settings.tscn)

Persistence: `Global.get_config()` (ConfigData extends Rust `DclConfig`, godot/src/config/config_data.gd) saved by
`save_to_settings_file()` to `user://settings.cfg` (ConfigFile, section `config`; skipped when `Global.testing_scene_mode`).
Setters emit `param_changed(ConfigParams)`; applying is done by static helpers (`GraphicSettings`, `AudioSettings`, `GeneralSettings`, `LocaleSettings`).
RN today has NO settings bridge: `rn/src/{GodotBridge,events,engine}.ts` and mockBridge contain nothing for settings/volume.

### 7.1 Graphics (container VBoxContainer_Graphics)
| key (cfg `config/…`) | type | default | UI | persisted / applied |
|---|---|---|---|---|
| graphic_profile | int 0 VeryLow,1 Low,2 Medium,3 High,4 Custom (Custom dev-only, hidden in prod) | 3 (HardwareBenchmark may downgrade on first launch; `first_launch_completed`, `benchmark_gpu_score`, `benchmark_ram_gb`) | dropdown "Graphic Profile" (desc "Sets a graphics quality preset for your device.") | on select: `GraphicSettings.apply_graphic_profile(i)` (i<4) then save; then `DynamicGraphicsManager.on_manual_profile_change(i)` |
| dynamic_graphics_enabled | bool | true | switch "Dynamic Graphics" (desc "Adjusts graphics automatically for optimal performance."), MOBILE ONLY; when on, profile dropdown is disabled and shows the manager's current profile | save + `Global.dynamic_graphics_manager.set_enabled()`; manager emits `profile_change_requested(profile)` -> refresh dropdown |
| dynamic_skybox | bool | true | switch "Dynamic Skybox" (desc "The sky changes over time as you play."); disabled when an SDK scene forces skybox time (`Global.sdk_skybox_time_active`, signal `sdk_skybox_time_active_changed`; message "The skybox is set by the creator. It can't be changed.") | save |
| skybox_time | int seconds | 43200 | dropdown "Custom Skybox": Midnight 86400, Afternoon 64800, Midday 43200, Morning 21600; disabled while dynamic on; turning dynamic off selects index 3 (Morning) | setter only (emits SKYBOXTIME); saved on the next save |
| (derived by profile, no direct UI) | | | | anti_aliasing (0 off,1 x2,2 x4,3 x8), shadow_quality (0/1/2), bloom_quality (0/1/2), skybox (quality 0/1/2), texture_quality, limit_fps (FpsLimitMode 0 vsync,1 none,2=18,3=30,4=60,5=120), resolution_3d_scale, view_distance, particle_quality, mesh_lod_threshold, dcl_lights/dcl_light_shadows/dcl_max_lights/dcl_light_range_cap |
| view_distance | float m | 300 (profile: 100/150/200/300) | Custom profile only (dev): slider 20-320 step 10, label "%dm" | sets `Global.player_camera_node.far`, save |
| particle_quality | int 0 Off,1 Low,2 Medium,3 High | 3 | Custom only: dropdown "Particles" | `GraphicSettings.apply_particle_quality`, `AvatarAnimHelpers.apply_particles_enabled(q>0)`, save |
| max lights | int 0-64 | per profile 0/2/4/8 | Custom only: "Max Active Lights" spinbox | `DclLightSourceComponent.apply_light_settings(...)` (Rust), save |

Profile table (GraphicSettings.PROFILE_DEFINITIONS) aa/shadow/bloom/skybox/texture/fps/scale/lod/viewdist/particles/lights:
VeryLow 0/0/0/0/0/18fps/0.5/8/100/0/off(0) ; Low 0/0/0/0/0/30/0.75/6/150/1/on,2 ; Medium 0/1/1/1/1/30/1.0/3/200/2/on,4 ;
High 1/1/2/2/2/60/1.0/2/300/3/on,8 + shadows. Profile apply also sets `viewport.scaling_3d_scale`, `mesh_lod_threshold`.
RN only needs to send the profile index (+ dynamic toggle); Godot applies the rest. Other config fields (not in UI):
window_mode (0/1/2, desktop), ui_zoom (-1.0), resolution_3d_scale, limit_fps, show_fps (default true; debug overlay).

### 7.2 Audio (7 sliders, 0-100 step 1, default 100.0, all persisted `config/audio_*` + applied immediately)
| cfg key | label | applied by |
|---|---|---|
| audio_general_volume | General Volume (Master bus) | `AudioSettings.apply_general_volume_settings()` |
| audio_scene_volume | Scene Volume ("Scene" bus; labelled "Music & Ambient"/scene in strings: SETTINGS_MUSIC_AMBIENT, SETTINGS_SCENE) | `apply_scene_volume_settings()` |
| audio_voice_chat_volume | Voice Chat ("VoiceChat" bus) | `apply_voice_chat_volume_settings()` |
| audio_ui_volume | UI ("UI" bus) | `apply_ui_volume_settings()` |
| audio_music_volume | Music ("Music" bus) | `apply_music_volume_settings()` |
| audio_avatar_and_emotes_volume | Avatar & Emote ("AvatarAndEmotes" bus) | `apply_avatar_and_emotes_volume_settings()` |
| audio_mic_amplification | Mic Amplification | `apply_mic_amplification_settings()` is currently a no-op (`pass`) - value stored only |
Mapping: percent -> dB via `linear_to_db(p/100)`, floor -80 dB (0 = muted). Section captions in strings: VOLUME, VOICE CHAT. Each change saves immediately.

### 7.3 Gameplay
| key | type | default | persisted | notes |
|---|---|---|---|---|
| submit_message_closes_chat | bool | false | yes (`config`) | switch "Close Chat After Sending" |
| camera mode | enum Global.CameraMode FIRST_PERSON/THIRD_PERSON (CINEMATIC read-only) | runtime (player spawns THIRD_PERSON) | NOT persisted | dropdown "Camera" First person / Third person; row hidden unless inside explorer; disabled with warning "The camera mode is set by the creator. It can't be changed." when `Global.camera_mode_blocked` or cinematic. Apply `Global.set_camera_mode(mode)`; signals `camera_mode_set`, `camera_mode_block_changed` |
| Hide Interface (master) | bool session | false | session-only, `explorer.set_hide_main_hud_from_settings`, `is_session_hide_main_hud()`; desc "Hide the on-screen interface to take clean screenshots" | Interface group hidden outside explorer |
| Hide "View Profile" | bool session | true (when not in explorer) | `explorer.set_hide_view_profile` | disabled (50% alpha) while master off |
| Hide Interaction Prompts | bool session | true | `explorer.set_hide_world_interactions` | disabled while master off |
| Hide Player Names | bool session | false | `explorer.set_hide_player_names` | independent of master |
| Hide Scene Interface | bool session | true | `explorer.set_hide_scene_ui` | disabled while master off |
Sync signals: `Global.session_hide_ui_toggle_sync(bool)`, `session_hide_ui_options_sync(view_profile, interactions, labels, scene_ui)`. Metrics: `track_click_button("HIDE_UI","SETTINGS","")`.
Language (section "Language", desc "The language used across the app's interface."): `config/locale` string, "" = follow device; values en, es, pt_BR (+ "xx-pseudo" non-prod); labels "English", "Español", "Português (Brasil)"; apply `LocaleSettings.set_locale(code)`.

### 7.4 Account
Header: avatar (landscape only in legacy), nickname `profile.get_name()`, tag `#` + last 4 hex of address (hidden for claimed names), shortened address (`Global.shorten_address`). Data: `Global.player_identity.get_profile_or_null()`, signal `profile_changed`.
- GuestUpgradeCard (guest only, owns its own visibility; upgrade-to-email OTP via `Global.upgrade_to_otp`).
- Sign out button (`Global.sign_out()`: tears down explorer, stops notification polling, unsubscribes social service, clears state, goes to login; idempotent via `_signing_out`).
- "Do you want to delete your account?" button -> `Global.delete_account` -> AccountDeletionPopup: confirm / processing / done / fail dialogs; non-upgraded guest => automatic on-device deletion (`Global.clear_guest_device_storage()` then `Global.sign_out()`); signed-in => signed `POST DclUrls.account_deletion()` (response `{ok:true}` shows "done"; `{ok:false,error}` or PromiseError shows fail); `DELETE` cancel path exists but its button is hidden. Upgraded-guest auto delete only with `enable-upgraded-deletion` deeplink in non-prod.

### 7.5 Help & Support (not in the manifest range but part of the section list)
Rows: Help (https://decentraland.org/help), Discord (https://discord.com/channels/417796904760639509/1446513533893218465), Report bug (native BugReport modal with last screenshot, `Global.modal_manager.async_show_bug_report_modal`), Report content (`ReportContentHelper.open_form()`), Privacy Policy (https://decentraland.org/privacy/), Content Policy (https://decentraland.org/content/), Terms of Use (https://decentraland.org/terms/). All via `Global.open_url`.

### 7.6 Storage
| item | detail |
|---|---|
| Max Cache Size | `config/max_cache_size` int index 0/1/2 -> 1 GB / 2 GB / 4 GB (CACHE_SIZE_MB [1024,2048,4096]); default 1 (=2 GB; note ConfigData comment "0=512mb" is stale). Desc "Sets how much storage the app can use." Apply `GeneralSettings.apply_max_cache_size()` -> `Global.content_provider.set_cache_folder_max_size(bytes)` (decimal GB 1000^3), then save |
| Current Cache Size | `Global.content_provider.get_cache_folder_total_size()` bytes -> MB (/1e6), shown "{n} MB" or "{n.n} GB" if >=1024; progress bar value=MB, max = selected cache MB; refreshed whenever the Storage container becomes visible |
| CLEAN CACHE button | `Global.content_provider.clear_cache_folder()` then refresh size; disabled when size==0. Desc "Clears downloaded files used for most visited scenes. Safe to delete." |

### 7.7 Dev Tools (hidden when `Global.is_production()`); Note frame 257 probably explains this
Realm dropdown (11 hard-coded realms; select => `Navigator.async_go(Destination.from_input(text),"on_changerealm")` or boot explorer with `last_realm_joined`), Preview URL line edit + connect (`Global.scene_fetcher.set_preview_url`), Scene Logs Enabled (`request_debug_panel`), Raycast Debugger (`Global.set_raycast_debugger_enable`), Show Interactable Area (`explorer.set_interactable_area_visible`), Scene Paused (`request_pause_scenes`), Multiplayer Debug (`request_multiplayer_debug`), Process Tick Quota slider (`config/process_tick_quota_ms`, default 10), Fast Day/Night Cycle (10s) (`Global.skybox_time.debug_time_rotation`), light debug toggles (Scene Lights, Light Shadows, Debug Gizmos, Auto Activation Range, Global Light Budget, Max Active Lights; `DclLightSourceComponent.get/apply_light_settings`, only editable in Custom profile), Test Notification with Emojis, Show Reward Modal (MobilePet), Open User Data Folder (mobile copies path to clipboard), RETURN TO DISCOVER (`Global.return_to_discover()`), Run Avatar Impostor Benchmark.
Version row: "Version {major.minor.patch}" in prod; full string (`DclGlobal.get_full_version()`) elsewhere; tapping copies it and toasts "Version copied to clipboard" + haptic.

