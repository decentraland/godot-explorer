# Scene tests

SDK7 scenes that check the explorer handles each SDK component the way the SDK expects. Each scene
lives on its own parcel, runs its `test()` blocks when the explorer is started with `--scene-test`,
and reports every result back to the explorer through `~system/Testing`.

## Layout

| Path | What |
|---|---|
| `tests/scene-tests/<x,y-name>/` | one scene per component, e.g. `52,-52-raycast` |
| `tests/scene-tests/testing-library/` | `test()`, asserts, `assertSnapshot` and helpers shared by the scenes |
| `tests/scene-tests/scenes.json` | every scene's parcel and tier; the one list xtask and the scripts read |
| `tests/snapshots/scenes/` | reference screenshots for the `render` tier |
| `scripts/serve-scene-tests.sh` | builds the scenes as a static realm and serves it on `localhost:7666` |
| `scripts/ci/scene-tests.sh` | serves the realm, runs Godot on a tier and writes the summary |

The suite was copied from the archived
[decentraland/scene-explorer-tests](https://github.com/decentraland/scene-explorer-tests) at 2b82e41
and moved from an unreleased SDK branch build to the released `@dcl/sdk` 7.31.0.

## Tiers

| Tier | Scenes | Needs |
|---|---|---|
| `ecs` | raycast, transform, billboard, camera-mode, engine-info, gltf-container, visibility, mesh-renderer, avatar-attach, locomotion, touch-controls, app-commands, text-shape, ui-background, ui-text | nothing: state checks only, runs headless |
| `render` | material | a renderer: they compare screenshots with `tests/snapshots/scenes/` |
| `disabled` | video-player, avatar-shape, ui-button, ui-dropdown, nft-shape | never run; `scenes.json` says why |

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
scripts/ci/scene-tests.sh                 # ecs tier, headless: exits 0 when every scene passed
scripts/ci/scene-tests.sh --tier all      # every enabled scene
scripts/ci/scene-tests.sh --windowed      # watch it run
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
cargo run -- run --stest --stest-tier ecs # terminal 2
```

`--stest` fills in `--scene-test`, `--realm` and `--snapshot-folder` from `scenes.json` unless they are
given after `--`. The scene build is reused while nothing under `tests/scene-tests/` changes.

## In CI

Every push and PR runs the `ecs` tier as the "🧪 Scene tests / Scene tests (ECS)" check, with the
per-scene summary on the run page and the Godot log as the `scene-tests-log` artifact.

`android_builds.yml` builds each library once and every other job downloads it:

| Job | Runner | Does |
|---|---|---|
| `linux-lib` | `ubuntu-latest` | builds the Linux `libdclgodot.so`, uploads `libdclgodot-linux-release` |
| `android-lib` | WarpBuild, builder image | builds the Android `libdclgodot.so` (unstripped), uploads `libdclgodot-android-release` |
| `scene-tests` | `ubuntu-latest` | needs `linux-lib`; runs `scripts/ci/scene-tests.sh --tier ecs` (`scene_tests.yml`) |
| `build` (Build Android) | WarpBuild, builder image | needs both libraries; imports, exports, signs and uploads the APK/AAB |

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
scene's UI renderer, so a scene that sets one up for QA builds its own panel instead.

## QA World

`.github/workflows/scene_tests_world.yml` deploys the scenes with `scripts/ci/deploy-scene-tests.mjs`:

- **Push to `main`** (scene tests changed) or manual run: every scene, at its own parcels.
- **PR with the `qa-scene` label**: only the scenes the PR changed (all of them when it changed
  `testing-library`), moved together into the PR's 20x20 zone on the World's west half, redeployed
  on every push. The run summary lists each scene's mobile and desktop links.
- **PR closed**: the zone is undeployed.

The World is set by the repository variable `SCENE_TESTS_WORLD` (`mobile1.dcl.eth`: its own scene
stands on 0,0, which neither the test parcels nor the zones touch). Deploys sign with the secret
`SCENE_TESTS_DEPLOYER_KEY`, the key of a wallet on the World's deployment list (not the owner's).
The key lives only in that secret: deploys run in CI, never from a shell. Without the variable or
the secret the jobs only leave a notice; fork PRs never deploy.

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
      cameraTarget)`, in a `render` scene.

   A glTF that fails to load is checked by its loading state, not by a picture. A known bug gets a
   `skipTest` that expects the correct result, with a comment naming the issue.
3. In the scene's `main()`, call `showQaSteps('<name>', [...])` with the steps QA should follow by
   hand in the World, or `[]` when the automated tests cover it.
4. Add the scene to `scenes.json` with tier `ecs` (no screenshots) or `render`.
5. Run it with `scripts/ci/scene-tests.sh --tier <tier> --windowed`. For a new screenshot, copy the
   capture from `tests/snapshots/scenes/comparison/` into `tests/snapshots/scenes/` after checking it.
