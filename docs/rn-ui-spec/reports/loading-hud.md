# Report: loading-hud (manifest seq 0-12)

Sources: Loading file `mhmvDVAQ2ZaJbfLuJW1VM4` (seq 0-3), HUD file `nxP78pUCEHj3BY0meiXipe` (seq 4-12). All 13 screenshots captured (1600x720 landscape) under `docs/rn-ui-spec/shots/{loading,hud}/`.
Unit convention: Figma px at 1600x720; RN `u(px/2)` (SCALE = min(w,h)/360, rn/src/ds/scale.ts). All sizes below are Figma px.

## 1. Inventory

| seq | id | name | what it shows | variant |
|---|---|---|---|---|
| 0 | loading__loading-1__13-11729 | Loading 1 | Place loading cover: blurred place image, left purple gradient, place logo, title, creator, tip card (lightbulb icon), progress text + bar, close X | tip = generic (bulb icon, emote text) |
| 1 | loading__loading-2-tip-emotes__175-530 | Loading 2 | same, tip icon = emote figure | tip: emotes |
| 2 | loading__loading-3-tip-friends__178-21086 | Loading 3 | same, tip icon = two-people-heart | tip: friends ("Add friends so you can find each other again. Open profile and tap "Add Friend".") |
| 3 | loading__loading-4-tip-chat__178-21128 | Loading 4 | same, tip icon = chat bubble | tip: chat ("Open the chat and start a conversation with people nearby.") |
| 4 | hud__hud__609-7021 | HUD | In-world HUD: avatar/menu button + chat button + 5 chat lines (top-left), joystick bottom-left, emote button below it, gamepad bottom-right (main jump + hand + E + F + "+") | 3 secondaries + "+" overflow closed (this is the "+" present layout) |
| 5 | hud__hud-4-side-buttons__706-6623 | HUD (+4 side buttons) | Same, "+" pressed/active (glow) and combo column 1-4 expanded vertically above it | overflow OPEN |
| 6 | hud__joystick-movement__706-6537 | Joystick Movement | Chat/messages gone (empty chat area), joystick base disappears; only a small thumb disc (~80px) at (456,314) = floating joystick spawned where the touch began; gamepad as seq 4 | joystick active (dynamic/floating joystick), chat list empty |
| 7 | hud__hud-3-secondaries__706-6149 | HUD- 3 secondaries | no chat lines; gamepad: main + hand + E + F, no "+" | scene uses 3 secondaries |
| 8 | hud__hud-2-secondaries__706-6231 | HUD- 2 secondaries | chat lines; main + hand + E (single satellite E raised) + "F"? (screenshot shows E only + hand; see 5.2) | 2 secondaries |
| 9 | hud__hud-defaults__1111-12496 | HUD- Defaults | chat lines; main + hand + E (E at 1285,484) | "defaults" = jump + pointer + primary |
| 10 | hud__hud-1-secondary__706-6304 | HUD- 1 secondary | chat lines; main + hand + E | 1 secondary |
| 11 | hud__hud-no-secondary__706-6373 | HUD- No secondary | chat lines; ONLY the main jump button (no hand, no E) | no secondary (scene hid them / main only) |
| 12 | hud__hideui__706-1414 | HideUI | Everything hidden except a 64px X "restore" disc at top-left (140,64); joystick/gamepad invisible (joystick still works, see 3.4) | Hide-UI on |

Notes: seq 8-10 (and 7) differ only in the number of satellite buttons; they are one component with N = 0..3 secondaries (+ "+" overflow when a scene uses >3, seq 4/5). Reviewer should treat Figma labelling as nominal: in the shots, seq 11 = main only; seq 10 = main + hand; seq 9/8 = main + hand + E; seq 7 = + F; seq 4 = + "+" ... see 5.2 (inconsistencies).

## 2. Flow graph

