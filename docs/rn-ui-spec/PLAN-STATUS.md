# RN UI rebuild: status

Plan: `~/.claude/plans/great-now-lets-plan-floofy-thacker.md`. Verification: Android phone only (Moto edge 60 pro).

## Done
- Design system (`rn/src/ds`), route stack (`rn/src/app`), i18n table, gallery + `dclrn://preview/<id>` deep link, `tools/ui-spec/rn_capture.sh` (retries when the device has not rotated).
- Screens for all areas: loading + HUD, Discover, Backpack, Profile, Settings (revamp shell), in-HUD menu, entry flow. All 286 Figma frames have a preview (`__tests__/gallery-coverage`).
- Request/response bridge (`rn/src/bridge`, `godot/src/rn_bridge`); security review fixed a `fetch.signed` allowlist bypass and a header override.
- Capture pass 2: 293 previews, none in the wrong orientation; compare sheets in `docs/rn-ui-spec/out/`.

## Verified on the real engine (Godot Android build, guest account)
- Entry flow to Discover, Jump in, loading cover with real progress, world HUD, coach marks, live chat (sender names resolve), unread badge.
- Drawer: Discover with the real scene header and live places/events, Notifications (empty), Social (guest state), Settings, Profile, Backpack.
- Settings write through to the engine and persist across an app restart. Profile edit (`profile.deploy`) updates the engine profile. Backpack equip updates the avatar and the grid. Favorite heart in the place sheet.
- Device bugs found and fixed: chat profile keyed by address, guest state for Social, Profile nav at the top, avatar overlay bleeding through Edit Profile, avatar framing on Profile, Backpack landscape avatar over the header, emote slots overflow, missing active Settings icon, crash on reload from the gallery registry.

## Decisions applied (review these)
- Settings authority: Figma page "HI I revamp Setings" (1397:6493); `settings-portrait` only for states the revamp does not redraw.
- Rail title "Friends N"; FTUE "Jump in" enters the chosen place; keyboard-state previews 204/205 skipped.
- HUD menu button kept top-left (coach-mark frames show top-right); loading bar follows the percent label.
- Voice-chat volume, mic amplification and Hide Scene Interface have no UI in the revamp; their keys are kept.
- Backpack keeps the inline colour swatch row (new slot-tile frames draw none); landscape emote slots are a 2x5 grid (no Figma frame).
- Entry flow: splash and "Get started!" follow the Figma frames (`settings-portrait` page); "Play as guest" and Terms have no frame and were kept.
- Share and calendar frames are OS sheets: aliased to the place/event sheet behind them. Report Content is an external form.

## Open questions
1. HUD menu button: top-left or top-right?
2. Backpack: tiles or pills generation? Empty-state "Shop" link per store policy?
3. Keep voice/mic/hide-interface out of Settings?
4. Dev Tools section is shown in debug builds only; fine?

## Not done / known gaps
- Untested on device: friends actions with a signed-in account, notifications with data, `account.delete`/`sign_out`, `screenshot.capture`, `places.vote`, event reminders (signed POST/DELETE endpoints follow recollection of the APIs and are unverified).
- `loading_progress.mbps` is always 0; menu friends have no online status or location; "My places" rail not wired.
- Residual security items: redirects on allowed hosts, no path/method limit in `fetch.signed`, unbounded `friends.list` limit.
- iOS: nothing built or captured.
- Nothing is committed.
