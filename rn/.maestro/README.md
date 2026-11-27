# Maestro flows for the React Native UI

UI flows for the React Native layer. They run on the mock build: `src/core/mock` stands in for the core and
`GodotPlaceholder` for the world and the avatar preview, so the data is fixed and the screens are reproducible.

## Run

With one Android device or emulator connected (`adb devices`), from the repository root:

```bash
cargo run -- maestro                   # build the mock app, install it, run every flow in file order
cargo run -- maestro 04 11             # only the flows whose file name starts with 04 and 11
cargo run -- maestro 04 --skip-build   # reuse the mock build already on the device
cargo run -- maestro --device <id>     # when several devices are connected
```

The command exits non-zero when a flow fails and prints one `PASS` / `FAIL` line per flow. Each flow leaves a JUnit
report (`exports/maestro/<flow>.xml`) and a folder with Maestro's logs and failure screenshot
(`exports/maestro/<flow>/`).

It needs [Maestro](https://docs.maestro.dev/getting-started/installing-maestro), `adb`, Node 22 and JDK 17
(`JAVA_HOME`).

- **Mock build only.** The flows clear the app's data, sign out, open the delete-account dialog and toggle
  Favorite / Like. `appId` is the id of the shipping app, so never point them at a build with the engine and a real
  account. The command installs the mock build itself; it has `versionCode` 1 and the local debug key, so when another
  build is installed the command removes it first.
- **English device.** The mock leaves the app language empty, which follows the device language, and every flow
  asserts English text. Set the device to English.
- **Network.** Featured, Popular, Events, the place lists and search come from the public services (see Data). A flow
  that fails on an empty rail may be a service problem, not a UI regression.

`smoke.yaml`, next to this file, is a different test: it runs on the shipping build with the engine, accepts the three
app languages and only checks that a new guest reaches the first world. `cargo run -- maestro` does not run it.

Every flow starts from a clean app (`01-entry` clears the app's data and `subflows/ensure-home.yaml` runs it first), so
flows do not depend on each other or on their order.

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
| `10-backpack` | Item selection in every slot, colors, Collectibles filter, state kept on leaving, Emotes (assign, move) |
| `11-profile` | Profile data, edit form, unsaved-changes dialog |
| `12-world-menu` | Chat, drawer panels (Discover, Social, Notifications, Settings), leaving the world |
| `13-settings` | The eight sections and their labels |
| `14-settings-toggles` | Gameplay and Graphics toggles and the options that depend on them |
| `15-account` | Account page, delete and sign-out dialogs, sign out |

## Data

- **Mock (fixed, in the repo):** session, avatar and wearables, profile, settings, notifications, chat, friends,
  the current place, favorites and votes (in memory). Flows assert these by exact name.
  The Backpack catalog (`src/core/mock/avatarApi.ts`) has several items per slot so selection can be tested. Mask has
  none on purpose, to assert the empty state; `neon_jacket`, `crystal_tiara` and `laser_visor` are collectible, so the
  Collectibles filter has results on wearables. No emote is collectible: base emotes never are, so on emotes that filter
  shows the empty state.
- **Live public services:** Featured, Popular, Events, the place lists and search results. They change daily, so flows
  check structure (a rail has content under its title), never a place or event name. The flows still need the rails
  to have content and `05-search` needs a result that contains "plaza".
- **Emote thumbnails** are also requested from the Catalyst, with a fallback to drawn silhouettes when it fails. The
  emote list and the wearables never come from the Catalyst in this build, and no flow asserts on an image.

If a file under `src/core/mock` or `src/menu/mockData.ts` changes, the flows that assert its values change with it.

## Writing a flow

- Name the file `NN-name.yaml`; the number is the run order and the prefix `cargo run -- maestro NN` selects.
- Start with `subflows/ensure-home.yaml` when the flow needs the Discover home, or with `clearState` and `launchApp`
  otherwise, so the flow does not depend on what ran before it.
- Check live data by structure (`text: ".+"` with `below` / `above`); check mock data by name.
- Text matching ignores case; use labels that only the screen under test has.
- For anything the app keeps between steps, tap only when not selected, then assert it is selected.
  `subflows/select.yaml` does exactly that: `runFlow` it with `env: ITEM: "<label>"`. The avatar the entry's Randomize
  leaves is random, so a flow cannot assume what is worn when it starts.
- Elements that are off screen are not in the hierarchy: bring them in with a `swipe` `from:` a visible element (the
  Extras slots after Glasses, the emote slots 7 to 10) before asserting or tapping them.
- When a label repeats on screen, narrow the match with `above` / `below` (an emote name appears in the chip over the
  slots and in its grid tile).
- Prefer a fixed `swipe` over `scrollUntilVisible` on the Discover home (see Known limits).

## Known limits

- Android only: the flows use the system Back button and were written against Android.
- Profile and Backpack opened from the world drawer crash this build
  ([#3265](https://github.com/decentraland/godot-explorer/issues/3265)). From the bottom tabs both pages work, so no
  flow here opens them from the drawer.
- The Backpack landscape layout is not covered. Menus are portrait-locked (`src/platform/orientation.ts`), so rotating
  the device does not change the layout, and the only route to landscape is the world drawer, which crashes (above).
- `Report Bug` (Settings > Help & Support) and `ADD EMAIL` (Settings > Account) do nothing
  ([#3266](https://github.com/decentraland/godot-explorer/issues/3266)). No flow covers them.
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