- Entry: Discover "Jump In" (rn `DetailSheet`) -> `enter_world{x,y,realm}` -> WorldScreen mounts (landscape forced) -> Loading cover (seq 0-3, one tip chosen) -> engine `loading{finished}` -> cover fades -> HUD (seq 4-11).
- Loading X (top-right 60x60 at (1508,32)): cancel loading -> `exit_world` -> back to Discover (portrait). Legacy: `close_button` in loading_screen.gd cancels (`_loading_cancelled`) and returns to previous place/discover.
- HUD: avatar/menu button (82px) -> opens the Menu hub (other reports: Navbar/profile/menu); chat button (66px) -> chat panel (chat report; the Figma for the open chat is outside seq 0-12, only message list shown); emote button (80px, bottom-left under the joystick) -> Emote wheel; Hide-UI is entered from Settings/menu (explorer `_session_hide_main_hud`) -> seq 12; X on seq 12 restores the HUD (`Button_ShowUI`).
- Back behaviour: Android back in world = open menu (legacy) / never exit directly; Loading X is the only exit during load. If loading exceeds 90 s legacy shows the "Loading is taking longer than expected" modal with RUN ANYWAY (not in Figma; keep).

## 3. Per-screen spec

### 3.1 Loading (seq 0-3)  - 100% RN (replaces godot/src/ui/pages/loading_screen)
Layout (1600x720):
- Background: place image full-bleed, blurred + a left-to-right gradient scrim (solid dark purple ~#2a0d44 at x=0 fading to transparent by x~1050, plus ~25% purple tint over the whole image).
- Place logo/thumbnail: circle 58x58 at (80,96) (Decentraland logo when the place has none).
- Title: "Genesis Plaza" Inter Bold ~64px, white, wraps in 2 lines, left 80, width ~300 (top 180, line height ~78). Title is the place title (`Place.title`).
- Creator: "By Decentraland Foundation" at (80,348); "By" Inter Regular 26 (#DF9CFF-ish light in legacy; here white-ish), name Inter SemiBold 26 white. Legacy string `LOADING_CREATED_BY` = "[color=#DF9CFF]By[/color] [b]{creator}[/b]".
- Tip card: x80 y441, 600x84, 1px border lavender (#E8B9FF / #B48CD7 range), radius 4, fill rgba purple ~(70,15,111,0.7). Left icon 32px at x105 (bulb | emote | friends | chat), text at x154: "**Tip:** " Inter SemiBold + body Inter Regular 20px, white, 2 lines, color #FCFCFC. Format string `TIP_CARD_FORMAT` = "[b]Tip:[/b] {tip}".
- Progress row: label left at (80,637) "00/00 resouces at 0.10 mb/s" (sic: typo "resouces" in Figma; fix to "resources") Inter Regular 20 white; percent right-aligned at x1520 "10%" Inter Bold 20. Bar: x80..1520, y661, h14, radius 7; track #1b1b1b-ish dark (rgba 0,0,0,0.85), fill purple gradient (#691FA9 -> #A14BF3 style) at 10% shown as ~57% (the shot shows fill to x=908 = 57%: Figma mock mismatch with "10%", see 5.2).
- Close X: 60x60 at (1508,32), radius 8, fill rgba(0,0,0,0.5), white X 28px.
Safe area: content is positioned 80px from left/bottom/top in Figma, i.e. wrapper must add `insets.left/right/top/bottom`; the image/gradient bleed under the notch.
Tips (copy, localised via i18n keys already in legacy `godot/locale/en.csv`): `TIP_EMOTES` "Use Emotes to wave, react, or celebrate. Tap the Emote icon to open the Emote Wheel."; `TIP_FRIENDS` (Figma text differs: "Add friends so you can find each other again. Open profile and tap "Add Friend"." vs legacy "Decentraland is better with friends..."); chat tip is new (no key); `TIP_CREATOR_HUB`, `TIP_WEARABLES` exist in legacy but have no Figma variant; `TIP_CARD_PLACEHOLDER` before data. Rotate tips (legacy picks random per load); icon follows tip category: generic=lightbulb, emotes, friends, chat.
Data: place title/creator/image from places API (`/api/places?positions=x,y` or worlds by name) - already in rn `Place` (discover/api.ts); progress from bridge `loading` events (see 6; currently only `started|finished` - gap); resource count/speed need a new `loading_progress{loaded,total,mbps,percent}` event.
States: pre-data (title "Enjoy your Experience", no creator, no image: legacy waits 0.35 s `CARD_IMAGE_GRACE_S` for the image), loading, slow (>90 s modal), failed realm (legacy `realm_change_failed` hides screen and shows error), finished (fade out).

### 3.2 HUD base (seq 4, 9) - boundary: see section 4
Top-left (RN, Menu-Chat group): starts at (91,39), gap 32.
- Menu button 82x82 (avatar snapshot circle 72 inside a 82 ring with white 2px/ white glow ring; image = player avatar face snapshot; fallback silhouette). Tap -> open Menu.
- Chat button 66x66 circle, fill rgba(0,0,0,0.5)/dark purple #24033B-ish, 2px translucent white border, white outline chat-bubble glyph 28px. Unread badge not drawn (legacy `chat_button.gd` has `_get_unread_count`).
- Chat message list ("notifications" mode, closed chat): column starting at x=90.57, y=135 (i.e. 14 below the buttons), stacked 38.67 apart (row h ~34), width hugs content up to ~337 (max text width 218.67 after the 8.86px name column -> total ~337). Each row: bg rgba(0,0,0,0.4) (own message rgba(0,0,0,0.7)), radius 5.33, padding 6.67 x 5.33, gap 5.33. Inside: sender block = name (Inter SemiBold 8.86 in the sample but the screenshot shows ~19px: Figma frame is half-scaled; treat as 18.67/2 -> **render name at same size as message, 18.67px**, color from name palette: #FF7439 orange shown; mention/own = #FF4BED mythic pink) + "#1l0u" tag (Inter Regular, rgba(255,255,255,0.5)); message Inter SemiBold 18.67 #CFCDD4 (system line) / white (own, #fff). Own/last row sample: "Teleported sdk sjflk s vnsd vnds vnkv xcv dxx . dcsdcc sd nmlk" wraps at 218.67 width.
- System lines are "Clara #1l0u Teleported" (teleport join/leave notice from comms); 4 grey + 1 highlighted (own) row in the mock.
- Behaviour (match legacy `notifications.gd`): keep max 10 rows, each auto-removes after 6.5 s, new rows push at the bottom; list is click-through (`pointerEvents="none"`) unless overflowing; truncate to 80 chars with "..." (legacy MAX_CHARS_REDUCED); chat button tap switches to the full chat panel (hides notifications).
- Open chat (not in this range; target Figma "chat refactor"): reading mode width 420 (legacy `WIDTH_OPEN`), writing mode keyboard-avoiding input; mention autocomplete (3 results), URL/coords/mentions clickable, avatar color nick, claimed-name check icon, system welcome message. RN must reimplement all of these (see 6).
Bottom-left (Godot): joystick base 172x170 outline at center ~(247,527), thumb 64px at center; resting position safe-area-relative. Emote button 80x80 (left 20, top 593) is an RN-or-Godot overlay: legacy it is an emote_wheel trigger in the Navbar -> **RN** (it is HUD chrome, not a gameplay input); glyph = dancing figure 48px, fill rgba(0,0,0,0.5)+border.
Bottom-right (Godot): see 3.3.

### 3.3 Gamepad variants (seq 4,5,7-11) - Godot native (`Joypad`)
Btn-HUD-Controls: disc 80px (back asset 80, label/icon 56 inside at opacity .8, glyph color --iconhud #DFD0FF, font SF Pro Rounded Medium 41px for letters). Main button 156/160px (icon 100 up-arrow = jump; `HUD-Icons-Btn` 100). Positions from seq 4 (centres, Figma px): main (1410,591); hand/pointer (1245,633); E (1255,523); F (1342,438); "+" (1457,426). Satellite arc fills priority order: jump (main), pointer, primary(E), secondary(F), action_3.. ("1","2","3","4" in combo column); mapping from legacy `joypad.gd` PRIORITY_ORDER = ia_jump, ia_pointer, ia_primary, ia_secondary, ia_action_3..6. The first visible action becomes the big central button; scene `main_action` moves its action to the front; `JoypadArc` places satellites tangent to the bottom edge ... last tangent to the right edge. Overflow "+" is the LAST arc element; when open (seq 5) it glows (purple ring, filled) and a vertical column "1,2,3,4" stacked above (centres x=1457, y=337,251,166,79; 86px pitch, discs 80) with the main button nudged to (1418,589). Counts: 0 secondaries -> main only (seq 11), 1 -> +hand (10), 2 -> +E (8/9), 3 -> +F (7), more -> "+" (4) opens combo (5).
Glyphs: jump arrow (double jump / glider variants from state), hand (pointer/tap), letter keys E/F for primary/secondary, digits for combo; scene can replace icons via `PBTouchScreenControls` (`sdk_touch_controls_applier.gd`).
Joystick (seq 6): when touch begins in the left half the base is hidden and a ~80px translucent thumb marks the touch point (floating joystick); base rest position is bottom-left otherwise (seq 4). `PBTouchScreenControls.hide_joystick` hides it.

### 3.4 HideUI (seq 12)
Only a 64px dark disc with white X at (140,64) (legacy `Button_ShowUI`, top-left of the safe area). Menu, chat, notifications, emote and the gamepad are hidden; the joystick stays functional but alpha 0 (legacy `_apply_mobile_controls_hide_ui`), camera drag still works; scene SDK UI stays (SceneUIContainer). RN: render only the restore button; tell Godot `hide_ui{hidden:true}` to hide Joypad; unhide on tap.

## 4. RN / Godot boundary (key deliverable)

Z-order (bottom -> top): Godot 3D world (surface) / Godot native HUD (joystick, gamepad, MobileCameraInput, SceneUIContainer, version/FPS labels - `RnUiFilter._is_native`) / RN overlay view (top-left HUD, loading cover, modals). RN root is `pointerEvents="box-none"`, and since commit 0d72913c9 the SafeAreaProvider is also transparent to touches, so only RN views that are real touch targets consume input; everything else falls through to Godot (camera look, joystick, gamepad, scene UI).

RN owns (100% React Native, must not exist in Godot in RN mode): top-left Menu button, Chat button, chat message list / notifications / chat panel, emote button, Hide-UI restore button, loading screen, Navbar panels (friends, notifications, discover), emote wheel (separate report), toasts/modals.
Godot owns (native, hidden-by-RN-never): Joystick (VirtualJoystick_Left, floating), gamepad Joypad (main + up to 3 satellites + "+" + combo 1-4) incl. glyph/priority logic driven by the scene SDK (PBTouchScreenControls, input actions, main_action), MobileCameraInput (drag to look), SceneUIContainer (SDK UI), crosshair, interaction pill/tooltip, version/FPS labels.
Bridge additions needed (not existing): `hud_state` Godot->RN {joypadVisible, joypadBounds[x,y,w,h]} so RN can keep its overlay clear of the gamepad; RN->Godot `set_hud_insets{left,top,right,bottom}` (pixels in window space) so the joypad/joystick respect RN's safe area; `hide_ui{hidden}`; `chat_open{open}` (legacy hides emote button + joypad while chat is focused: replicate by Godot hiding Joypad on this command); `loading_progress`; `unread_chat{count}`.

Safe-area insets (legacy `hud_margins.tres`, in 720-high viewport px): landscape left 108, top 32, right 108, bottom 45 (portrait 40 left/right). This is a floor on top of device insets (notch/gesture bar): effective inset = max(device inset, floor). Figma agrees: Menu at x=91 (+ ~17 avatar halo), gamepad right edge at ~1500 (=1600-100), joystick left 160. RN equivalent: `useSafeAreaInsets()` + floor `{left: u(54), top: u(16), right: u(54), bottom: u(22.5)}` (floor/2 in dp) i.e. Menu-Chat group left = max(insets.left, u(54)) with Figma x=91 -> 45.5dp; keep consistent: x = insets.left + u(16)?? Spec: use Figma coords relative to the notch-free 1600x720 frame and clamp with max(inset, floor).
Scene interactable area (legacy `get_interactable_area`): safe rect minus chat column (`Global.chat_notifications_width`, notifications column ~ 337-420 wide at left). Godot pushes it to scenes (`set_interactable_area`, `set_safe_area`). With RN owning the chat, RN must report its chat column width to Godot (`set_chat_width{px}`) or the SDK `UiCanvasInformation.interactable_area` will be wrong.

Touch passthrough regions (RN `box-none`, only these rects intercept):
- Menu button (82x82 @ (91,39)) + Chat button (66x66 @ (205,47)); gap 32 is passthrough.
- Chat list: `pointerEvents="none"` while not overflowing/scrolling; when the open chat panel is shown the whole panel rect (x=insets.left, w=420, y=top..bottom) is a touch target, plus a full-screen tap-catcher that closes panels/chat when tapped outside (legacy HUD dismiss catcher, active only when a panel/chat is open).
- Emote button 80x80 @ (20,593) (just below joystick, so the joystick's reachable area (172x170 @ ~(161,442)) and the button must not overlap; joystick floating zone = left ~40% of the screen, below chat region).
- Everything else, incl. the whole right side gamepad rects (main 160, satellites 80, combo column x=1417..1497, y=39..467), is NOT covered by RN: passes to Godot. RN must keep any element out of x>=1180 and y>=380 in landscape.
- Loading cover (seq 0-3): full-screen intercept (stops touches reaching Godot) except none; only X closes.
- HideUI: only the 64px X intercepts.
Keyboard: chat input uses KeyboardAvoidingView; when keyboard is open Godot should receive nothing (legacy `WRITING` state hides chatbar in landscape, safe bottom area hidden).

## 5. Tokens, components, inconsistencies

5.1 Tokens (from get_design_context HUD 609:7021): DCL/Orange #FF7439 (sender), Gray #CFCDD4 (system text), Mythic #FF4BED (own/mention highlight; legacy mention `#FC03EC`, gold `#FFD700` for @mentions of others), White #FFF, Silver #A09BA8, Black40% rgba(0,0,0,.4), own rows 0.7, radius 5.33, iconhud #DFD0FF, font Inter (SemiBold/Regular), keys use SF Pro Rounded Medium (RN: use Inter-Medium or bundle the font; iOS has SF Pro Rounded via `fontFamily: 'System'` w/ design rounded not available in RN -> accept Inter). Loading uses lavender #E8B9FF border, purple gradient fill. All exist in `rn/src/ds/tokens.ts` (orange, gray, silver, lavender, scrim, purple450/550 ...) except mythic pink #FF4BED (in `rarityColors.mythic.dark`) and iconhud.
Existing ds: `AppText`, `IconButton`, `Pill`, `LinearFill`, `Icon` (check iconSvgs for chat/bulb/emote/friends/X; likely missing: lightbulb, chat-bubble, dancing figure, friends-heart, up-arrow jump, hand, X). New components to build: `LoadingScreen`, `TipCard`, `ProgressBar`, `HudAvatarButton`, `HudCircleButton`, `ChatNotificationList`, `ChatRow`, `ChatPanel`, `ChatInput`, `MentionList`, `EmoteButton`, `HideUiRestoreButton`.

5.2 Inconsistencies / questions
- Figma frame names vs content: "HUD- Defaults" (seq 9), "HUD- 2 secondaries" (8) and "HUD- 1 secondary" (10) look identical (main + hand + E) in the renders; only seq 7 (+F), 4 (+F,+"+"), 11 (main only) are distinct. Confirm what 1 vs 2 secondaries should show (is the hand pointer a "secondary" or part of the base?). Legacy order treats pointer as the 2nd action, so hand + E + F = 3 satellites; consequently seq 7 (hand,E,F) = "3 secondaries", seq 9/8/10 = 2 satellites?? Needs designer confirmation.
- Chat sizes: Figma text nodes in design context say 8.86/18.67px (mixed after auto-scale); the visible rendering is ~19px names. Use 18.67.
- Progress label "00/00 resouces at 0.10 mb/s" typo; bar fill 57% vs "10%".
- Friends tip copy differs between Figma and legacy `TIP_FRIENDS`; chat tip has no key; update `godot/locale/*.csv` or the RN i18n catalogue.
- Seq 6 shows joystick-active state without chat rows: is the chat list hidden while moving, or just an empty chat?
- Menu/profile avatar button has no unread/notification dot in Figma, legacy navbar has notifications; clarify where RN shows the bell/friends entry (Menu hub).
- Emote button: RN vs Godot ownership (proposed RN).
- Open-chat Figma for the "chat refactor" is not in seq 0-12; this report only defines the closed (notification) state.

## 6. Legacy counterparts and RN status

Legacy:
- Loading: `godot/src/ui/pages/loading_screen/loading_screen.gd/.tscn`, `loading_screen_progress_logic.gd` (progress = loaded assets / expected count `_expected_asset_count`, activity timeout, 90 s hard cap `MAX_LOADING_TIME_SECONDS`, thumbnail grace 0.35 s, `RUN ANYWAY` modal, `realm_change_failed`, low-spec toast, `TIP_*` keys).
- Chat: `godot/src/ui/pages/chat/{chat_panel,chat,chat_message,notifications,chat_button,mention_item}.gd/.tscn` (states CLOSED/OPEN/WRITING, width 420, msg fonts landscape 18/20 and portrait 24/26, backgrounds other 0.4 / own 0.7 / system 0.4, max 10 notifications 6.5 s, bbcode escaping, URL safety `_is_safe_url`, coordinates click -> teleport, mention click -> profile, claimed-name check icon, "system" address welcome `CHAT_SYSTEM_WELCOME`, Android nav-bar insets `async_apply_system_bar_insets`).
- HUD: `godot/src/ui/explorer.gd/.tscn` (`SafeAreaHud`, `InteractableHUD`, `SafeAreaControls/MobileUI/Joypad`, `VirtualJoystick_Left`, `Button_ShowUI`, `set_visible_ui`, `_apply_mobile_controls_hide_ui`, `_push_scene_interactable_area`, `hud_margins.tres`), `ui/components/organisms/joypad/joypad.gd` + `joypad_arc.gd`, `ui/components/utils/sdk_touch_controls_applier.gd`, `rn_bridge/rn_ui_filter.gd` (what stays native).
Behaviour absent from Figma but in legacy: chat commands (`/` commands, share_place), mention autocomplete, unread count, hide emote+joypad while chat focused, preview-mode HUD panel, 90 s loading modal, "scene not responding", pointer tooltip/interaction pill, crosshair, dismiss catcher.

RN status (rn/src):
- `world/WorldScreen.tsx` (256 lines, spike): placeholder cover (gradient + image + "Jumping in…" spinner, 25 s timeout), top bar with chip "{title} · {parcel}", Wave / Chat / Leave text buttons, simple chat ScrollView + TextInput (shows `address.slice(0,6)`), `send({type:'send_chat'})`, `emote`; orientation lock via `setOrientation('landscape')`.
- `events.ts`: `loading{started|finished}`, `state{parcel,realm,fps,position}`, `chat{address,message,timestamp}`, reducer keeps 50 lines. `engine.ts` `useEngine().hud`.
- Gaps: (1) LoadingScreen to Figma (tip rotation, creator, progress text/bar, close X, 90 s modal); (2) loading progress event; (3) chat profile resolution (address -> name, #tag, color, claimed-name check) - no profile event/API in bridge; (4) message model for system/own/mention, URL/coord/mention parsing; (5) Hud top-left (Menu avatar snapshot, chat button, notifications list with 6.5 s expiry, unread); (6) chat panel open/writing, keyboard handling, autocomplete; (7) emote button + wheel; (8) HideUI state + `hide_ui` command; (9) insets/passthrough contract with Godot (`set_hud_insets`, `set_chat_width`, `chat_open`, joypad bounds); (10) remove spike buttons Wave/Leave (Leave moves to Menu); (11) icons missing in `ds/iconSvgs.ts`; (12) scene interactable area must include RN chat width; (13) mock bridge needs loading_progress + system chat samples.
