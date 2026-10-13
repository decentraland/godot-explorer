# Report: settings-landscape (manifest seq 259-272)

Source: Figma file `l6t9Qs4ZueQdFhE6Gcp6hZ`, page `1002:2897` ("Settings landscape"). All 14 screenshots captured and viewed (`docs/rn-ui-spec/shots/settings-landscape/`). Frames are 1600x720 landscape (a phone held sideways, scene blurred behind), 640x1180 / 544xN content crops, and small fragments. The Figma MCP quota throttled bursts (about 3 calls per minute); `get_design_context` was NOT called, so tokens below are measured from the PNGs or taken from `rn/src/ds/tokens.ts`.

## 1. Screen inventory

| seq | id (suffix) | name | what it shows | state / variant |
|---|---|---|---|---|
| 259 | content__1086-4832 | Content (544x876) | Scroll content of Graphics: PERFORMANCE (Dynamic Graphics ON, "Graphic Profiles" dropdown dimmed "High", hint dimmed), VISUAL (Dynamic Skybox ON, "Custom Skybox" dropdown dimmed "Select") | Dynamic ON => dependent controls disabled |
| 260 | settings-graphics-landscape__1002-2907 | Settings-GRAPHICS-Landscape | Panel with title, tab strip (Graphics active), PERFORMANCE header, Dynamic Graphics ON, "Graphic Profiles" dropdown "High" (looks enabled), hint "Sets a graphics quality preset for your device." | default, top of scroll |
| 261 | settings-graphics-landscape__1086-4492 | Settings-GRAPHICS-Landscape | Custom Skybox dropdown OPEN-state header ("Morning", chevron up, lavender 1px border) over a 544x290 skybox preview image (Genesis Plaza in morning light) | dropdown expanded, preview image; active tab pill is a lighter pink-lavender here |
| 262 | settings-graphics-landscape__1184-3793 | Settings-GRAPHICS-Landscape | Dynamic Skybox toggle OFF+disabled (dark thumb), "Custom Skybox" label + "Select" dropdown dimmed, warning box "The skybox is set by the creator. It can't be changed." | scene-controlled skybox (SDK), disabled |
| 263 | settings-audio-landscape__1010-4366 | Settings-AUDIO-Landscape | Audio tab active; VOLUME header; sliders General 100%, Scene 100%, UI SFX 100% (cut off at bottom, list scrolls) | default |
| 264 | settings-gameplay-landscape__1058-4090 | Settings-GAMEPLAY-Landscape | Gameplay tab; CHAT & SOCIAL: "Close Chat After Sending" toggle OFF; INTERFACE: "Hide Interface" ON + hint "Hide the on-screen interface to take clean screenshots." | default; note NO Camera section in this frame |
| 265 | detail__1306-1568 | Detail (640x1180) | Tall crop of Gameplay: Chat & Social, CAMERA: "Camera Mode" dropdown "First person" (enabled), INTERFACE: Hide Interface ON, Hide Player Names OFF, Hide "View Profile" OFF | camera selectable |
| 266 | detail__1350-5062 | Detail (640x1180) | Same crop, Camera Mode "Third person" dimmed + warning "The camera mode is set by the creator. It can't be changed." (dark purple box, lavender border); Hide Interface ON, Hide Player Names OFF | camera locked by scene |
| 267 | settings-account-landscape__1063-4461 | Settings-ACCOUNT-Landscape | Tab strip scrolled right (Gameplay, Account active, Storage). Rows: "Term Of Service" (external-link icon), "Discord" (external), "Version 1.10.0" (copy icon), SIGN OUT outlined button, "Delete Account" ruby underlined link | tab strip scrolled; list partly scrolled (HELP header off-screen) |
| 268 | content__1376-7019 | content (544x793) | Full Account scroll content: HELP header, Help, Privacy Policy, Content Policy, Term Of Service, Discord (all with external-link icon), SIGN OUT, Delete Account | complete list |
| 269 | settings-storage-landscape__1063-4851 | Settings-STORAGE-Landscape | Storage tab; TEMPORARY FILES header; "Current Cache Size  0.5 GB" with 50% ruby progress bar; "Max Cache Size" dropdown "1 GB" + hint "Sets how much storage the app can use."; top of a button (CLEAN CACHE) peeking at the bottom | default |
| 270 | options-list__1086-4735 | Options List (fragment) | Open dropdown list: Afternoon, Midday, Morning (selected = lavender text + check icon, row highlighted darker) | dropdown popover |
| 271 | buttons__1086-4776 | Buttons (fragment) | SIGN OUT button: black fill, white 2px border, exit icon + "SIGN OUT" | component |
| 272 | buttons__1086-4770 | Buttons (fragment) | CLEAN CACHE button: same style, text only | component |

