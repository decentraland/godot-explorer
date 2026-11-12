# Maestro flows for the React Native UI

UI flows for the React Native layer. They run on the mock build: `src/core/mock` stands in for the core and
`GodotPlaceholder` for the world and the avatar preview, so the data is fixed and the screens are reproducible.

## Build and install

```bash
cd rn && npm ci
cargo run -- run --target android --no-godot        # bundled build, installs on the connected device
```

The mock APK has `versionCode` 1 and is not signed with the CI key, so it does not install over a CI build. Set
`DCL_BUILD_NUMBER` above the installed version, or uninstall the installed app first:

```bash
DCL_BUILD_NUMBER=9999 cargo run -- run --target android --no-godot
```

The Android toolchain needs JDK 17 (`JAVA_HOME`).

## Run

```bash
maestro --device <id> test rn/.maestro/flows/<file>.yaml
```

Maestro runs a folder in its own order, not by file name. To run all flows in order:

```bash
for f in rn/.maestro/flows/*.yaml; do maestro --device <id> test "$f"; done
```

Nothing persists across launches in this build, so every flow starts from "Before you start";
`subflows/ensure-home.yaml` repairs the starting point when a flow needs the Discover home. The six flows that cross the
Discover home (`04`, `05`, `06`, `07`, `08`, `09`) take 3 to 10 minutes each on a Galaxy A54 and the whole set about
45 minutes; the other nine take 20 seconds to 4 minutes each.

## Flows

| Flow | Covers |
| --- | --- |
| `01-entry` | Terms, guest sign-in, avatar screen, tutorial `SKIP`, mock world |
| `02-sign-in` | Sign-in methods, wallet cancel, `CREATE ACCOUNT`, email code (wrong and right), "Welcome back" |
| `03-tutorial` | Guided tutorial: first step and `SKIP TUTORIAL` |
| `04-discover` | Home rails, Friends, category chips, Events, bottom bar |
| `05-search` | Suggestions, results, empty state, recent searches |
| `06-categories` | The six category screens |
| `07-place-sheet` | Place sheet, Favorite / Like / Dislike, Share, `JUMP IN` |
| `08-event-sheet` | Event sheet, expand, share |
| `09-friend-sheet` | Friend sheet |
| `10-backpack` | Tabs, slots, selection pairs, Emotes, Filter |
| `11-profile` | Profile data, edit form, unsaved-changes dialog |
| `12-world-menu` | Chat, drawer panels (Discover, Social, Notifications, Settings), leaving the world |
| `13-settings` | The eight sections and their labels |
| `14-settings-toggles` | Gameplay and Graphics toggles and the options that depend on them |
| `15-account` | Account page, delete and sign-out dialogs, sign out |

## Data

- **Mock (fixed, in the repo):** session, avatar and wearables, profile, settings, notifications, chat, friends,
  favorites and votes (in memory). Flows assert these by exact name.
- **Live public APIs:** Featured, Popular, Events and the place lists. They change daily, so flows check structure
  (a rail has content under its title), never a place or event name.

If a file under `src/core/mock` or `src/menu/mockData.ts` changes, the flows that assert its values change with it.

## Writing a flow

- Check live data by structure (`text: ".+"` with `below` / `above`); check mock data by name.
- Text matching ignores case; use labels that only the screen under test has.
- Start with `subflows/ensure-home.yaml` when the flow needs the Discover home.
- For anything the app keeps between steps, tap only when not selected, then assert it is selected.
- Prefer a fixed `swipe` over `scrollUntilVisible` on the Discover home (see Known limits).

## Known limits

- Profile and Backpack opened from the world drawer crash this build: `showAvatarPreview` in
  `src/core/mock/control.ts` rejects the `InWorld` state and nothing catches the throw in `src/backpack/AvatarStage.tsx`.
  From the bottom tabs both pages work, so no flow here opens them from the drawer.
- `Report Bug` (Settings > Help & Support) and `ADD EMAIL` (Settings > Account) do nothing: their API calls in
  `src/settings/coreApi.ts` are empty. No flow covers them.
- `CREATE ACCOUNT` opens a page titled "Sign in" with the same methods as `SIGN IN`.
- The friend sheet has no close button: its `Close` element is the dimmed scrim over the whole screen, so the flow taps
  the area above the sheet by point.
- The tutorial steps ask for gestures such as a pinch, which Maestro cannot send, so only the first step is checked.
- Maestro drives the Discover home slowly: 6 to 26 s per swipe or tap on a Galaxy A54, against about 1 s on a category
  screen. `scrollUntilVisible` ran to its timeout there with the target on screen, so the flows use fixed swipes.
- After returning from a category the home keeps a different scroll position, so swipes by coordinates are unreliable;
  use `swipe` with `from:` an element.
- `EXPLORE MORE` in the world drawer opens a different page from the Discover home; flows that need the home leave the
  world with Back and `LEAVE`.
