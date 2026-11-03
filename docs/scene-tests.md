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
| `ecs` | raycast, transform, billboard, camera-mode, engine-info | nothing: state checks only, runs headless |
| `render` | gltf-container, visibility, mesh-renderer, avatar-attach, material, text-shape, ui-background, ui-text | a renderer: they compare screenshots with `tests/snapshots/scenes/` |
| `disabled` | video-player, avatar-shape, ui-button, ui-dropdown, nft-shape | never run; `scenes.json` says why |

A test fails when anything in it throws: a failed assert, a screenshot below 0.9995 similarity or with
no reference image, or a timeout. A scene that never reports its plan and results fails the run too.

## Run them locally

Needs Node (the scripts load nvm when `npm` is not on `PATH`), Godot from `cargo run -- install`
and a built client library (`cargo run -- build`).

```bash
scripts/ci/scene-tests.sh                 # ecs tier, headless: exits 0 when every scene passed
scripts/ci/scene-tests.sh --tier all      # every enabled scene
scripts/ci/scene-tests.sh --windowed      # watch it run
```

Or keep the realm up and use xtask, which builds the client first:

```bash
scripts/serve-scene-tests.sh              # terminal 1: build if changed, serve :7666
cargo run -- run --stest --stest-tier ecs # terminal 2
```

`--stest` fills in `--scene-test`, `--realm` and `--snapshot-folder` from `scenes.json` unless they are
given after `--`. The scene build is reused while nothing under `tests/scene-tests/` changes.

## Add a scene test

1. Copy the closest scene folder to a free parcel, e.g. `54,-70-my-component`, and set `scene.json`'s
   `parcels`/`base`, the folder's `package.json` `name`, and add the folder to the root
   `package.json` `workspaces` and `dcl-workspace.json`.
2. Write the tests in `src/tests/<component>/index.test.ts` with `test()` from
   `testing-library/src/testing`. Prefer state checks (`assertEquals`, `assertComponentValue`); use
   `assertSnapshot('screenshot/$explorer_snapshot_<name>.png', cameraPosition, cameraTarget)` only
   when the change is visual.
3. Add the scene to `scenes.json` with tier `ecs` (no screenshots) or `render`.
4. Run it with `scripts/ci/scene-tests.sh --tier <tier> --windowed`. For a new screenshot, copy the
   capture from `tests/snapshots/scenes/comparison/` into `tests/snapshots/scenes/` after checking it.
