# UI Component Audit

This document accompanies the Atomic Design reorganization of `godot/src/ui/` (issue [#1876](https://github.com/decentraland/godot-explorer/issues/1876)). It captures duplication candidates, misplaced files, and follow-up cleanup items that were **deliberately left out of the reorganization PR** so each change can be reviewed independently.

The reorganization PR moved files into the new tier structure and rewrote `res://` references. It did not change behavior, merge components, rename classes, or delete unused code.

## New structure

```
godot/src/ui/
├── explorer.gd / explorer.tscn        # app shell (top-level entry point)
├── components/
│   ├── atoms/                         # smallest single-purpose controls
│   │   ├── buttons/                   # 9 single-button components
│   │   ├── controls/                  # sliders, pagination, label variants, etc.
│   │   ├── images/                    # texture / image wrappers
│   │   └── inputs/                    # LineEdit / TextEdit wrappers
│   ├── molecules/                     # 2-3 atoms working as a unit
│   └── organisms/                     # complex composite sections
├── layouts/                           # responsive / safe-area wrappers
└── pages/                             # full screens with state and data
```

The full mapping that was applied is in the git history of this PR — every move was performed with `git mv` so file history is preserved.

## Migration map

Every path is relative to `godot/src/ui/`. Old paths describe where each file lived in `main` before this PR.

### Pages

| File | New path | Old path | Description |
| --- | --- | --- | --- |
| `login.tscn` | `pages/auth/` | `components/auth/` | Wallet / email sign-in screen |
| `lobby.tscn` | `pages/auth/` | `components/auth/` | Pre-world lobby screen |
| `backpack.tscn` | `pages/backpack/` | `components/backpack/` | Backpack root (responsive) |
| `backpack_portrait.tscn` | `pages/backpack/` | `components/backpack/` | Mobile portrait layout |
| `backpack_landscape.tscn` | `pages/backpack/` | `components/backpack/` | Mobile landscape / desktop layout |
| `avatar_preview.tscn` | `pages/backpack/` | `components/backpack/` | 3D avatar preview viewport |
| `emote_name.tscn` | `pages/backpack/` | `components/backpack/` | Emote name label |
| `no_items_message.tscn` | `pages/backpack/` | `components/backpack/` | Empty-state placeholder |
| `chat.tscn` | `pages/chat/` | `components/chat/` | Chat container |
| `chat_panel.tscn` | `pages/chat/` | `components/chat/` | Side chat panel |
| `chat_message.tscn` | `pages/chat/` | `components/chat/` | Single chat message row |
| `mention_item.tscn` | `pages/chat/` | `components/chat/` | @-mention suggestion item |
| `notifications.tscn` | `pages/chat/` | `components/chat/` | In-chat notification banner |
| `discover.tscn` | `pages/discover/` | `components/discover/` | Discover root view |
| `first_time_user_experience.tscn` | `pages/discover/` | `components/discover/` | FTUE overlay |
| `scrollbar_requester.tscn` | `pages/discover/` | `components/discover/` | Scrollbar visibility helper |
| `search_keyword.tscn` | `pages/discover/` | `components/discover/` | Search keyword chip |
| `search_sugestions_container.tscn` | `pages/discover/` | `components/discover/` | Search-suggestion container |
| `discover_carrousel.tscn` | `pages/discover/carrousel/` | `components/discover/carrousel/` | Discover carrousel |
| `discover_carrousel_item_loading.tscn` | `pages/discover/carrousel/` | `components/discover/carrousel/` | Carrousel loading skeleton |
| `place_discover_card.tscn` | `pages/discover/carrousel/` | `components/discover/carrousel/` | Place card |
| `place_discover_skeleton.tscn` | `pages/discover/carrousel/` | `components/discover/carrousel/` | Place card skeleton |
| `categories_bar.tscn` | `pages/discover/categories/` | `components/discover/categories/` | Discover categories bar |
| `category_tag.tscn` | `pages/discover/categories/` | `components/discover/categories/` | Single category tag |
| `event_pills_bar.tscn` | `pages/discover/event_pills_bar/` | `components/discover/event_pills_bar/` | Pill-style event filter bar |
| `friend_discover_card.tscn` | `pages/discover/friends/` | `components/discover/friends/` | Friend card |
| `friend_discover_skeleton.tscn` | `pages/discover/friends/` | `components/discover/friends/` | Friend card skeleton |
| `friend_jump_in.tscn` | `pages/discover/friends/` | `components/discover/friends/` | "Jump to friend" affordance |
| `friends_generator.tscn` | `pages/discover/friends/` | `components/discover/friends/` | Friends list generator |
| `panel_friend_detail_portrait.tscn` | `pages/discover/friends/` | `components/discover/friends/` | Friend detail panel |
| `jump_in.tscn` | `pages/discover/jump_in/` | `components/discover/jump_in/` | Jump-in flow root |
| `download_warning.tscn` | `pages/discover/jump_in/` | `components/discover/jump_in/` | Asset-download warning |
| `panel_jump_in_landscape.tscn` | `pages/discover/jump_in/` | `components/discover/jump_in/` | Landscape jump-in panel |
| `panel_jump_in_portrait.tscn` | `pages/discover/jump_in/` | `components/discover/jump_in/` | Portrait jump-in panel |
| `places_generator.tscn` | `pages/discover/places/` | `components/discover/places/` | Places list generator |
| `search_suggestions.tscn` | `pages/discover/search_bar/` | `components/discover/search_bar/` | Search-suggestion list |
| `event_details.tscn` | `pages/events/` | `components/events/` | Event details view |
| `event_details_portrait.tscn` | `pages/events/` | `components/events/` | Portrait event details |
| `event_details_landscape.tscn` | `pages/events/` | `components/events/` | Landscape event details |
| `event_discover_card.tscn` | `pages/events/` | `components/events/` | Event card |
| `event_discover_skeleton.tscn` | `pages/events/` | `components/events/` | Event card skeleton |
| `events_generator.tscn` | `pages/events/` | `components/events/` | Events list generator |
| `button_reminder.tscn` | `pages/events/` | `components/events/` | Event-reminder button |
| `friends_panel.tscn` | `pages/friends/` | `components/friends/` | Friends side panel |
| `loading_screen.tscn` | `pages/loading_screen/` | `components/loading_screen/` | Full-screen loading view |
| `profile.tscn` | `pages/profile/` | `components/profile/` | Profile root |
| `profile_container.tscn` | `pages/profile/` | `components/profile/` | Profile panel container |
| `profile_portrait.tscn` | `pages/profile/` | `components/profile/` | Portrait profile layout |
| `profile_portrait_content.tscn` | `pages/profile/` | `components/profile/` | Portrait content body |
| `profile_about.tscn` | `pages/profile/` | `components/profile/` | "About" tab |
| `about_data.tscn` | `pages/profile/` | `components/profile/` | About-data block |
| `profile_equipped.tscn` | `pages/profile/` | `components/profile/` | "Equipped" tab |
| `profile_equipped_item.tscn` | `pages/profile/` | `components/profile/` | Single equipped item |
| `profile_links.tscn` | `pages/profile/` | `components/profile/` | "Links" tab |
| `profile_link_button.tscn` | `pages/profile/` | `components/profile/` | Link button |
| `profile_new_link_popup.tscn` | `pages/profile/` | `components/profile/` | Add-link popup |
| `profile_editor.tscn` | `pages/profile/` | `components/profile/` | Profile-edit form |
| `profile_field_text.tscn` | `pages/profile/` | `components/profile/` | Text profile field |
| `profile_field_option.tscn` | `pages/profile/` | `components/profile/` | Option-select profile field |
| `settings.tscn` | `pages/settings/` | `components/settings/` | Settings panel root |
| `section_title.tscn` | `pages/settings/` | `components/settings/` | Settings section header |
| `slider.tscn` | `pages/settings/` | `components/settings/` | Labeled slider row |
| `underlined_button.tscn` | `pages/settings/` | `components/settings/` | Text-link button |

### Layouts

Layouts ship as scripts only (no `.tscn`); each is a reusable container/wrapper Control.

| File | New path | Old path | Description |
| --- | --- | --- | --- |
| `safe_margin_container.gd` | `layouts/` | `components/utils/` | Respects device safe-area insets |
| `responsive_container.gd` | `layouts/` | `components/utils/` | Switches layout on orientation change |
| `orientation_container.gd` | `layouts/` | `components/utils/` | Selects child by orientation |
| `clean_orientation.gd` | `layouts/` | `components/utils/` | Strips unused orientation children |
| `hide_orientation.gd` | `layouts/` | `components/utils/` | Hides node in selected orientation |
| `figma_margins.gd` | `layouts/` | `components/utils/` | Applies Figma-token margins |

### Components / Atoms

| File | New path | Old path | Description |
| --- | --- | --- | --- |
| `custom_button.tscn` | `components/atoms/buttons/custom_button/` | `components/custom_button/` | Base text + icon button |
| `static_button.tscn` | `components/atoms/buttons/static_button/` | `components/static_button/` | Non-interactive visual button |
| `hud_button.tscn` | `components/atoms/buttons/animated_button/` | `components/animated_button/` | HUD button with sprite animation |
| `button_touch_action.tscn` | `components/atoms/buttons/button_touch_action/` | `components/button_touch_action/` | Contextual touch action button |
| `calendar_button.tscn` | `components/atoms/buttons/calendar_button/` | `components/calendar_button/` | Date-picker trigger |
| `fav_button.tscn` | `components/atoms/buttons/fav_button/` | `components/fav_button/` | Favorite toggle |
| `menu_navbar_button.tscn` | `components/atoms/buttons/menu_navbar_button/` | `components/menu_navbar_button/` | Nav menu button |
| `menu_navbar_highlight.tscn` | `components/atoms/buttons/menu_navbar_button/` | `components/menu_navbar_button/` | Active-state highlight overlay |
| `mini_map_button.tscn` | `components/atoms/buttons/mini_map_button/` | `components/mini_map_button/` | Mini-map toggle button |
| `custom_slider.tscn` | `components/atoms/controls/custom_slider/` | `components/custom_slider/` | Themed slider |
| `custom_background_slider.tscn` | `components/atoms/controls/custom_slider/` | `components/custom_slider/` | Slider variant with custom track |
| `loading_spinner.tscn` | `components/atoms/controls/loading_spinner/` | `components/loading_spinner/` | Spinner indicator |
| `marquee_label.tscn` | `components/atoms/controls/marquee_label/` | `components/marquee_label/` | Scrolling-text label |
| `pagination.tscn` | `components/atoms/controls/pagination/` | `components/pagination/` | Page-dot pager |
| `radio_selector.tscn` | `components/atoms/controls/radio_selector/` | `components/radio_selector/` | Radio-group selector |
| `async_image.tscn` | `components/atoms/images/async_image/` | `components/async_image/` | Async image loader |
| `profile_picture.tscn` | `components/atoms/images/profile_picture/` | `components/profile_picture/` | Avatar profile image |
| `right_arrow.tscn` | `components/atoms/images/right_arrow/` | `components/right_arrow/` | Right-arrow icon |
| `dcl_line_edit.tscn` | `components/atoms/inputs/` | `components/` (loose file) | Single-line text input wrapper |
| `dcl_text_edit.tscn` | `components/atoms/inputs/` | `components/` (loose file) | Multi-line text input wrapper |
| `line_edit_custom.tscn` | `components/atoms/inputs/line_edit_custom/` | `components/line_edit_custom/` | Alt single-line input (see audit §1) |

### Components / Molecules

| File | New path | Old path | Description |
| --- | --- | --- | --- |
| `navbar_profile_button.tscn` | `components/molecules/button_profile/` | `components/button_profile/` | Avatar-pic + dropdown trigger |
| `profile_icon_button.tscn` | `components/molecules/profile_icon_button/` | `components/profile_icon_button/` | Icon-only profile button |
| `carrousel_page_item.tscn` | `components/molecules/carrousel_page_item/` | `components/` (loose file) | Generic carrousel card |
| `color_carrousel.tscn` | `components/molecules/color_carrousel/` | `components/color_carrousel/` | Horizontal color picker |
| `color_button.tscn` | `components/molecules/color_carrousel/` | `components/color_carrousel/` | Single color swatch |
| `dropdown_list.tscn` | `components/molecules/dropdown_list/` | `components/dropdown_list/` | Dropdown control |
| `dropdown_item.tscn` | `components/molecules/dropdown_list/` | `components/dropdown_list/` | Single dropdown item |
| `label_avatar_name.tscn` | `components/molecules/label_avatar_name/` | `components/label_avatar_name/` | Avatar pic + nickname combo |
| `pointer_tooltip.tscn` | `components/molecules/pointer_tooltip/` | `components/pointer_tooltip/` | Floating pointer tooltip |
| `tooltip_label.tscn` | `components/molecules/pointer_tooltip/` | `components/pointer_tooltip/` | Tooltip content label |
| `search_bar.tscn` | `components/molecules/search_bar/` | `components/search-bar/` | Search input + clear button (dir renamed) |
| `snap_carousel.tscn` | `components/molecules/snap_carousel/` | `components/snap_carousel/` | FTUE / snap-scroll carousel |
| `snap_carousel_card.tscn` | `components/molecules/snap_carousel/` | `components/snap_carousel/` | Snap-carousel card |
| `wearable_item.tscn` | `components/molecules/wearable_item/` | `components/wearable_item/` | Wearable grid item |
| `wearable_category.tscn` | `components/molecules/wearable_category/` | `components/wearable_category/` | Wearable category tab |
| `wearable_filter_button.tscn` | `components/molecules/wearable_button/` | `components/wearable_button/` | Wearable filter chip |

### Components / Organisms

| File | New path | Old path | Description |
| --- | --- | --- | --- |
| `navbar.tscn` | `components/organisms/navbar/` | `components/navbar/` | Bottom navigation bar |
| `menu.tscn` | `components/organisms/menu/` | `components/menu/` | Main menu controller |
| `menu_profile_button.tscn` | `components/organisms/menu/` | `components/menu/` | Profile entry in menu |
| `account_deletion_popup.tscn` | `components/organisms/menu/` | `components/menu/` | Account-deletion confirmation |
| `modal.tscn` | `components/organisms/modal/` | `components/modal/` | Generic modal scaffold |
| `travel_modal.tscn` | `components/organisms/modal/` | `components/modal/` | Travel/realm-change modal |
| `dialog_stack.tscn` | `components/organisms/dialogs/` | `ui/dialogs/` (top-level) | Stacked-dialog manager |
| `confirm_dialog.tscn` | `components/organisms/dialogs/` | `ui/dialogs/` | Yes/no confirmation dialog |
| `nft_dialog.tscn` | `components/organisms/dialogs/` | `ui/dialogs/` | NFT details dialog |
| `change_nick_popup.tscn` | `components/organisms/change_nick_popup/` | `components/` (loose file) | Nickname-change popup |
| `popup_warning.tscn` | `components/organisms/popup_warning/` | `components/popup_warning/` | Generic warning popup |
| `url_popup.tscn` | `components/organisms/url_popup/` | `components/url_popup/` | External-URL confirmation |
| `jump_in_popup.tscn` | `components/organisms/jump_in_popup/` | `components/jump_in_popup/` | Jump-in confirmation popup |
| `terms_and_conditions.tscn` | `components/organisms/terms_and_conditions/` | `components/terms_and_conditions/` | T&Cs acceptance dialog |
| `update_available.tscn` | `components/organisms/update_available/` | `components/update_available/` | Update-available banner |
| `warning_messages.tscn` | `components/organisms/warning_messages/` | `components/warning_messages/` | Inline warning toast stack |
| `disconnect_handler.tscn` | `components/organisms/disconnect_handler/` | `components/disconnect_handler/` | Disconnect handler / overlay |
| `notifications_panel.tscn` | `components/organisms/notifications/` | `components/notifications/` | Notification side panel |
| `notification_toast.tscn` | `components/organisms/notifications/` | `components/notifications/` | Toast variant — default |
| `alert_toast.tscn` | `components/organisms/notifications/` | `components/notifications/` | Toast variant — alert (audit §3) |
| `low_spec_toast.tscn` | `components/organisms/notifications/` | `components/notifications/` | Toast variant — low-spec warning |
| `notification_item.tscn` | `components/organisms/notifications/` | `components/notifications/` | Notification panel row |
| `notification_content.tscn` | `components/organisms/notifications/` | `components/notifications/` | Notification row content |
| `recording_notification.tscn` | `components/organisms/recording_notification/` | `components/recording_notification/` | "Recording" status banner |
| `chatbar.tscn` | `components/organisms/chatbar/` | `components/chatbar/` | Bottom chat input bar |
| `engagement_bar.tscn` | `components/organisms/engagement_bar/` | `components/engagement_bar/` | Like/share engagement bar |
| `joypad.tscn` | `components/organisms/joypad/` | `components/joypad/` | Mobile virtual joypad |
| `voice_chat_recorder.tscn` | `components/organisms/voice_chat_recorder/` | `components/voice_chat_recorder/` | Push-to-talk recorder UI |
| `draggable_bottom_sheet.tscn` | `components/organisms/draggable_bottom_sheet/` | `components/draggable_bottom_sheet/` | Draggable bottom sheet |
| `item_preview.tscn` | `components/organisms/item_preview/` | `components/item_preview/` | Backpack item-detail preview |
| `emote_wheel.tscn` | `components/organisms/emotes/` | `components/emotes/` | Radial emote picker |
| `emote_wheel_item.tscn` | `components/organisms/emotes/` | `components/emotes/` | Emote-wheel slot |
| `emote_square_item.tscn` | `components/organisms/emotes/` | `components/emotes/` | Emote grid tile |
| `emote_editor.tscn` | `components/organisms/emote_editor/` | `components/emote_editor/` | Emote-slot editor |
| `emote_editor_item.tscn` | `components/organisms/emote_editor/` | `components/emote_editor/` | Single emote-editor slot |
| `color_picker_panel.tscn` | `components/organisms/color_picker/` | `ui/color_picker/` (top-level) | Color-picker panel |
| `color_picker_button.tscn` | `components/organisms/color_picker/` | `ui/color_picker/` | Color-picker swatch trigger |
| `colorable_square.tscn` | `components/organisms/color_picker/` | `ui/color_picker/` | Hue/saturation square |
| `mutual_friends.tscn` | `components/organisms/social/` | `components/social/` | Mutual-friends row |
| `social_item.tscn` | `components/organisms/social/social_item/` | `components/social/social_item/` | Friend list row |
| `profile_settings.tscn` | `components/organisms/profile_settings/` | `components/profile_settings/` | Profile edit form (account-level) |
| `debug_panel.tscn` | `components/organisms/debug_panel/` | `components/debug_panel/` | Dev debug panel |
| `network_inspector_ui.tscn` | `components/organisms/debug_panel/network_inspector/` | `components/debug_panel/network_inspector/` | Network-inspector UI |
| `request_entry.tscn` | `components/organisms/debug_panel/network_inspector/` | `components/debug_panel/network_inspector/` | Network-inspector row |
| `livekit_debug_panel.tscn` | `components/organisms/livekit_debug/` | `components/livekit_debug/` | LiveKit voice debug panel |

## Duplication / unification candidates

Each row below is a candidate for a follow-up unification PR. Estimates assume only the noted files change.

### 1. Single-line text inputs

- **Files**: `components/atoms/inputs/dcl_line_edit/`, `components/atoms/inputs/line_edit_custom/`
- **Proposal**: Diff the two. Both extend `LineEdit`. Keep `dcl_line_edit` as the canonical wrapper unless `line_edit_custom` has features (placeholder behavior, focus styling, validation hooks) that `dcl_line_edit` lacks. If the deltas are additive, port them into `dcl_line_edit` and replace `line_edit_custom` instantiations.
- **Before unifying**: list every scene that instances `line_edit_custom.tscn` (it's referenced by `class_name LineEditCustom`, so also grep that) and verify visual/behavior parity in each.
- **Risk**: Low. Both classes are leaf inputs; visual regression possible if themes differ.

### 2. Button family

- **Files** (under `components/atoms/buttons/`): `animated_button`, `button_touch_action`, `calendar_button`, `custom_button`, `custom_touch_button`, `fav_button`, `menu_navbar_button`, `mini_map_button`, `static_button`
- **Plus** molecule-level buttons: `button_profile`, `profile_icon_button`
- **Proposal**: Map each button's usage. Most are likely thin wrappers around `Button` with different styling. The likely canonical set is 3–4 variants: `Primary`, `Secondary`, `Icon`, `Ghost` (plus a touch-optimized variant for mobile that may already be `custom_touch_button`). Define those as themes/styles on a single `DclButton` rather than separate scenes.
- **Before unifying**: build a usage matrix (which button is used where, with what theme override) before touching any files. The wide adoption makes this a multi-PR effort, not a single sweep.
- **Risk**: Medium. Visual regressions across the whole app are easy to miss.

### 3. Toast / notification variants

- **Files** (under `components/organisms/notifications/`): `alert_toast.tscn`, `low_spec_toast.tscn`, plus the base notification scenes
- **Proposal**: Confirm whether `low_spec_toast` is a hardware-detection variant or obsolete. If it's a styling variant of the standard toast, collapse into one `Toast` with a `variant` enum (alert / info / low_spec). Keep the existing trigger APIs.
- **Risk**: Low. Toasts are leaf, short-lived UI.

### 4. Modal vs Dialog systems

- **Files**:
  - `components/organisms/modal/` — `modal.tscn`, `modal_manager.gd`, `travel_modal.gd`
  - `components/organisms/dialogs/` — `dialog_stack.tscn`, `confirm_dialog.tscn`, `nft_dialog.tscn`
- **Proposal**: Two managers coexist. `modal_manager` is used for general modal content (loaded dynamically); `dialog_stack` handles confirmation and NFT prompts as a stack. Decide whether they're actually specializations (modal = full-screen content, dialog = confirmation prompt) or duplicated infrastructure. If specializations, document the boundary; if duplicates, pick one manager and migrate callers.
- **Before unifying**: trace every `modal_manager.show_*()` call and every `dialog_stack.add_child()` call. The behavior contracts (signal flow, dismiss timing, escape handling) must be compared before merging.
- **Risk**: Medium. Modal/dialog logic is wired into many flows (auth, NFT prompts, settings confirms, travel prompts).

### 5. Profile button surfaces

- **Files**:
  - `components/atoms/images/profile_picture/profile_picture.tscn` — bare avatar image
  - `components/molecules/button_profile/navbar_profile_button.tscn` — animated profile button (in navbar)
  - `components/molecules/profile_icon_button/profile_icon_button.tscn` — icon-style profile button
- **Proposal**: Clarify whether these are size/density variants of one concept (small icon, medium nav button, bare image) or three distinct components. If variants, define a `ProfileButton` with size/style props; if distinct, document where each is used and why.
- **Risk**: Low. All three are leaf widgets.

### 6. List item / row scaffolds

- **Files**:
  - `components/molecules/carrousel_page_item/` — discover carousel card
  - `components/molecules/place_item/` — search result place row
  - `components/molecules/wearable_item/` — equipment grid item
  - `components/organisms/notifications/notification_item.tscn` — notification panel row
  - `components/organisms/emote_editor/emote_editor_item.tscn` — emote wheel item
  - `components/organisms/social/social_item/` — friends list row
- **Proposal**: Extract a shared `BaseListItem` molecule with slots for thumbnail, title, subtitle, trailing action. Each existing item becomes a thin wrapper that fills the slots. This is more about reuse than dedup — the items aren't redundant, but they share padding/selection/touch-target patterns that could live in one place.
- **Risk**: Medium. Touches a lot of feature scenes.

## Misplaced files

These were found in `godot/src/ui/components/` but are **not UI components**. They were intentionally **left in place** by the reorganization PR because moving non-UI code is out of scope. They should be relocated in a follow-up:

- `godot/src/ui/components/floating_island_walls.gd` — `class_name FloatingIslandWalls extends Node3D`. Manages 3D collision walls around scene bounds. Referenced from `lib/`-side game logic (`godot/src/logic/scene_fetcher.gd`). Suggested home: `godot/src/decentraland_components/` or `godot/src/logic/`.
- `godot/src/ui/components/invisible_wall.gd` — `class_name InvisibleWall extends StaticBody3D`. Used by `floating_island_walls.gd`. Same suggested home as above.

## Unused / dead components found during the audit

Tagging these as candidates for removal after a usage sweep — **not removed in this PR**:

- `godot/src/ui/components/atoms/images/right_arrow/right_arrow.tscn` — a plain `TextureRect` of an arrow icon. No `res://...right_arrow.tscn` reference found in the codebase. Likely either inlined in callers or dead.

## Things this PR explicitly did NOT do

- Merge or rename any duplicate components (see table above)
- Consolidate the modal/dialog systems
- Rename any `class_name` declarations (paths changed; class registrations stayed identical)
- Change scene composition, node trees, or signal contracts
- Refactor themes / styling
- Delete `floating_island_walls.gd` or `invisible_wall.gd`
- Delete `right_arrow/`
- Touch GDScript or `.tscn` formatting beyond path substitutions

Every follow-up unification listed above should land as its own PR linked back to issue #1876.
