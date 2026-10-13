# Profile (view, Edit Profile, Add Link, dropdowns) - UI spec

Source: Figma file `l40AKWaS456BvehAO5PiRu`, manifest seq 176-219 (44 captures). Screens: portrait 720x1600 (Profile, Profile-FullView, Edit Profile, AddLink modal), landscape 1600x720 (Profile-Landscape), 1 tall edit frame 720x3060, 4 dropdown fragments 624x124(540 shot).
All shots at `docs/rn-ui-spec/shots/profile/<id>.png`. Figma px; RN `u(px/2)`.
`get_design_context` used once (node 270:8413, Edit Profile) - tokens in section 4. Limit not hit; the other 2 calls were not needed.

## 1. Inventory (every seq)

Distinct screens: **P1** Profile portrait (bottom sheet), **P2** Profile full view, **L1** Profile landscape, **E1** Edit Profile (portrait), **M1** Leave-without-saving modal, **M2** Add Link modal, **D1** dropdown/option list.

| seq | id (node) | name | shows | state / variant |
|---|---|---|---|---|
| 176 | 232-2269 | Profile-Landscape | card: avatar left, name+#tag, address, EDIT PROFILE, X, ABOUT ME description (2 lines), SEE MORE, EQUIPPED carousel w/ labels+rarity | L1 description-mode, collapsed |
| 177 | 232-2806 | Profile-Landscape | description + all 10 about fields in 3-col grid, SEE LESS clipped at bottom | L1 expanded (equipped pushed off, card scrolls) |
| 178 | 9-509 | Profile | sheet at y=584: name/#1234, copy icons, address, EDIT PROFILE, ABOUT ME text, 2-col fields (clipped by nav) | P1 half-open, own profile, dark-purple avatar bg |
| 179 | 49-1616 | Profile-FullView | sheet fully up: X close, same header, ABOUT ME, 10 fields, LINKS chips (Instagram, GitHub, Youtube, Web, Discord) | P2 full |
| 180 | 270-8413 | Edit Profile | header, avatar snapshot 200px, Username "Carlos" (plain, no counter), Description "text" 0/300, Links + ADD LINK, Country "Albania", footer CANCEL/SAVE | E1 filled, plain username (no claimed/tag UI) |
| 181 | 270-9413 | Edit Profile | blurred E1 + dialog "Leave without saving?" / "You'll lose any unsaved changes." LEAVE (secondary) / STAY (ruby), warning icon | M1 modal |
| 182 | 270-9977 | Profile | same as 178 | P1 duplicate (identical) |
| 183 | 415-8560 | Edit Profile | Username "text" + "#1234" suffix, counter 0/15, Country placeholder "Select" (cut by footer) | E1 non-claimed user, field shows tag |
| 184 | 417-9081 | AddlLink-Modal | "Add Link": Link title (placeholder "e.g. Instagram", 0/15), Link ("Link URL ("https://")"), CANCEL / SAVE (SAVE dimmed) | M2 empty, save disabled |
| 185 | 417-11210 | AddlLink-Modal | title "Instagram" focused (1px light border, clear X), 9/15; URL empty | M2 title focused |
| 186 | 418-11432 | AddlLink-Modal | URL focused, empty, system "Paste / Clipboard" toolbar overlaid | M2 URL focused (OS toolbar, not ours) |
| 187 | 418-11672 | AddlLink-Modal | URL "https://www.instagram.com/ru..." truncated, SAVE still dimmed | M2 filled (see open question 1) |
| 188 | 415-8733 | Edit Profile | Links shows chip "Instagram x" (link icon, delete x); ADD LINK button replaced/hidden after add (only 1 link; see Q) | E1 with link chip |
| 189 | 491-8375 | Edit Profile | Username has tabs UNIQUE NAME (selected, ruby underline) / NON-UNIQUE USERNAME; dropdown "Name" + claimed badge (ruby seal+check) | E1 claimed user, dropdown closed |
| 190 | 491-8558 | Edit Profile | same, dropdown open: Name 01, Name 02, Name (selected, check) | E1 name dropdown open, sel=Name |
| 191 | 491-9803 | Edit Profile | same, open, Name 02 selected | E1 dropdown open, sel=Name 02 |
| 192 | 491-10036 | Edit Profile | dropdown closed, "Name 02" chosen | E1 after select |
| 193 | 270-10604 | Profile-Landscape | identical to 176 | duplicate |
| 194 | 270-11285 | Edit Profile | identical to 183 | duplicate |
| 195 | 491-8739 | Edit Profile | NON-UNIQUE tab selected: input placeholder "username" (dim) + "#1234", 0/15 | E1 non-unique, empty |
| 196 | 491-8960 | Edit Profile | non-unique, "Other Name" focused (border + clear x before #1234) | E1 non-unique, focused |
| 197 | 491-9149 | Edit Profile | non-unique "Other Name #1234" unfocused | E1 non-unique, filled |
| 198 | 271-6872 | Profile | identical to 178 | duplicate |
| 199 | 31-1608 | Edit Profile | identical to 189 | duplicate |
| 200 | 54-2414 | Edit Profile | scrolled: header stays; Username greyed/disabled "text"; Description; an option list ("Option" x5, last selected+check) open ABOVE a field "A" (chevron up); Pronouns "Select" below | E1 scrolled + dropdown open upward (placeholder copy) |
| 201 | 86-2087 | Edit Profile | scrolled: Links, Country dropdown open (Option x4, "Label" selected), then Sexual Orientation, Relationship "Select" | E1 scrolled + dropdown open downward |
| 202 | 86-2417 | Edit Profile | Username greyed "text" (disabled look), avatar, Country "Albania" | E1 disabled-username variant |
| 203 | 144-2750 | Edit Profile | Android keyboard up (Samsung kbd), Description focused area visible above kbd, no footer | E1 keyboard open |
| 204 | 144-2195 | Edit Profile | keyboard up, content scrolled 128px (avatar half hidden under header), description counter visible | E1 keyboard + scrolled |
| 205 | 144-2419 | Edit Profile | keyboard up, country option list (Brasil, Argentina, Argelia, Albania, Select(selected)) open above field | E1 keyboard + dropdown |
| 206 | 270-10394 | Edit Profile | identical to 180 | duplicate |
| 207 | 270-10205 | Profile | as 178 but avatar background light lavender gradient (E8B9FF-like) | P1 alt avatar background (per-user bg / theme) |
| 208 | 303-4252 | Profile-Landscape | ABOUT ME shows only data (Country, Language, Pronouns), no description, no SEE MORE; EQUIPPED | L1 data-mode, 3 fields only |
| 209 | 303-5189 | Profile-Landscape | same + SEE MORE (more data exists) | L1 data-mode collapsed w/ more |
| 210 | 303-5658 | Profile-Landscape | 6 fields (2 rows) + SEE LESS; EQUIPPED cards scrolled so labels clip | L1 data-mode expanded |
| 211 | 242-2171 | Edit Profile (720x3060) | full-length form, NO footer/nav: Username(text greyed), Description, Links, Country (TEXT field), Pronouns, Gender, Sexual Orientation, Relationship, Employment Status, Profession, Real name, Hobby | E1 full spec / scroll length reference |
| 212 | 491-9606 | Edit Profile | non-claimed username "username" + #1234, 0/15 | E1 (same as 183 but value) |
| 213 | 244-2980 | Dropdown | Pronouns open: He/Him, She/Her, They/Them, Ze/Zir, Xe/Xem | D1 |
| 214 | 244-3045 | Dropdown | label "Gender" but options Heterosexual..Asexual (copy bug) | D1 |
| 215 | 244-3076 | Dropdown | Sexual Orientation open: Heterosexual, Bisexual, Lesbian, Gay, Asexual | D1 |
| 216 | 254-3614 | Dropdown | Relationship open: Single, In a relationship, Engaged, Married, It's complicated | D1 |
| 217 | 340-4509 | Profile | sheet at y=582 (same as 178, avatar 12px bigger/zoomed) | P1 half (current) |
| 218 | 340-5133 | Profile | sheet at y=718: header, EDIT PROFILE, EQUIPPED carousel (no ABOUT ME) | P1 profile with no about data / sheet lower snap |
| 219 | 340-5575 | Profile | sheet collapsed to peek (top 1352, handle only), avatar full body, nav visible | P1 collapsed |

## 2. Flow graph

- Entry: bottom nav "Profile" tab (avatar ring icon) -> P1 own profile (half-open sheet over 3D avatar). Also (not in Figma, legacy): tap avatar in world, chat/friends list -> `open_profile_by_address` (other user; Figma only shows OWN profile).
- P1: drag handle up -> P2 (full); drag down -> half -> peek (219). Tap handle toggles (same as backpack Sheet). Tap X in P2 -> back to half (inferred).
- P1/P2/L1 "EDIT PROFILE" -> E1 (full-screen, replaces sheet + nav bar hidden; header back chevron + title).
- E1: back chevron or CANCEL -> if dirty -> M1 (LEAVE = discard + back to P1; STAY = dismiss). If clean -> back to P1 directly (inferred).
- E1 SAVE -> optimistic close to P1 with new values, deploy in background; on failure re-fetch/rollback (legacy). No toast shown in Figma.
- E1 ADD LINK -> M2. M2 CANCEL / backdrop tap -> E1; SAVE -> chip added to E1.
- E1 link chip x -> removes immediately (no confirm).
- E1 tabs UNIQUE / NON-UNIQUE toggle username input vs claimed-names dropdown (tabs only when user owns names).
- Dropdown fields -> inline popup list (D1) with check on selected; close on selection or outside tap.
- Landscape (L1): opened from HUD side menu avatar button (right pill menu: friends, bell, backpack, avatar(active), settings); X closes. EDIT PROFILE from landscape switches to portrait edit and returns to landscape on close (legacy `_opened_from_landscape`).

## 3. Per-screen spec

### P1 / P2 Profile (portrait, 720x1600)
Layout (Figma px):
- Background: Godot 3D avatar preview full-bleed (RN: transparent area; Godot renders avatar), gradient purple (#6B1FB5 -> #A24BF3 top, magenta bottom-right #C040D8). 207 uses light lavender gradient: avatar bg varies (probably selected backdrop/theme); treat as parameter.
- Dynamic-island safe area top 24..92 (black pill 234x68 is device mock; not UI).
- Bottom sheet: x 0..720, rounded top radius ~32 (Figma 24-32px), fill #470071-ish dark purple (gradient to #350854 downward), top edge y=584 (half), 718 (lower), 1352 (peek). Handle bar 200x8 centered, y~604 (20px below top), color white30. Nav bar (existing BottomNav, height 180 incl 40 bottom pad) overlays bottom.
- Content (padding 48 horizontal): name row y~680: nickname 44px/Bold white + "#1234" 32px Regular silver; copy icon (36) at right x~640 copies name+tag. Address row y~748: "0x4e1...1c13B" 30px Medium lavender (#E8B9FF-ish), copy icon after it copies full address. Claimed users: tag hidden, ruby claimed seal shown after name (legacy).
- EDIT PROFILE: outline button x48..672, h96, radius 16, 1px border white30, fill black ~40%, pencil icon 36 + label "EDIT PROFILE" Bold 36 uppercase. (Only for own profile; other users get friend/block/mute buttons - NOT in Figma.)
- ABOUT ME: heading 28/Bold uppercase lavender; description 32/Regular snow, line-height ~34; shows full text in portrait (no truncation).
- Fields: 2-column grid (col x 48 and 372), each: icon (24) + label 24/Medium uppercase silver/lavender-ish (#A09BA8-ish purple), value 28-30 Regular snow below. Order: COUNTRY, LANGUAGE, PRONOUNS, GENDER, RELATIONSHIP STATUS, SEXUAL ORIENTATION, EMPLOYMENT STATUS, PROFESSION, REAL NAME, HOBBY. Row pitch 77. Empty fields hidden (legacy `has_value`).
- LINKS (P2): heading "LINKS"; wrapping chips: link icon + title, 36px Medium color #9AA0FF-ish (periwinkle), bg black 40% radius 16, h~64, gap 24/24. Tap opens URL externally (legacy `OS.shell_open`).
- Equipped (218, also below About in scroll): heading "EQUIPPED"; horizontal scroll of cards 150 wide: rarity-gradient square with category corner badge, name "Asset Name" 24, rarity pill (BASE, LEGENDARY, UNIQUE, COMMON, EXOTIC). Matches existing `ItemCard`. Includes emotes; tap emote plays it on the 3D avatar (legacy), tap again stops.
- P2: X at (60,115), sheet fills screen, avatar hidden; content scrolls.
Interactions: sheet drag (3 snaps: peek 1352, half 584/718, full 0), copy-to-clipboard (needs a toast/haptic - not drawn), scroll in sheet when full, drag-to-rotate avatar in Godot (legacy `can_move`).
Data: own profile from catalyst (`/lambdas/profiles/<addr>` -> avatars[0]: name, hasClaimedName, description, country, language, pronouns, gender, sexualOrientation, relationshipStatus, employmentStatus, profession, realName, hobbies, links[{title,url}], avatar.wearables/emotes URNs); wearables via content server; avatar rendering via Godot bridge (`set_avatar`/preview already used by backpack). Rust-bridge needed for: profile read + deploy + names list.
States to build: loading (skeleton name/rows), loaded, empty-about (hide About block; 218), no links (hide LINKS), error (retry), peek/half/full snaps, own-vs-other (other not designed).

### L1 Profile landscape (1600x720)
- Floating card x105..1440, y50..670, radius ~32, same purple gradient, magenta glow bottom-right; right of it the HUD pill menu (x1456..1556) is visible (not part of this screen).
- Left 3D avatar (full body, x~200..350). Right column starting x441: name 40/Bold + #tag 30 silver; address 26 lavender + copy; top-right buttons: EDIT PROFILE outline (x1125..1332, h60) and X square (1350..1408, 58) black 40%.
- ABOUT ME heading y236; below either description (176/177) or data grid (3 columns x441/773/1105, row pitch ~81) - mode logic: description-only if no data; data-only if no description (208-210); BOTH -> description (2 lines, "SEE MORE" underlined link, periwinkle, right-aligned) and tapping expands to full description + all data in 3-col grid, link becomes "SEE LESS" (177/210). Collapsed data-mode shows first 3 fields (country, language, pronouns); "SEE MORE" appears only if more exist.
- EQUIPPED heading + horizontal carousel of cards 150x150 art + name + rarity pill (labels clipped when expanded, card scrolls vertically).
- No LINKS section drawn in landscape (legacy has ProfileLinks in the card; confirm).

### E1 Edit Profile (portrait)
Regions (720x1600): Header 190 tall (safe-area pill + row: back chevron 60 at x60, title "Edit Profile" Inter Bold 48 snow; fixed, content scrolls under it with background gradient behind: 270/86 shots show scrolled content passing under header). Footer 200 tall fixed: bg #470071, padding 18/48/40, two buttons h96 radius 16 gap 18: CANCEL (black 40%, snow Bold 30 uppercase) | SAVE (ruby #FF2D55). Footer replaces BottomNav. Scroll area between, content padding 48, section gap 64, 32 gap between avatar and first field.
Content order and exact copy:
1. Avatar snapshot 200px circle (153 image inside ruby ring 4-6px), centered; face snapshot from profile (not editable here; legacy shows `ProfilePicture`).
2. **Username** (label 30/Medium snow). Variants:
   - plain text input (180/202/206): value "Carlos", no counter.
   - non-claimed (183/194/212): input + right-aligned "#1234" tag (silver 26), counter "0/15" below (20/Regular pearl). Clear "x" appears while focused (196).
   - claimed owner (189-192, 199): tabs "UNIQUE NAME" | "NON-UNIQUE USERNAME" (24/Medium uppercase, selected: snow + 4px ruby underline; unselected: white70). Unique tab -> dropdown field showing name + claimed seal (ruby 8-point badge with check); list items "Name 01", "Name 02", "Name" each with seal; selected row has lavender text + check on right, darker row bg. Non-unique tab -> text input with #tag.
   - disabled variant (200/202/203/204/211): text dim (white50), bg lighter - used when name can't be edited (e.g. guest; legacy has no such state - confirm).
3. **Description** label + helper "Describe yourself in a few words." (20 pearl); textarea h216 radius 12 bg black 40% pad 16, text 30/Medium; counter "0/300".
4. **Links** label + helper "Add up to 5 links to your website or social profiles."; ADD LINK button h68 radius 16 bg black40 px24, "+" icon + "ADD LINK" Bold 22 uppercase. Added links render as chips (link icon, title in periwinkle, x to delete) wrapping; ADD LINK button stays last and disappears at 5 links (legacy).
5. **Country** dropdown (180/188: "Albania" selected; empty shows "Select" at 50% alpha). In 211 it's a text field - see Q.
6. **Pronouns**, **Gender**, **Sexual Orientation**, **Relationship** dropdowns: field h68 radius 12 bg black40, label 30/Medium white (placeholder "Select" 50% alpha), chevron 40 (down closed / up open). Open: floating list (bg #1F0033-ish, radius 12, 1px border purple-lavender, rows pitch 76, text 30/Medium; selected row: lavender text, rounded inner highlight, check icon right); list opens upward if no room below (200, 205). Max 5 rows visible in Figma (legacy lists more -> list must scroll).
7. **Employment Status**, **Profession**, **Real name**, **Hobby**: text inputs (placeholder "text" in Figma = value placeholder).
Option copy in Figma: Pronouns He/Him, She/Her, They/Them, Ze/Zir, Xe/Xem. Orientation: Heterosexual, Bisexual, Lesbian, Gay, Asexual. Relationship: Single, In a relationship, Engaged, Married, It's complicated. Gender (213 bug) - see Q. Full legacy lists in section 5.
Interactions: keyboard (203-205): Android IME overlays bottom ~680px; footer hidden under keyboard; scroll so the focused field's bottom +24 is above keyboard (legacy `_async_ensure_field_visible`); header stays. Done key closes. Dropdown tap toggles list; outside tap closes. Save enabled only when dirty and no field error (legacy; Figma always draws ruby SAVE). Back/Cancel -> M1 if dirty.
Validation (legacy `DclTextEdit`): username max 15; description max 300; country/employment/profession/real name/hobby max 30; links title max 15, url must match `^https?://[^\s]+\.[^\s]+$`; error text + error border (not designed). Whitespace-edge check exists.
Data/IO: read profile; username list from `GET <lambda>/users/<addr>/names` (NamesRequest; prefetched on open); write via Rust bridge `deploy_profile` (changed fields only: name, hasClaimedName, description, country, pronouns, gender, sexualOrientation, relationshipStatus, employmentStatus, profession, realName, hobbies, links). Option IDs are stable English strings (e.g. "He / Him") not display text.
States: pristine, dirty, field error, names loading (unique tab appears when names arrive), saving (optimistic), save failed (revert + message - undesigned), keyboard, dropdown open, disabled username.

### M1 Leave without saving (720x1600)
Backdrop: blurred screen + dim. Dialog x100..620 (520 wide) y555..950, radius ~24, purple gradient w/ magenta glow top-right and bottom. Warning triangle icon (orange fill/black outline, 48) overlapping top edge, centered. Title "Leave without saving?" Bold 36 white centered (2 lines). Body "You'll lose any unsaved changes." 26/Regular. Buttons h72 radius 16: LEAVE (black40 secondary, left) | STAY (ruby, right), Bold 24 uppercase. Backdrop tap = STAY (inferred). Existing `PrimaryButton` tones cover this.

### M2 Add Link modal
Dialog x50..670 (620 wide) y~490..1108, same gradient/radius as M1. "Add Link" Bold 40 centered. Fields: label "Link title" 30/Medium; input h68 placeholder "e.g. Instagram" (white40), counter "0/15" (22); label "Link"; input placeholder `Link URL ("https://")`, no counter. Focused input: 1px #D9B3FF-ish border + clear "x" (lavender) at right. Buttons: CANCEL (black40) | SAVE (ruby; dim 50% = disabled when title/url invalid), h74 radius 16. Keyboard pushes dialog up (legacy: spacer = keyboard height). Backdrop tap closes (legacy: any touch on scrim). Paste toolbar in 186 is the OS.

### D1 Dropdown list fragments (624 wide)
Label above (30/Medium), trigger h68 (focused: 1px border white-ish, chevron up), list below at y128 radius 12 bg #1F0033, 1px border #5B2A8A, rows 76 pitch, left pad 24. Shown on dark #1C1C1C only because the fragment sits on gray; use screen context from 190/201.

## 4. Tokens / components

Extracted (get_design_context, 270:8413): screen bg #470071 (hero gradient: top-left #7D26B5-ish to #470071, rotated ellipse glow); Ruby #FF2D55; Snow #FCFCFC; Pearl #ECEBED; Silver #A09BA8; Gray #CFCDD4; Black40 rgba(0,0,0,.4); nav bg #161518 (drop shadow 0 -4 2 .25, backdrop blur 2).
Type (Inter): H5 Medium 30 (field labels, values); Button Bold 30 uppercase; title Bold 48; helper/counter Regular 20 #ECEBED; ADD LINK Bold 22.05.
Sizes: content gutter 48; input h68 (single line), radius 12, padding 16; textarea 216; buttons h96 radius 16 (footer), h68 radius 16 (add link); avatar 200 (image 153); chevron/back icons 40/60; section gap 64; label->field gap 20; label->helper gap 10.
Exists in rn/src/ds: `colors` (ruby, snow, pearl, silver, lavender, purple350-550, glass, scrim, white30/70/10), `PrimaryButton` (primary/secondary/ghost; ok for CANCEL/SAVE/LEAVE/STAY), `Tabs` (check it supports underline + uppercase for UNIQUE/NON-UNIQUE), `ItemCard`/`SkeletonCard` (equipped carousel), `Pill` (rarity), `IconButton`, `BottomNav` (profile ring icon exists), `Icon` (profileFill/ring), `GradientView/LinearFill/RadialFill`, `AppText`, `Sheet` (backpack/Sheet.tsx: 2-state drag sheet, needs 3 snaps), `ColorPicker` n/a.
Missing: TextField (single/multiline, counter, clear x, focus border, error, disabled), Dropdown (trigger + upward/downward list, check, scroll), Modal/Dialog (blurred scrim, gradient card, icon), LinkChip, ClaimedBadge icon, copy/pencil/link/warning/chevron/close/back/globe/language/pronoun/gender/heart/briefcase/at/gamepad icons (only 10 SVGs exist in iconSvgs), Toast (copied/saved), keyboard-avoiding scroll helper, field row (icon + label + value).

## 5. Legacy counterpart (Godot) and un-drawn behaviour

Files: `godot/src/ui/pages/profile/` - `profile.gd/.tscn` (841 lines, landscape/desktop card, friend/block/mute, copy), `profile_portrait.gd/.tscn` + `profile_portrait_content.*` (mobile sheet via `DraggableBottomSheet`, avatar via `AvatarPreview`), `profile_about.gd` + `about_data.gd` (modes NONE/DESCRIPTION_ONLY/ABOUT_DATA_ONLY/BOTH, see more/less, 2-col portrait / 3-col landscape), `profile_equipped*.gd` (wearables + emotes carousel, tap emote plays), `profile_links.gd`, `profile_link_button.gd`, `profile_editor.gd` (dirty tracking, save, keyboard avoidance), `profile_new_link_popup.gd`, `profile_constants.gd` (countries 200+, languages, option tables with stable IDs), `components/molecules/username_picker/`, `components/atoms/inputs/dcl_text_edit.gd`. `docs/PROFILE_IMAGE.md` is only the Docker avatar-snapshot renderer - not UI, no relevance beyond "face/body snapshot URLs exist on profile (`snapshots.face256`)". `reports/legacy-godot.md` section 5 confirms.
Legacy behaviour not in Figma:
- Other users' profiles: friend add/cancel/unfriend/accept, block/unblock, mute/unmute, mutual friends, menu (button_menu). No Figma -> needs design (decision).
- Copy: tap name copies "name#tag"; tap address copies full address.
- Empty description hint `PROFILE_USE_THIS_SPACE_TO_DESCRIBE_YOURSELF`; empty fields hidden.
- Full option lists: Genders (Female, Male, Transgender, Neutral, Non-Binary, Fluid, Agender, Pangender, Queer, Two-Spirit, Other); Orientation (Heterosexual, Bisexual, Lesbian, Gay, Asexual, Queer, Polysexual, Pansexual, Other); Pronouns (He / Him, She / Her, They / Them, Ze / Zir, Xe / Xem, Ze / Hir, Per / Per, Other); Relationship (Single, In a relationship, Engaged, Married, It's complicated, In an open relationship, Widowed, Separated, Divorced, Other). Stored value = ID with spaces ("He / Him"); Figma shows "He/Him" and "He/His" (typo in profile view).
- Country is a free-text field in legacy (COUNTRIES/LANGUAGES constants exist and are used by the About display); Language (`get_language`) is displayed but has no editor field.
- Deploy: `ProfileService.async_deploy_profile`, 15s deploy timeout timer, optimistic close, `save_failed` refresh.
- Name rules: claimed name -> tag hidden + seal; "Claim name" external link hidden on iOS/store policy (`change_nick_popup`).
- Haptics/sounds (`UiSounds`).
- Landscape orientation switching on edit.
- Fetch for other users: `async_fetch_profile(addr, lambda_url)`.

## 6. RN status and gap list

rn/src has: no profile screen. `Home.tsx` has a `profile: 'Profile'` tab label that renders the "Not part of this spike yet." placeholder; `BottomNav` has the avatar-ring icon; `backpack/` has `Sheet`, `AvatarStage`, `ItemCard` usage, and bridge commands (`GodotBridge.ts`: send_chat, set_color, randomize_avatar, ...). `flow/` has Naming (guest pick name) that will share the username field. No profile read/deploy bridge commands or events.
Gap list:
1. Bridge: `get_profile`, `deploy_profile(patch)`, `get_names`, profile-changed event, avatar preview for a given profile/address (reuse AvatarStage).
2. DS: TextField, Dropdown, Dialog, LinkChip, Toast, FieldRow, icons (~16 new), claimed badge.
3. Screens: ProfileSheet (3-snap), ProfileFull, ProfileLandscapeCard, EditProfile (+ ScrollView w/ keyboard avoidance), LeaveDialog, AddLinkDialog, UsernamePicker.
4. Logic: dirty tracking, validation (lengths + URL regex), stable option IDs, links max 5, names fetch, optimistic save + rollback.
5. Orientation: landscape card variant with HUD pill menu; edit from landscape.
6. Other-user profile (undesigned).

## 7. Open questions / inconsistencies

1. SAVE in AddLink (187) looks dimmed even with valid title + URL - presumably not updated variant; implement enabled when both valid.
2. Gender dropdown (214) lists orientation options (copy bug); use legacy gender list.
3. Country: dropdown in 180/183/201/205 vs plain text field in 211 and legacy; Figma 205 shows country-like list "Brasil/Argentina/Argelia/Albania" (placeholder names, Spanish). Decide: dropdown w/ search over `ProfileConstants.COUNTRIES` (needs scroll) vs free text.
4. Language is displayed (profile) but not editable in any edit frame; legacy also lacks it - confirm intent.
5. Username disabled look (200-204, 211) vs "Carlos" enabled (180) - when disabled? Plain username (no counter/tag, 180) vs tag+counter (183).
6. Profile "He/His" typo vs "He/Him" option; Figma labels use "He/Him" without spaces while stored IDs have spaces.
7. ADD LINK hidden after one link in 188 (only 1 of 5) - likely a design mistake; legacy keeps button until 5 links.
8. Sheet snaps: 584 vs 582 (178 vs 217) near-duplicates; 718 shows Equipped without About (empty-about?) - define snap points (peek 1352, half ~584, full 0) and whether equipped sits below About in scroll.
9. Avatar background gradient differs (207 light lavender vs others purple) - per-profile? 
10. Landscape: no LINKS section and no copy-name icon (only address copy); portrait has both copy icons.
11. Duplicates (identical frames): 176=193, 178=182=198, 180=206, 183=194, 189=199.
12. Save feedback (success/failed toast), copy toast, field error states, loading skeleton, other-user profile (friend/block/mute) are not designed.
13. Edit footer replaces nav bar in 180 etc. but the nav bar is present in the Figma node (hidden under footer); confirm nav hidden during edit.
14. Keyboard frames (203-205) show footer hidden; confirm CANCEL/SAVE hide behind keyboard vs. float above.