## 2. Flow graph

- Entry: in-world HUD avatar "Menu" button (see menu-a report seq 273/275) -> drawer rail; the rail's gear icon opens Settings. In these frames the rail is on the RIGHT edge (avatar 86px at (1507,85), compass 182, backpack 270, gear 358 (active, purple), mini avatar 443), while the Discover/Social drawers (menu-a) have the rail on the LEFT. Flag as inconsistency.
- Settings panel is a right-hand sheet: x 805-1445 (640 wide), y 47 to bottom (bleeds off-screen; scrollable), radius ~24 top corners, fill ~#232228 (near `colors.shadow` #161518 lighter). Scene behind is blurred and darkened (scrim).
- Tabs (horizontal scroll, pills): Graphics -> Audio -> Gameplay -> Account -> Storage. Tapping a tab swaps the content below the (fixed) title + tab strip; the strip auto-scrolls to keep the active pill visible (seq 267/269 show it scrolled).
- Dropdown rows: tap -> expands in place (seq 261: header becomes "Morning" with up chevron and 1px lavender border; list 270 appears as popover/inline list; selecting closes and shows the value).
- Account: external-link rows -> open system browser; Version row tap -> copy to clipboard (toast); SIGN OUT -> sign out (legacy: Global.sign_out); Delete Account -> deletion popup (not designed here).
- Close/back: none drawn (tap outside the sheet or tap the rail's gear/another rail item). Open question.

## 3. Per-screen spec (Figma px at 1600x720)

### Settings panel shell (seq 260-269)
- Panel: left 805, width 640, top 47. Inner padding 48 (content x 853-1397 = 544 wide).
- Title "Settings": Inter Bold ~40px white, baseline y~103.
- Tab strip: pills height 56 at y 164-220, gap 24, radius 100. Inactive: fill ~rgba(255,255,255,0.2-0.3) on dark (grey #4A484E look), icon 28 + label Inter SemiBold ~26 in light grey. Active: fill lavender-purple (#B48CD7 `cardLight` in 260/263/264/267/269; lighter #E8B9FF-ish pink-lavender in 261), text+icon dark (#161518). Icons: monitor (Graphics), speaker (Audio), gamepad (Gameplay), person (Account), archive/box (Storage). Strip clips at the panel edge (partial pill visible, no fade).
- Section header: uppercase Inter Bold ~26, color `lavender`/`cardLight` (#B48CD7 in settings, #E8B9FF for later headers like CAMERA/INTERFACE in 265), followed by 1px rule (same color, ~0.6 opacity) spanning 544. Header y=278, rule y=304.
- Row title: Inter Medium ~32 white; hint: Inter Regular ~26, `gray` #CFCDD4 / white 70%.
- Toggle: 140x64 pill. ON = ruby #FF2D55 track with 2px ruby-light border, white thumb right; OFF = dark #1E1D21 track, white thumb left; DISABLED = track and thumb both dark grey (262). Toggle right edge at x=1397; title+hint left-aligned max width ~360.
- Dropdown: 544x68, fill #161518 (`shadow`), radius 12, value Inter Regular 32 white at x+16, chevron-down 40 at right; open: 1px lavender border, chevron up; disabled: label/value/chevron/hint at ~30% opacity.
- Warning box: 544 x ~94, 1px white (262) or lavender (266) border, radius 12, dark fill (#161518 or deep purple #200035 in 266), triangle-alert icon 48 at left, Inter Regular 28 white 2-line text.
- Slider (Audio): label left + percent right ("100%", Inter Regular 32), track 544x40 radius 8 ruby fill over dark #161518 track, thumb = white rounded rect 28x50 at end of fill. Rows pitch 174.
- Progress bar (Storage): same track as slider without thumb, fill = used/max (0.5 of 1 GB => 50%).
- Buttons (271/272): 544x74, radius ~14, fill #121212, 2px white border, label Inter Bold 32 uppercase white, SIGN OUT has exit icon 32 left of label. "Delete Account": centered text, ruby (#FF2D55-ish pink #F0507A) underlined, Inter Regular 32.

### Graphics (259-262, 270)
Copy: PERFORMANCE; "Dynamic Graphics" / "Adjusts graphics automatically for optimal performance."; "Graphic Profiles" / value "High" / "Sets a graphics quality preset for your device."; VISUAL; "Dynamic Skybox" / "The sky changes over time as you play."; "Custom Skybox" / "Select"; warning "The skybox is set by the creator. It can't be changed."; options Afternoon, Midday, Morning (legacy also Midnight; selected shows check).
Interactions/rules (legacy-confirmed): Dynamic Graphics is mobile-only; ON disables the profile dropdown (259) - 260 shows it enabled, treat 260 as a Figma slip. Profiles: Very Low, Low, Medium, High (Custom is dev-only). Dynamic Skybox ON disables Custom Skybox; turning it OFF enables the dropdown and selects Midday; selecting an option previews the skybox (seq 261 shows a thumbnail preview under the dropdown, not in legacy) . When the scene sets skybox time (`sdk_skybox_time_active`), the toggle and dropdown are disabled and the warning is shown.
Data: local config via bridge (`graphic_profile`, `dynamic_graphics_enabled`, `dynamic_skybox`, `skybox_time`), applied engine-side; `sdk_skybox_time_active` as Godot->RN state event.

### Audio (263)
VOLUME header; sliders General, Scene, UI SFX shown, more below (legacy: General, Scene, Interface/UI, Music & Ambient, Avatar & Emotes, Voice chat, Mic amplification; all 0-100 step 1, default 100, label = value + "%"). Labels in Figma differ ("UI SFX" vs legacy "Interface"). Drag = live apply; persist on release.

### Gameplay (264-266)
CHAT & SOCIAL: "Close Chat After Sending" (persisted `submit_message_closes_chat`, default false). CAMERA (in-world only): "Camera Mode" dropdown First person / Third person; locked state with creator warning. INTERFACE: "Hide Interface" (+hint), "Hide Player Names", Hide "View Profile". Hide Interface ON hides the HUD (restore via X disc, see loading-hud seq 12). Session-only values (not persisted), applied via bridge (`hide_ui`, camera mode, hide names).

### Account (267-268)
HELP header; Help, Privacy Policy, Content Policy, Term Of Service ("Term Of Service" is the Figma copy; legacy "Terms of use" - fix typo), Discord (all open URLs: decentraland.org/help, /privacy, /content, /terms, Discord channel), Version row "Version 1.10.0" + copy icon (copies version), SIGN OUT, Delete Account. Row height 84, icon = external-link 36 right after label (8px gap, NOT right-aligned).
Missing vs legacy: profile header (nickname/#tag/address), guest upgrade card ("Add email"), Report bug / Report content rows (these exist in menu-a as the Discover kebab menu), Language tab (>=2 locales), Developer tab (non-prod).

### Storage (269, 272)
TEMPORARY FILES; "Current Cache Size" value "0.5 GB"; progress bar; "Max Cache Size" dropdown ("1 GB" shown; legacy options 1/2/4 GB, default 2 GB); hint; CLEAN CACHE button (disabled when 0 MB). Data: `get_cache_folder_total_size`, `clear_cache_folder`, `max_cache_size`.

## 4. Reusable components / tokens

Colors (all already in `rn/src/ds/tokens.ts`): ruby #FF2D55 (toggle/slider fill/links), shadow #161518 (dropdown + button fill), lavender #E8B9FF, cardLight #B48CD7 (active tab, headers), gray #CFCDD4 (hints), purple550 #24033B (warning in 266), scrim. Not in tokens: panel grey ~#232228 (settings sheet surface; menu drawers use purple instead), toggle off track ~#1E1D21.
New DS components needed (none exist in `rn/src/ds`): `Toggle`, `Slider`, `Dropdown` (+ options popover), `SettingsSection` header with rule, `WarningBox`, `ProgressBar`, `OutlinedButton` (PrimaryButton exists but is ruby fill), `LinkRow` (external-link/copy icon), scrollable `PillTabs` (existing `Tabs.tsx` is uppercase text with underline, not pills). Existing: `AppText`, `Icon`/`iconSvgs` (need icons: monitor, speaker, gamepad, person, box, external-link, copy, exit, alert-triangle, chevrons, check), `IconButton`.

## 5. Legacy counterpart

`godot/src/ui/pages/settings/settings.gd` + `settings.tscn` (1644 lines), `slider.gd`, `section_title.gd`, `settings_button.gd`, `underlined_button.gd`; molecules `settings_section_item`, `dropdown_list`, `settings_warning`, `guest_upgrade_card`; config in `godot/src/config/config_data.gd`, applied by `GraphicSettings`, `AudioSettings`, `GeneralSettings`, `LocaleSettings`; opened via `Global.open_settings` -> `Menu.async_show_settings()`. Full key table: `docs/rn-ui-spec/reports/legacy-godot.md` section 1a. Legacy behaviours absent from Figma: Language tab, Developer tab, Help & Support (report bug/content), account header + guest upgrade card, account deletion popups/OTP, custom graphics controls (view distance, particles, lights), "Hide interaction prompts / Hide scene UI" sub-toggles, dimmed sub-toggles when master OFF, 3D avatar preview in landscape Account.

## 6. RN status and gap list

`Home.tsx` Settings tab is a placeholder ("Not part of this spike yet."); no settings store, no bridge commands (`get_setting`/`set_setting`/`apply`), no sheet host in `world/WorldScreen.tsx`. Gaps: (1) settings state module + persistence (RN store pushing values to Godot, or bridge get/set) ; (2) bridge events: `sdk_skybox_time_active`, `camera_mode_blocked`, cache size; commands: set_graphic_profile, set_dynamic_graphics, set_skybox, set_volume(bus), set_camera_mode, hide_ui/hide_names, clear_cache, sign_out; (3) DS components listed above; (4) the settings sheet and tab pills; (5) dropdown popover; (6) external links + copy-to-clipboard + toast; (7) skybox preview thumbnails (4 images: midnight/afternoon/midday/morning); (8) landscape lock handling.

## 7. Open questions / inconsistencies

1. 260 vs 259: Graphic Profiles enabled vs dimmed while Dynamic Graphics is ON.
2. 264 lacks the CAMERA section that 265/266 show (camera only exists in-world; confirm it is always present in-world).
3. Active-tab pill colour differs between 261 (pink-lavender) and others (#B48CD7).
4. Rail position: right in Settings, left in Discover/Social drawers.
5. Midnight is missing from the skybox options list (270 shows 3 of the 4 legacy options; may be scrolled).
6. "UI SFX" vs legacy "Interface"; "Term Of Service" typo; Max Cache "1 GB" default vs legacy default 2 GB.
7. No close affordance / back behaviour drawn; Hide Interface ON is shown in 264-266 while the panel is open (it only takes effect after closing?).
8. Frames 265/266 are 640x1180 portrait-shaped crops of the same Gameplay content (design-time scroll captures), not a portrait layout.
9. Language, Developer, Report bug/content, account header and guest upgrade have no landscape design.
