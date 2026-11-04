# Scene tests

SDK7 scenes that check the explorer handles each SDK component the way the SDK expects. Each scene
lives on its own parcel, runs its `test()` blocks when the explorer is started with `--scene-test`,
and reports every result back to the explorer through `~system/Testing`.

## Layout

| Path | What |
|---|---|
| `tests/scene-tests/<x,y-name>/` | one scene per component, e.g. `52,-52-raycast` |
| `tests/scene-tests/testing-library/` | `test()`, asserts, `assertSnapshot` and helpers shared by the scenes |
| `tests/scene-tests/scenes.json` | every scene's parcel and name; the one list xtask and the scripts read |
| `tests/snapshots/scenes/` | reference screenshots for the render tests |
| `scripts/serve-scene-tests.sh` | builds the scenes as a static realm and serves it on `localhost:7666` |
| `scripts/ci/scene-tests.sh` | serves the realm, runs Godot on the scenes and writes the summary |

The suite was copied from the archived
[decentraland/scene-explorer-tests](https://github.com/decentraland/scene-explorer-tests) at 2b82e41
and moved from an unreleased SDK branch build to the released `@dcl/sdk` 7.31.0.

## Three kinds of test

Every scene in `scenes.json` runs on every run; what each test needs decides whether it runs.

| Declared with | Runs | Used for |
|---|---|---|
| `test()` | always | state checks: SDK state the explorer writes back, engine queries |
| `renderTest()` | when Godot draws (`--windowed`, `--render`); a headless run skips it | screenshots compared with `tests/snapshots/scenes/` |
| `skipTest()` | never | a known bug (its comment names the issue) or a scene that cannot run yet |

All three come from `testing-library/src/testing`. A headless run (the PR job) runs the state
checks and skips the pictures; the scene asks the engine whether Godot draws. Skips never fail a
run, but every one is logged (`⏭️ Skipped ...`) and counted in the summary, so a switched-off
test is not forgotten. `--render` runs with a window and fails if any render test was still
skipped: a job meant to draw cannot pass without drawing.

Switched off today: the pinch camera switch (touch-controls) and `flexGrow` next to an undefined
sibling (ui-background), both known bugs; and every test in video-player (video events not working
well), avatar-shape (never run on Godot, no current reference), ui-button and ui-dropdown (never
run on Godot, most tests skipped upstream) and nft-shape (needs remote NFT content).

The realm the tests run on is offline: its `about` points the content server at the local one,
which answers the start-parcel lookup itself, so a run never asks Decentraland's servers which
scene stands at a parcel (it used to, and timed out whenever they were slow).

Scene tests run as a phone: `--scene-test` turns on mobile mode, so the touch controls and the
mobile movement rules are live, and the graphics autotune stays off so screenshots don't vary.
`scripts/ci/scene-tests.sh` also runs Godot with `--fixed-fps 60`, so every frame advances the same
time and an input sequence replays to the same positions on any machine.

### Simulated input

`testing-library/src/utils/input.ts` drives the player with the actions the on-screen controls
press (`ia_forward`, `ia_backward`, `ia_left`, `ia_right`, `ia_jump`, `ia_sprint`, `ia_walk`),
through the Godot-only `~system/Testing.simulateInput` that exists only under `--scene-test`:

- `hold(context, 'ia_forward', 60, 0.8)` pushes the joystick to 0.8 for exactly 60 frames (the
  engine counts them and releases the action). Under 0.5 a movement action does nothing, like the
  real stick's deadzone; from 0.5 to 1 the speed goes from walk to jog.
- `tap(context, 'ia_jump')` is one press of one frame; `press` / `release` hold an action (such as
  `ia_sprint`) around other holds.

Measure at rest (start after the player has settled, read after it has stopped) and the numbers
come out the same every run; `52,-70-locomotion` is the example.

Touch gestures go through the real mobile input code as touch events, via
`~system/Testing.simulateGesture`; the engine places them on the real controls, so tests hold no
pixel coordinates (`52,-72-touch-controls` is the example):

- `joystick(context, { x: 0, y: -1 }, 1, 60)` touches the joystick's area, pushes it forward fully
  and holds it 60 frames.
- `look(context, { x: 300, y: 0 }, 20)` drags one finger across the camera area.
- `pinch(context, 200, 20)` spreads two fingers apart in the camera area (negative closes them).

### App commands and engine queries

`testing-library/src/utils/engine.ts`, through the Godot-only `~system/Testing.testCommand`:

- `query(cmd, args)` runs a read-only debug-hub query (`state`, `scenes`, `scene`, `entity`,
  `ui_scene`, `ui_entity`, `avatars`, `avatar`, `app_ui`, with the hub's `filters`) and returns
  its JSON, for engine state no SDK component exposes. Commands that change state are refused.
- `setSetting(key, value)` changes a setting as the settings screen does (not saved), e.g.
  `graphic_profile`.
- `shell(control)` hands a shell command (`{ type: 'set_color', target: 'eyes', color: '#ff0000' }`)
  to the core as the React Native app does. In scene-test mode the engine is not attached to the
  core yet, so these reach the core but not the engine.

What the engine made of an entity, without a screenshot:

| Helper | Returns |
|---|---|
| `renderedNode(entity)` | whether the node exists and is visible, its position in scene coordinates (wherever an `AvatarAttach` or a parent put it), the bounds of everything it draws, glTF meshes included |
| `renderedMesh(entity)` | the `MeshRenderer` mesh class and visibility |
| `renderedModel(entity)` | the `GltfContainer` source, loading state and visibility |
| `renderedText(entity)` | the `TextShape` label: text, size, colours, outline, alignment, font |
| `renderedUi(entity)` | a UI entity's rect in pixels, its background (colour, texture, nine-slice margins, stretch mode, UVs) and text |
| `renderedDropdown(entity)` | a `UiDropdown`'s options, selected index (-1 for none) and the button's text |

`settled(context, read, done)` re-reads one of them each tick until it holds, since the engine
applies a component a few ticks after the scene sets it.

A run also fails on any Rust panic before the summary line, and screenshots keep the scene's UI
(only the explorer's own HUD is hidden). `52,-74-app-commands` is the example.

A test fails when anything in it throws: a failed assert, a screenshot below 0.9995 similarity or with
no reference image, or a timeout. A scene that never reports its plan and results fails the run too.

## Run them locally

Needs Node (the scripts load nvm when `npm` is not on `PATH`), Godot from `cargo run -- install`
and a built client library (`cargo run -- build`).

```bash
scripts/ci/scene-tests.sh                 # headless: exits 0 when every scene passed
scripts/ci/scene-tests.sh --windowed      # watch it run; render tests run too
scripts/ci/scene-tests.sh --render        # with a window, failing if a render test is skipped
scripts/ci/scene-tests.sh --scene locomotion,touch-controls   # only these (names in scenes.json)
scripts/ci/scene-tests.sh --scene locomotion --watch          # rerun on every save or library build
```

To reproduce a bug and iterate on its fix: write the failing case, then keep
`scripts/ci/scene-tests.sh --scene <name> --watch` running. Saving a test scene rebuilds only that
scene (about 3 s) and reruns it; `cargo run -- build` after an engine change reruns it too. One
scene's run takes about 5 s, Godot's start included. Add `--windowed` to watch it, or
`-- --rust-log='dclgodot::player=debug,warn'` for one module's logs.

Or keep the realm up and use xtask, which builds the client first:

```bash
scripts/serve-scene-tests.sh              # terminal 1: build if changed, serve :7666
cargo run -- run --stest                  # terminal 2
```

`--stest` fills in `--scene-test`, `--realm` and `--snapshot-folder` from `scenes.json` unless they are
given after `--`. The scene build is reused while nothing under `tests/scene-tests/` changes.

## In CI

Every push and PR runs the scene tests headless as the "🛠️ Build & scene tests / 🧪 Scene tests /
Headless" check, with the per-scene summary on the run page and the Godot log as the
`scene-tests-log` artifact. A PR that changes nothing on the Godot side (the `changes` job matches
its files against `lib/`, `godot/`, `src/`, the suite, its scripts and the workflows and actions
they use) skips both the tests and the Linux library.

`android_builds.yml` builds each library once and every other job downloads it:

| Job | Runner | Does |
|---|---|---|
| `changes` | `ubuntu-latest` | on PRs, whether any changed file is on the Godot side |
| `builder-image` | `ubuntu-latest` | pushes the ghcr builder image (`docker/android-builder`) when its Dockerfile changed |
| `linux-lib` | `ubuntu-latest` | needs `changes`; builds the Linux `libdclgodot.so`, uploads `libdclgodot-linux-release` |
| `android-lib` | `ubuntu-latest`, builder image | builds the Android `libdclgodot.so` (unstripped), uploads `libdclgodot-android-release` |
| `scene-tests` | `ubuntu-latest` | needs `linux-lib`; runs `scripts/ci/scene-tests.sh`, headless (`scene_tests.yml`) |
| `build` (Build Android) | `ubuntu-latest`, builder image | needs both libraries; imports, exports, signs and uploads the APK/AAB |

On PRs `runner.yml` passes `build_android: false`, so only `linux-lib` and `scene-tests` run; a PR
builds Android through the `build` label (`mobile_distribute.yml`).

The Linux library is the Godot editor's GDExtension for the import and export as well as the client
the scene tests run. The builder image and `ubuntu-latest` are both Ubuntu 24.04, so it loads in
either. `mobile_distribute.yml` calls `android_builds.yml` without `run_scene_tests`, so its
on-demand rebuilds skip the tests.

## Two modes: automated tests and QA

Every scene serves both, decided by the build:

| Build | Where | What the scene does |
|---|---|---|
| development (`npm run build`, the serve script) | the local realm CI and `--scene-test` use | runs its `test()` blocks and reports to the explorer |
| production (`--production`, the World deploy) | the QA World | `test()` does nothing; a panel lists the scene's manual QA steps |

The steps come from `showQaSteps(title, steps)` (`testing-library/src/qa/panel.tsx`) in the scene's
`main()`. An empty list says the scene has automated checks only. In QA mode the panel takes the
scene's UI renderer; a repro the steps refer to (buttons, the component under test) goes in its
third argument, drawn under the steps.

To see a scene as QA does, run `scripts/qa-scene.sh <name>`: it serves production builds
(`serve-scene-tests.sh --production`, port 7667) and opens the client on the scene; Ctrl-C closes
both. `--screenshot <file.png>` saves a screenshot once the scene loaded and closes them, through
the debug hub (started when none runs); it fails within a minute if the scene does not load.

## QA World

`.github/workflows/scene_tests_world.yml` deploys the scenes with `scripts/ci/deploy-scene-tests.mjs`:

- **Push to `main`** (scene tests changed) or manual run: every scene, at its own parcels.
- **PR with the `qa-scene` label**: only the scenes the PR changed (all of them when it changed
  `testing-library`), moved together into the PR's 30x30 zone on the World's west half, redeployed
  on every push. The run summary lists each scene's mobile and desktop links, and the PR gets a
  "View deployment" button (`qa-world`): the scene when the PR deployed one, else the run summary.
- **PR closed**: the zone is undeployed and the PR's deployment marked inactive.

The World is set by the repository variable `SCENE_TESTS_WORLD` (`mobile1.dcl.eth`: its own scene
stands on 0,0, which neither the test parcels nor the zones touch). Deploys sign with the secret
`SCENE_TESTS_DEPLOYER_KEY`, the key of a wallet on the World's deployment list (not the owner's).
The key lives only in that secret: deploys run in CI, never from a shell. Without the variable or
the secret the jobs fail; fork PRs never deploy.

To see which scenes a deploy would put where, with their links (no key needed):

```bash
(cd tests/scene-tests && npm ci)
SCENE_TESTS_WORLD=mobile1.dcl.eth node scripts/ci/deploy-scene-tests.mjs deploy --dry-run
SCENE_TESTS_WORLD=mobile1.dcl.eth node scripts/ci/deploy-scene-tests.mjs deploy --zone 3207 --dry-run
```

## Add a scene test

1. Copy the closest scene folder to a free parcel, e.g. `54,-70-my-component`, and set `scene.json`'s
   `parcels`/`base`, the folder's `package.json` `name`, and add the folder to the root
   `package.json` `workspaces` and `dcl-workspace.json`.
2. Write the tests in `src/tests/<component>/index.test.ts` with `test()` from
   `testing-library/src/testing`. Check, in this order, the first that can see what is tested:
   1. SDK state the explorer writes back: `GltfContainerLoadingState`, `RaycastResult`,
      `TweenState`, `PointerEventsResult`, the player's and camera's `Transform`.
   2. An engine query through the helpers above (`renderedText`, `renderedUi`, ...).
   3. A screenshot, only when the look itself is tested: materials, lighting, shaders, glyphs,
      texture sampling. `assertSnapshot('screenshot/$explorer_snapshot_<name>.png', cameraPosition,
      cameraTarget)`, in a `renderTest`, next to the scene's state checks.

   A glTF that fails to load is checked by its loading state, not by a picture. A known bug gets a
   `skipTest` that expects the correct result, with a comment naming the issue.
3. In the scene's `main()`, call `showQaSteps('<name>', [...])` with the steps QA should follow by
   hand in the World, or `[]` when the automated tests cover it.
4. Add the scene to `scenes.json` (`{ "parcel": [x, y], "name": "<name>" }`).
5. Run it with `scripts/ci/scene-tests.sh --scene <name>`, and `--windowed` for its render tests. For a new screenshot, copy the
   capture from `tests/snapshots/scenes/comparison/` into `tests/snapshots/scenes/` after checking it.
